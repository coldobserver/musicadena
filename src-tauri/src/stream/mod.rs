pub mod innertube;
pub mod piped;
pub mod ytdlp;

use crate::db::Db;
use crate::model::{ResolvedStream, Track, TrackSource};
use piped::PipedPool;
use std::sync::RwLock;
use ytdlp::YtDlp;

pub struct StreamManager {
    pub http: reqwest::Client,
    pub piped: PipedPool,
    pub ytdlp: YtDlp,
    pub ytdlp_available: RwLock<bool>,
}

impl StreamManager {
    pub fn new(http: reqwest::Client, instances: Vec<String>) -> Self {
        let ytdlp = YtDlp::new();
        let ytdlp_available = check_available(&ytdlp);
        StreamManager {
            http: http.clone(),
            piped: PipedPool::new(http, instances),
            ytdlp,
            ytdlp_available: RwLock::new(ytdlp_available),
        }
    }

    pub async fn resolve(&self, db: &Db, track: &Track) -> anyhow::Result<ResolvedStream> {
        let now = chrono::Utc::now().timestamp();
        if let Some((_, Some(url), Some(exp))) = db.get_stream_cache(&track.id)? {
            if exp > now {
                return Ok(ResolvedStream {
                    url,
                    via: "cache".to_string(),
                });
            }
        }
        let video_id = self.video_id_for(db, track).await?;
        let (url, via) = self.resolve_video(&video_id).await?;
        db.set_stream_cache(&track.id, Some(&video_id), Some(&url), Some(now + 3600))?;
        Ok(ResolvedStream { url, via })
    }

    pub async fn video_id_for(&self, db: &Db, track: &Track) -> anyhow::Result<String> {
        if track.source == TrackSource::Youtube {
            return Ok(track.source_id.clone());
        }
        if let Some((Some(vid), _, _)) = db.get_stream_cache(&track.id)? {
            return Ok(vid);
        }
        let artist = track.artists.join(" ");
        let candidate = self.piped.search_music(&track.title, &artist).await;
        if let Some(v) = candidate {
            if let Some(vid) = piped::extract_video_id(v.url.as_deref().unwrap_or("")) {
                db.set_stream_cache(&track.id, Some(&vid), None, None)?;
                return Ok(vid);
            }
        }
        if *self.ytdlp_available.read().unwrap() {
            let query = format!("{} {} audio", track.title, artist);
            if let Ok(Some(vid)) = self.ytdlp.search_video_id(&query).await {
                db.set_stream_cache(&track.id, Some(&vid), None, None)?;
                return Ok(vid);
            }
        }
        anyhow::bail!("could not find a YouTube match for '{}'", track.title)
    }

    pub async fn resolve_video(&self, video_id: &str) -> anyhow::Result<(String, String)> {
        if let Ok(Some(url)) = innertube::resolve(&self.http, video_id).await {
            return Ok((url, "innertube".to_string()));
        }
        if let Ok((url, _)) = self.piped.resolve_audio(video_id).await {
            return Ok((url, "piped".to_string()));
        }
        if *self.ytdlp_available.read().unwrap() {
            if let Ok(url) = self.ytdlp.resolve_url(video_id).await {
                return Ok((url, "yt-dlp".to_string()));
            }
        }
        anyhow::bail!("all resolvers failed for {video_id}")
    }

    pub async fn related_video_ids(&self, db: &Db, track: &Track) -> Vec<String> {
        let Ok(video_id) = self.video_id_for(db, track).await else {
            return Vec::new();
        };
        self.piped
            .related(&video_id)
            .await
            .into_iter()
            .filter_map(|v| piped::extract_video_id(v.url.as_deref().unwrap_or("")))
            .collect()
    }
}

fn check_available(ytdlp: &YtDlp) -> bool {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(&ytdlp.binary);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    cmd.arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl Default for StreamManager {
    fn default() -> Self {
        StreamManager::new(reqwest::Client::new(), Vec::new())
    }
}
