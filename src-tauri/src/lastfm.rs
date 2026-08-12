use crate::db::Db;
use crate::model::{AudioFeatures, Track};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SimilarResponse {
    similartracks: Similartracks,
}

#[derive(Debug, Deserialize)]
struct Similartracks {
    track: Vec<SimilarTrack>,
}

#[derive(Debug, Deserialize)]
struct SimilarTrack {
    name: String,
    artist: Artist,
    image: Vec<Image>,
}

#[derive(Debug, Deserialize)]
struct Artist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Image {
    size: String,
    text: String,
}

pub struct LastFm {
    pub http: reqwest::Client,
}

impl LastFm {
    pub fn new(http: reqwest::Client) -> Self {
        LastFm { http }
    }

    fn api_key(&self, db: &Db) -> Option<String> {
        db.get_settings().ok()?.lastfm_api_key.filter(|k| !k.is_empty())
    }

    pub async fn similar_tracks(&self, db: &Db, title: &str, artist: &str) -> Vec<Track> {
        let Some(key) = self.api_key(db) else {
            return Vec::new();
        };
        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=track.getsimilar&artist={}&track={}&api_key={}&limit=15&format=json",
            urlencoding(artist),
            urlencoding(title),
            key
        );
        let Ok(resp) = self
            .http
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        else {
            return Vec::new();
        };
        let Ok(json) = resp.json::<SimilarResponse>().await else {
            return Vec::new();
        };
        json.similartracks
            .track
            .into_iter()
            .filter_map(|t| {
                let artwork = t
                    .image
                    .into_iter()
                    .max_by_key(|i| match i.size.as_str() {
                        "small" => 1,
                        "medium" => 2,
                        "large" => 3,
                        "extralarge" => 4,
                        _ => 0,
                    })
                    .map(|i| i.text);
                let mut track = Track::spotify(
                    &format!("lastfm-{}-{}", t.artist.name, t.name),
                    &t.name,
                    vec![t.artist.name],
                    None,
                    artwork.filter(|a| !a.is_empty()),
                    None,
                    None,
                    None,
                );
                track.features = Some(AudioFeatures::default());
                Some(track)
            })
            .collect()
    }
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
