use crate::db::Db;
use crate::model::{AudioFeatures, Track};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

const AUTH_URL: &str = "https://accounts.spotify.com/api/token";
const API_URL: &str = "https://api.spotify.com/v1";
const REDIRECT_URI: &str = "http://localhost:29171/callback";

#[derive(Clone, Default)]
struct Token {
    access: Option<String>,
    expires_at: u64,
    refresh: Option<String>,
}

pub struct SpotifyClient {
    pub http: reqwest::Client,
    token: RwLock<Token>,
}

impl SpotifyClient {
    pub fn new(http: reqwest::Client) -> Self {
        SpotifyClient {
            http,
            token: RwLock::new(Token::default()),
        }
    }

    fn credentials(&self, db: &Db) -> (String, String) {
        let s = db.get_settings().unwrap_or_default();
        (
            s.spotify_client_id.unwrap_or_default(),
            s.spotify_client_secret.unwrap_or_default(),
        )
    }

    async fn ensure_token(&self, db: &Db) -> anyhow::Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        {
            let t = self.token.read().unwrap();
            if let Some(access) = &t.access {
                if now < t.expires_at.saturating_sub(60) {
                    return Ok(access.clone());
                }
            }
        }
        let (client_id, client_secret) = self.credentials(db);
        if client_id.is_empty() || client_secret.is_empty() {
            anyhow::bail!("Spotify client id/secret not configured");
        }
        let basic = B64.encode(format!("{client_id}:{client_secret}"));
        let mut form_data = std::collections::HashMap::new();
        if let Some(refresh) = self.token.read().unwrap().refresh.clone() {
            form_data.insert("grant_type", "refresh_token".to_string());
            form_data.insert("refresh_token", refresh);
        } else {
            form_data.insert("grant_type", "client_credentials".to_string());
        }
        let resp: serde_json::Value = self
            .http
            .post(AUTH_URL)
            .header("Authorization", format!("Basic {basic}"))
            .form(&form_data)
            .send()
            .await?
            .json()
            .await?;
        let access = resp
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("spotify token response: {resp}"))?
            .to_string();
        let expires_in = resp.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
        let mut t = self.token.write().unwrap();
        t.access = Some(access.clone());
        t.expires_at = now + expires_in;
        if let Some(r) = resp.get("refresh_token").and_then(|v| v.as_str()) {
            t.refresh = Some(r.to_string());
        }
        Ok(access)
    }

    pub fn auth_url(&self, db: &Db) -> anyhow::Result<String> {
        let (client_id, _) = self.credentials(db);
        if client_id.is_empty() {
            anyhow::bail!("Spotify client id not configured");
        }
        let scopes = "user-library-read playlist-read-private playlist-modify-private playlist-modify-public";
        let state = uuid::Uuid::new_v4();
        Ok(format!(
            "https://accounts.spotify.com/authorize?response_type=code&client_id={client_id}&redirect_uri={}&scope={}&state={state}",
            urlencoding(REDIRECT_URI),
            urlencoding(scopes)
        ))
    }

    pub async fn exchange_code(&self, db: &Db, code: &str) -> anyhow::Result<()> {
        let (client_id, client_secret) = self.credentials(db);
        let basic = B64.encode(format!("{client_id}:{client_secret}"));
        let form_data = [
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", REDIRECT_URI.to_string()),
        ];
        let resp: serde_json::Value = self
            .http
            .post(AUTH_URL)
            .header("Authorization", format!("Basic {basic}"))
            .form(&form_data)
            .send()
            .await?
            .json()
            .await?;
        let access = resp
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("exchange failed: {resp}"))?
            .to_string();
        let refresh = resp.get("refresh_token").and_then(|t| t.as_str()).map(|s| s.to_string());
        let expires_in = resp.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut t = self.token.write().unwrap();
        t.access = Some(access);
        t.refresh = refresh;
        t.expires_at = now + expires_in;
        Ok(())
    }

    pub fn connected(&self) -> bool {
        self.token.read().unwrap().access.is_some()
    }

    pub async fn search(&self, db: &Db, query: &str, limit: u32) -> anyhow::Result<Vec<Track>> {
        let token = self.ensure_token(db).await?;
        let url = format!("{API_URL}/search?q={}&type=track&limit={limit}", urlencoding(query));
        let resp: serde_json::Value = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await?;
        Ok(parse_tracks(&resp))
    }

    pub async fn get_audio_features(&self, db: &Db, id: &str) -> anyhow::Result<AudioFeatures> {
        let token = self.ensure_token(db).await?;
        let url = format!("{API_URL}/audio-features/{id}");
        let resp: serde_json::Value = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await?;
        Ok(AudioFeatures {
            energy: resp.get("energy").and_then(|v| v.as_f64()).map(|f| f as f32),
            danceability: resp
                .get("danceability")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32),
            valence: resp.get("valence").and_then(|v| v.as_f64()).map(|f| f as f32),
            acousticness: resp
                .get("acousticness")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32),
            tempo: resp.get("tempo").and_then(|v| v.as_f64()).map(|f| f as f32),
            key: resp.get("key").and_then(|v| v.as_i64()).map(|i| i as i32),
            mode: resp.get("mode").and_then(|v| v.as_i64()).map(|i| i as i32),
        })
    }

    pub async fn recommendations(
        &self,
        db: &Db,
        seed_tracks: &[String],
        limit: u32,
        features: Option<&AudioFeatures>,
    ) -> anyhow::Result<Vec<Track>> {
        let token = self.ensure_token(db).await?;
        let mut params = vec![format!("limit={limit}")];
        if !seed_tracks.is_empty() {
            params.push(format!("seed_tracks={}", seed_tracks.join(",")));
        }
        if let Some(f) = features {
            if let Some(v) = f.energy {
                params.push(format!("target_energy={v:.3}"));
            }
            if let Some(v) = f.danceability {
                params.push(format!("target_danceability={v:.3}"));
            }
            if let Some(v) = f.valence {
                params.push(format!("target_valence={v:.3}"));
            }
            if let Some(v) = f.acousticness {
                params.push(format!("target_acousticness={v:.3}"));
            }
            if let Some(v) = f.tempo {
                params.push(format!("target_tempo={v:.0}"));
            }
        }
        let url = format!("{API_URL}/recommendations?{}", params.join("&"));
        let resp: serde_json::Value = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await?;
        Ok(parse_tracks(&resp))
    }

    pub async fn playlist_tracks(&self, db: &Db, playlist_id: &str) -> anyhow::Result<Vec<Track>> {
        let token = self.ensure_token(db).await?;
        let url = format!("{API_URL}/playlists/{playlist_id}/tracks?limit=50");
        let resp: serde_json::Value = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await?;
        Ok(parse_tracks(&resp))
    }
}

