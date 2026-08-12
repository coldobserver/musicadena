use serde::Deserialize;
use std::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
pub struct PipedVideo {
    pub url: Option<String>,
    pub title: String,
    pub uploader_name: Option<String>,
    pub duration: Option<i64>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Option<Vec<PipedVideo>>,
}

#[derive(Debug, Deserialize)]
struct StreamsResponse {
    audio_streams: Vec<AudioStream>,
}

#[derive(Debug, Deserialize)]
struct AudioStream {
    url: Option<String>,
    bitrate: Option<u64>,
    mime_type: Option<String>,
}

pub struct PipedPool {
    pub instances: RwLock<Vec<String>>,
    http: reqwest::Client,
}

impl PipedPool {
    pub fn new(http: reqwest::Client, instances: Vec<String>) -> Self {
        PipedPool {
            instances: RwLock::new(instances),
            http,
        }
    }

    pub fn set_instances(&self, instances: Vec<String>) {
        *self.instances.write().unwrap() = instances;
    }

    pub fn add_known_instance(&self) {
        let mut guard = self.instances.write().unwrap();
        if !guard.iter().any(|i| i.contains("kavin.rocks")) {
            guard.insert(0, "https://pipedapi.kavin.rocks".to_string());
        }
    }

    async fn instances(&self) -> Vec<String> {
        self.instances.read().unwrap().clone()
    }

    pub async fn search_music(&self, title: &str, artist: &str) -> Option<PipedVideo> {
        self.search_results(title, artist).await.into_iter().next()
    }

    pub async fn search_results(&self, title: &str, artist: &str) -> Vec<PipedVideo> {
        let query = format!("{title} {artist}").trim().to_string();
        if query.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(f32, PipedVideo)> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for inst in self.instances().await {
            let url = format!(
                "{inst}/search?q={}&filter=music_songs",
                urlencoding(&query)
            );
            let Ok(resp) = self
                .http
                .get(&url)
                .timeout(std::time::Duration::from_secs(6))
                .send()
                .await
            else {
                continue;
            };
            let Ok(json) = resp.json::<SearchResponse>().await else {
                continue;
            };
            let items = json.items.unwrap_or_default();
            for v in items {
                if v.title.is_empty() {
                    continue;
                }
                let key = v.url.clone().unwrap_or_default();
                if !key.is_empty() && !seen.insert(key) {
                    continue;
                }
                let score = score_match(title, artist, &v.title, v.uploader_name.as_deref());
                scored.push((score, v));
            }
            if scored.iter().any(|(s, _)| *s >= 0.8) {
                break;
            }
        }
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        scored.into_iter().map(|(_, v)| v).take(20).collect()
    }

    pub async fn related(&self, video_id: &str) -> Vec<PipedVideo> {
        let mut out = Vec::new();
        for inst in self.instances().await {
            let url = format!("{inst}/next/videos/{video_id}");
            let Ok(resp) = self
                .http
                .get(&url)
                .timeout(std::time::Duration::from_secs(12))
                .send()
                .await
            else {
                continue;
            };
            let Ok(json) = resp.json::<serde_json::Value>().await else {
                continue;
            };
            if let Some(items) = json.get("relatedStreams").and_then(|v| v.as_array()) {
                for it in items {
                    let title = it.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let video_id = it
                        .get("url")
                        .and_then(|u| u.as_str())
                        .and_then(extract_video_id)
                        .unwrap_or_default();
                    if video_id.is_empty() {
                        continue;
                    }
                    out.push(PipedVideo {
                        url: Some(format!("/watch?v={video_id}")),
                        title: title.to_string(),
                        uploader_name: it
                            .get("uploaderName")
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string()),
                        duration: it.get("duration").and_then(|d| d.as_i64()),
                        thumbnail: None,
                    });
                }
                break;
            }
        }
        out
    }

    pub async fn resolve_audio(&self, video_id: &str) -> anyhow::Result<(String, u64)> {
        for inst in self.instances().await {
            let url = format!("{inst}/streams/{video_id}");
            let resp = self
                .http
                .get(&url)
                .timeout(std::time::Duration::from_secs(12))
                .send()
                .await?;
            if !resp.status().is_success() {
                continue;
            }
            let json = resp.json::<StreamsResponse>().await?;
            let mut best: Option<(u64, String)> = None;
            for s in json.audio_streams {
                let Some(url) = s.url else { continue };
                if let Some(mime) = &s.mime_type {
                    if !mime.starts_with("audio/") {
                        continue;
                    }
                }
                let bitrate = s.bitrate.unwrap_or(0);
                if best.is_none() || bitrate > best.as_ref().unwrap().0 {
                    best = Some((bitrate, url));
                }
            }
            if let Some((bitrate, url)) = best {
                return Ok((url, bitrate));
            }
        }
        anyhow::bail!("no piped instance returned audio for {video_id}")
    }
}

fn score_match(title: &str, artist: &str, video_title: &str, uploader: Option<&str>) -> f32 {
    let t = title.to_lowercase();
    let a = artist.to_lowercase();
    let vt = video_title.to_lowercase();
    let mut score = 0.0;
    if !t.is_empty() && vt.contains(&t) {
        score += 0.5;
    }
    if let Some(u) = uploader {
        let u = u.to_lowercase();
        if !a.is_empty() && (u.contains(&a) || a.contains(&u)) {
            score += 0.3;
        }
    }
    let title_words = t.split_whitespace().collect::<Vec<_>>();
    let mut matched = 0;
    for w in &title_words {
        if vt.contains(w) {
            matched += 1;
        }
    }
    if !title_words.is_empty() {
        score += 0.2 * (matched as f32 / title_words.len() as f32);
    }
    score
}

pub fn extract_video_id(url: &str) -> Option<String> {
    if let Some(stripped) = url.strip_prefix("/watch?v=") {
        if stripped.len() == 11 {
            return Some(stripped.to_string());
        }
    }
    if url.len() == 11 && url.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Some(url.to_string());
    }
    None
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
