use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrackSource {
    Local,
    Spotify,
    Youtube,
    Deezer,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioFeatures {
    pub energy: Option<f32>,
    pub danceability: Option<f32>,
    pub valence: Option<f32>,
    pub acousticness: Option<f32>,
    pub tempo: Option<f32>,
    pub key: Option<i32>,
    pub mode: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub source: TrackSource,
    pub source_id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artwork: Option<String>,
    pub duration_ms: Option<i64>,
    pub path: Option<String>,
    pub isrc: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub features: Option<AudioFeatures>,
    pub stream_url: Option<String>,
    pub resolvable: bool,
}

impl Track {
    pub fn spotify(
        id: &str,
        title: &str,
        artists: Vec<String>,
        album: Option<String>,
        artwork: Option<String>,
        duration_ms: Option<i64>,
        isrc: Option<String>,
        year: Option<i32>,
    ) -> Self {
        Track {
            id: format!("spotify:{id}"),
            source: TrackSource::Spotify,
            source_id: id.to_string(),
            title: title.to_string(),
            artists,
            album,
            album_artist: None,
            artwork,
            duration_ms,
            path: None,
            isrc,
            year,
            genre: None,
            features: None,
            stream_url: None,
            resolvable: true,
        }
    }

    pub fn youtube(
        video_id: &str,
        title: &str,
        artist: Option<&str>,
        duration_secs: Option<f64>,
        artwork: Option<String>,
    ) -> Self {
        Track {
            id: format!("yt:{video_id}"),
            source: TrackSource::Youtube,
            source_id: video_id.to_string(),
            title: title.to_string(),
            artists: artist
                .map(|a| vec![a.to_string()])
                .unwrap_or_default(),
            album: None,
            album_artist: None,
            artwork,
            duration_ms: duration_secs.map(|s| (s * 1000.0) as i64),
            path: None,
            isrc: None,
            year: None,
            genre: None,
            features: None,
            stream_url: None,
            resolvable: true,
        }
    }

    pub fn local(
        path: &str,
        title: &str,
        artists: Vec<String>,
        album: Option<String>,
        album_artist: Option<String>,
        duration_ms: Option<i64>,
        year: Option<i32>,
        genre: Option<String>,
    ) -> Self {
        let id = format!("local:{}", sha1(path.as_bytes()));
        Track {
            id,
            source: TrackSource::Local,
            source_id: path.to_string(),
            title: title.to_string(),
            artists,
            album,
            album_artist,
            artwork: None,
            duration_ms,
            path: Some(path.to_string()),
            isrc: None,
            year,
            genre,
            features: None,
            stream_url: None,
            resolvable: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub artwork: Option<String>,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedStream {
    pub url: String,
    pub via: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub lastfm_api_key: Option<String>,
    pub library_dirs: Vec<String>,
    pub piped_instances: Vec<String>,
    pub auto_radio: bool,
    pub crossfade_seconds: f32,
    pub download_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            spotify_client_id: None,
            spotify_client_secret: None,
            lastfm_api_key: None,
            library_dirs: Vec::new(),
            piped_instances: vec![
                "https://pipedapi.kavin.rocks".to_string(),
                "https://pipedapi.adminforge.de".to_string(),
                "https://pipedapi.leptons.xyz".to_string(),
            ],
            auto_radio: true,
            crossfade_seconds: 0.0,
            download_dir: None,
        }
    }
}

fn sha1(input: &[u8]) -> String {
    let digest = sha1_smol::Sha1::from(input).digest();
    format!("{digest}")
}
