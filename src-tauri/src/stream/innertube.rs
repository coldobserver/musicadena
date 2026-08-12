use serde_json::{json, Value};

const CLIENT: &str = "ANDROID_VR";
const VERSION: &str = "19.09.37";
const UA: &str =
    "com.google.android.apps.youtube.vr.oculus/1.58.21 (Linux; U; Android 10; VR) gzip";

#[derive(Debug, Clone)]
pub struct SearchVideo {
    pub video_id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_secs: Option<u64>,
    pub thumbnail: Option<String>,
}

async fn post(http: &reqwest::Client, path: &str, body: Value) -> reqwest::Result<Value> {
    http.post(format!("https://www.youtube.com/youtubei/v1/{path}"))
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .json(&body)
        .send()
        .await?
        .json()
        .await
}

pub async fn resolve(http: &reqwest::Client, video_id: &str) -> anyhow::Result<Option<String>> {
    let body = json!({
        "context": {
            "client": {
                "clientName": CLIENT,
                "clientVersion": VERSION,
                "androidSdkVersion": 30,
                "hl": "en"
            }
        },
        "videoId": video_id
    });

    let resp = post(http, "player", body).await?;

    let Some(formats) = resp
        .pointer("/streamingData/adaptiveFormats")
        .and_then(|v| v.as_array())
    else {
        return Ok(None);
    };

    let mut best: Option<(u64, String)> = None;
    for f in formats {
        let mime = f.get("mimeType").and_then(|m| m.as_str()).unwrap_or("");
        if !mime.starts_with("audio/") {
            continue;
        }
        let Some(url) = f.get("url").and_then(|u| u.as_str()) else {
            continue;
        };
        let bitrate = f
            .get("bitrate")
            .and_then(|b| b.as_u64())
            .unwrap_or(0);
        if best.is_none() || bitrate > best.as_ref().unwrap().0 {
            best = Some((bitrate, url.to_string()));
        }
    }
    Ok(best.map(|(_, url)| url))
}

pub async fn search(http: &reqwest::Client, query: &str) -> anyhow::Result<Vec<SearchVideo>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let body = json!({
        "context": {
            "client": {
                "clientName": CLIENT,
                "clientVersion": VERSION,
                "androidSdkVersion": 30,
                "hl": "en"
            }
        },
        "query": query
    });

    let resp = post(http, "search", body).await?;

    let mut out = Vec::new();
    collect_renderers(&resp, &mut out);
    out.truncate(20);
    Ok(out)
}

fn collect_renderers(node: &Value, out: &mut Vec<SearchVideo>) {
    if let Some(vid) = parse_renderer(node) {
        out.push(vid);
        return;
    }
    if let Some(map) = node.as_object() {
        for v in map.values() {
            collect_renderers(v, out);
        }
    } else if let Some(arr) = node.as_array() {
        for v in arr {
            collect_renderers(v, out);
        }
    }
}

fn parse_renderer(node: &Value) -> Option<SearchVideo> {
    let mut vid = None;
    let mut title = None;
    let mut uploader = None;
    let mut duration_secs = None;
    let mut thumbnail = None;

    for renderer in ["musicResponsiveListItemRenderer", "videoRenderer"] {
        let r = node.get(renderer)?;
        vid = video_id_of(r.get("navigationEndpoint"));
        title = text_of(r.get("title").or_else(|| r.get("name")));
        uploader = text_of(r.get("subtitle").or_else(|| r.get("ownerText")));
        duration_secs = text_of(r.get("fixedColumns"))
            .and_then(|t| parse_len(&t))
            .or_else(|| {
                text_of(r.get("lengthText")).and_then(|t| parse_len(&t))
            });
        thumbnail = r
            .pointer("/thumbnail/thumbnails")
            .and_then(|arr| arr.as_array())
            .and_then(|arr| arr.last())
            .and_then(|t| t.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                r.pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|t| t.get("url"))
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string())
            });
        if vid.is_some() {
            break;
        }
    }

    Some(SearchVideo {
        video_id: vid?,
        title: title.unwrap_or_default(),
        uploader,
        duration_secs,
        thumbnail,
    })
}

fn video_id_of(nav: Option<&Value>) -> Option<String> {
    let nav = nav?;
    nav.get("watchEndpoint")
        .or_else(|| nav.get("watchPlaylistEndpoint"))
        .and_then(|e| e.get("videoId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn text_of(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(runs) = value.get("runs").and_then(|r| r.as_array()) {
        let mut s = String::new();
        for run in runs {
            if let Some(t) = run.get("text").and_then(|t| t.as_str()) {
                s.push_str(t);
            }
        }
        if !s.trim().is_empty() {
            return Some(s);
        }
    }
    value
        .get("text")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

fn parse_len(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => parts[0].trim().parse().ok(),
        2 => {
            let m: u64 = parts[0].trim().parse().ok()?;
            let sec: u64 = parts[1].trim().parse().ok()?;
            Some(m * 60 + sec)
        }
        3 => {
            let h: u64 = parts[0].trim().parse().ok()?;
            let m: u64 = parts[1].trim().parse().ok()?;
            let sec: u64 = parts[2].trim().parse().ok()?;
            Some(h * 3600 + m * 60 + sec)
        }
        _ => None,
    }
}