fn parse_tracks(value: &serde_json::Value) -> Vec<Track> {
    let mut out = Vec::new();
    let items = value
        .get("tracks")
        .and_then(|t| t.get("items"))
        .or_else(|| value.get("items"))
        .and_then(|i| i.as_array());
    let Some(items) = items else { return out };
    for it in items {
        let id = match it.get("id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };
        let title = it.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let mut artists = Vec::new();
        if let Some(arr) = it.get("artists").and_then(|a| a.as_array()) {
            for a in arr {
                if let Some(n) = a.get("name").and_then(|v| v.as_str()) {
                    artists.push(n.to_string());
                }
            }
        }
        let album = it
            .get("album")
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let artwork = it
            .get("album")
            .and_then(|a| a.get("images"))
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|img| img.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let duration_ms = it.get("duration_ms").and_then(|v| v.as_i64());
        let isrc = it
            .get("external_ids")
            .and_then(|e| e.get("isrc"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let year = it
            .get("album")
            .and_then(|a| a.get("release_date"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse::<i32>().ok());
        let mut track = Track::spotify(
            id,
            title,
            artists,
            album,
            artwork,
            duration_ms,
            isrc,
            year,
        );
        track.features = it
            .get("audio_features")
            .and_then(|f| f.as_object())
            .map(|f| AudioFeatures {
                energy: f.get("energy").and_then(|v| v.as_f64()).map(|v| v as f32),
                danceability: f.get("danceability").and_then(|v| v.as_f64()).map(|v| v as f32),
                valence: f.get("valence").and_then(|v| v.as_f64()).map(|v| v as f32),
                acousticness: f.get("acousticness").and_then(|v| v.as_f64()).map(|v| v as f32),
                tempo: f.get("tempo").and_then(|v| v.as_f64()).map(|v| v as f32),
                key: f.get("key").and_then(|v| v.as_i64()).map(|v| v as i32),
                mode: f.get("mode").and_then(|v| v.as_i64()).map(|v| v as i32),
            });
        out.push(track);
    }
    out
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
