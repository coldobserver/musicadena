use std::path::PathBuf;
use std::process::Stdio;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct SearchVideo {
    pub video_id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<i64>,
    pub thumbnail: Option<String>,
}

pub struct YtDlp {
    pub binary: String,
}

impl YtDlp {
    pub fn new() -> Self {
        YtDlp {
            binary: find_binary().unwrap_or_else(|| {
                if cfg!(windows) {
                    "yt-dlp.exe".to_string()
                } else {
                    "yt-dlp".to_string()
                }
            }),
        }
    }

    async fn run(&self, args: &[&str]) -> anyhow::Result<Vec<String>> {
        let mut cmd = Command::new(&self.binary);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let mut lines = Vec::new();
        let mut reader = BufReader::new(stdout).lines();
        while let Some(line) = reader.next_line().await? {
            if !line.trim().is_empty() {
                lines.push(line.trim().to_string());
            }
        }
        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("yt-dlp exited with {status}");
        }
        Ok(lines)
    }

    pub async fn search_video_id(&self, query: &str) -> anyhow::Result<Option<String>> {
        let search = format!("ytsearch1:{query}");
        let args = [
            search.as_str(),
            "--get-id",
            "--no-playlist",
            "--skip-download",
            "--quiet",
        ];
        let lines = self.run(&args).await?;
        Ok(lines.first().cloned())
    }

    pub async fn search_results(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchVideo>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.max(1).min(20);
        let search = format!("ytsearch{limit}:{query}");
        let args = [
            search.as_str(),
            "-J",
            "--flat-playlist",
            "--skip-download",
            "--no-playlist",
            "--quiet",
        ];
        let lines = self.run(&args).await?;
        let Some(json) = lines.first() else {
            return Ok(Vec::new());
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
            return Ok(Vec::new());
        };
        let entries = root
            .get("entries")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for e in entries {
            let Some(video_id) = e.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            if video_id.is_empty() {
                continue;
            }
            out.push(SearchVideo {
                video_id: video_id.to_string(),
                title: e.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                uploader: e
                    .get("uploader")
                    .or_else(|| e.get("channel"))
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string()),
                duration_secs: e
                    .get("duration")
                    .and_then(|d| d.as_f64())
                    .map(|d| d.round() as i64),
                thumbnail: e
                    .get("thumbnail")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        e.get("thumbnails")
                            .and_then(|ts| ts.as_array())
                            .and_then(|ts| ts.last())
                            .and_then(|t| t.get("url"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string())
                    }),
            });
        }
        Ok(out)
    }

    pub async fn resolve_url(&self, video_id: &str) -> anyhow::Result<String> {
        let url = format!("https://www.youtube.com/watch?v={video_id}");
        let args = [
            "-f",
            "bestaudio/best",
            "--get-url",
            "--no-playlist",
            "--quiet",
            &url,
        ];
        let lines = self.run(&args).await?;
        lines
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("yt-dlp returned no url"))
    }
}

impl Default for YtDlp {
    fn default() -> Self {
        Self::new()
    }
}

fn find_binary() -> Option<String> {
    let candidates = ["yt-dlp.exe", "yt-dlp", "youtube-dl.exe", "youtube-dl"];
    for name in candidates {
        let mut cmd = std::process::Command::new("where");
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        if let Ok(output) = cmd
            .arg(name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if output.success() {
                return Some(name.to_string());
            }
        }
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(pathvar) = std::env::var("PATH").ok() {
        for dir in pathvar.split(';') {
            for name in ["yt-dlp.exe", "yt-dlp", "youtube-dl.exe"] {
                let p = PathBuf::from(dir.trim()).join(name);
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }
    let appdata = std::env::var("LOCALAPPDATA").ok();
    if let Some(ad) = appdata {
        paths.push(PathBuf::from(&ad).join("yt-dlp\\yt-dlp.exe"));
        paths.push(PathBuf::from(&ad).join("Microsoft\\WinGet\\Links\\yt-dlp.exe"));
        paths.push(PathBuf::from(&ad).join("Programs\\yt-dlp\\yt-dlp.exe"));
        paths.push(PathBuf::from(&ad).join("scoop\\shims\\yt-dlp.exe"));
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
    if let Some(h) = home {
        paths.push(PathBuf::from(&h).join(".local\\bin\\yt-dlp.exe"));
        paths.push(PathBuf::from(&h).join("Videos\\yt-dlp.exe"));
        paths.push(PathBuf::from(&h).join("yt-dlp\\yt-dlp.exe"));
    }
    for p in paths {
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}
