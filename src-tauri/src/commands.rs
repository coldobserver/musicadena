use crate::db::Db;
use crate::lastfm::LastFm;
use crate::local::LocalScanner;
use crate::model::{AppSettings, Playlist, ResolvedStream, Track};
use crate::radio::RadioEngine;
use crate::spotify::SpotifyClient;
use crate::stream::StreamManager;
use std::sync::Arc;
use tauri::{Emitter, State};

pub struct AppState {
    pub db: Arc<Db>,
    pub http: reqwest::Client,
    pub stream: StreamManager,
    pub spotify: SpotifyClient,
    pub lastfm: LastFm,
}

impl AppState {
    pub fn new(db: Arc<Db>, http: reqwest::Client) -> Self {
        let settings = db.get_settings().unwrap_or_default();
        let stream = StreamManager::new(http.clone(), settings.piped_instances.clone());
        let spotify = SpotifyClient::new(http.clone());
        let lastfm = LastFm::new(http.clone());
        AppState {
            db,
            http,
            stream,
            spotify,
            lastfm,
        }
    }
}

#[tauri::command]
pub async fn search_all(
    state: State<'_, AppState>,
    query: String,
    sources: Vec<String>,
    limit: Option<u32>,
) -> Result<Vec<Track>, String> {
    let limit = limit.unwrap_or(30);
    let mut out = Vec::new();

    if sources.contains(&"local".to_string()) {
        if let Ok(local) = state.db.search_tracks(&query) {
            out.extend(local);
        }
    }

    if sources.contains(&"spotify".to_string()) && state.spotify.connected() {
        if let Ok(mut spotify) = state.spotify.search(&state.db, &query, limit).await {
            for t in spotify.drain(..) {
                if !out.iter().any(|o| o.id == t.id) {
                    out.push(t);
                }
            }
        }
    }

    if sources.contains(&"youtube".to_string()) || sources.contains(&"deezer".to_string()) {
        let mut videos: Vec<crate::stream::piped::PipedVideo> = Vec::new();
        if let Ok(mut via_dlp) = state
            .stream
            .ytdlp
            .search_results(&query, (limit as usize).min(15))
            .await
        {
            videos.extend(via_dlp.drain(..).filter_map(|v| {
                if v.video_id.is_empty() {
                    return None;
                }
                Some(crate::stream::piped::PipedVideo {
                    url: Some(format!("/watch?v={}", v.video_id)),
                    title: v.title,
                    uploader_name: v.uploader,
                    duration: v.duration_secs,
                    thumbnail: v.thumbnail,
                })
            }));
        }
        if videos.is_empty() {
            videos = state.stream.piped.search_results(&query, "").await;
        }
        if videos.is_empty() {
            if let Ok(mut inner) = crate::stream::innertube::search(&state.http, &query).await {
                videos.extend(inner.drain(..).filter_map(|v| {
                    let video_id = v.video_id;
                    if video_id.is_empty() {
                        return None;
                    }
                    Some(crate::stream::piped::PipedVideo {
                        url: Some(format!("/watch?v={video_id}")),
                        title: v.title,
                        uploader_name: v.uploader,
                        duration: v.duration_secs.map(|d| d as i64),
                        thumbnail: v.thumbnail,
                    })
                }));
            }
        }
        for video in videos {
            if out.len() >= limit as usize {
                break;
            }
            if let Some(vid) = crate::stream::piped::extract_video_id(
                video.url.as_deref().unwrap_or(""),
            ) {
                let track = Track::youtube(
                    &vid,
                    &video.title,
                    video.uploader_name.as_deref(),
                    video.duration.as_ref().map(|d| *d as f64),
                    video.thumbnail.clone(),
                );
                if !out.iter().any(|o| o.id == track.id) {
                    out.push(track);
                }
            }
        }
    }

    out.truncate(limit as usize);
    Ok(out)
}

#[tauri::command]
pub async fn resolve_stream(
    state: State<'_, AppState>,
    track: Track,
) -> Result<ResolvedStream, String> {
    state
        .stream
        .resolve(&state.db, &track)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    state.db.local_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_library(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    dir: Option<String>,
) -> Result<usize, String> {
    let settings = state.db.get_settings().unwrap_or_default();
    let dirs = if let Some(d) = dir {
        vec![d]
    } else {
        settings.library_dirs
    };
    let scanner = LocalScanner::new(state.db.clone());
    let handle = app.clone();
    tokio::task::spawn_blocking(move || {
        scanner.scan_dirs(&dirs, |count, path| {
            let _ = handle.emit(
                "scan_progress",
                serde_json::json!({ "count": count, "file": path }),
            );
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    state.db.get_playlists().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Playlist>, String> {
    state.db.get_playlist(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_playlist(
    state: State<'_, AppState>,
    name: String,
) -> Result<Playlist, String> {
    state.db.create_playlist(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_to_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    tracks: Vec<Track>,
) -> Result<Playlist, String> {
    for t in &tracks {
        state.db.upsert_track(t).map_err(|e| e.to_string())?;
    }
    let ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
    state
        .db
        .add_to_playlist(&playlist_id, &ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_from_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    track_id: String,
) -> Result<Playlist, String> {
    state
        .db
        .remove_from_playlist(&playlist_id, &track_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<(), String> {
    state
        .db
        .delete_playlist(&playlist_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Vec<Track>, String> {
    let Some(playlist) = state.db.get_playlist(&playlist_id).map_err(|e| e.to_string())? else {
        return Ok(Vec::new());
    };
    let mut tracks = state
        .db
        .get_tracks(&playlist.track_ids)
        .map_err(|e| e.to_string())?;
    tracks.sort_by_key(|t| {
        playlist
            .track_ids
            .iter()
            .position(|id| id == &t.id)
            .unwrap_or(usize::MAX)
    });
    Ok(tracks)
}

#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<Track>, String> {
    state
        .db
        .history_tracks(limit.unwrap_or(100) as usize)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.db.clear_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn radio_suggestions(
    state: State<'_, AppState>,
    track: Track,
    count: Option<u32>,
) -> Result<Vec<Track>, String> {
    let engine = RadioEngine {
        spotify: &state.spotify,
        lastfm: &state.lastfm,
        stream: &state.stream,
        db: &state.db,
    };
    Ok(engine.suggest(&track, count.unwrap_or(12).max(1) as usize).await)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state.db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    state
        .db
        .set_settings(&settings)
        .map_err(|e| e.to_string())?;
    state
        .stream
        .piped
        .set_instances(settings.piped_instances.clone());
    Ok(settings)
}

#[tauri::command]
pub async fn spotify_auth_url(state: State<'_, AppState>) -> Result<String, String> {
    state
        .spotify
        .auth_url(&state.db)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn spotify_callback(
    state: State<'_, AppState>,
    code: String,
) -> Result<bool, String> {
    state
        .spotify
        .exchange_code(&state.db, &code)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn spotify_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "connected": state.spotify.connected()
    }))
}

#[tauri::command]
pub async fn record_playback(
    state: State<'_, AppState>,
    track: Track,
) -> Result<(), String> {
    state.db.upsert_track(&track).map_err(|e| e.to_string())?;
    state
        .db
        .record_play(&track.id)
        .map_err(|e| e.to_string())
}
