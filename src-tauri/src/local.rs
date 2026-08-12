use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

const EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav", "opus", "aac", "wma"];

pub struct LocalScanner {
    db: std::sync::Arc<crate::db::Db>,
}

impl LocalScanner {
    pub fn new(db: std::sync::Arc<crate::db::Db>) -> Self {
        LocalScanner { db }
    }

    pub fn scan_dirs<F: Fn(usize, &str)>(&self, dirs: &[String], on_file: F) -> Result<usize> {
        let mut count = 0;
        for dir in dirs {
            if !std::path::Path::new(dir).exists() {
                continue;
            }
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !EXTENSIONS.contains(&ext.as_str()) {
                    continue;
                }
                let path_str = path.to_string_lossy().to_string();
                on_file(count, &path_str);
                match read_tags(&path_str) {
                    Ok(track) => {
                        let _ = self.db.upsert_track(&track);
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!("failed to read {path_str}: {e}");
                    }
                }
            }
        }
        Ok(count)
    }

    pub fn remove_deleted(&self) -> Result<Vec<String>> {
        let tracks = self.db.local_tracks()?;
        let mut removed = Vec::new();
        for t in &tracks {
            if let Some(p) = &t.path {
                if !std::path::Path::new(p).exists() {
                    self.db.delete_track(&t.id)?;
                    removed.push(t.id.clone());
                }
            }
        }
        Ok(removed)
    }
}

fn read_tags(path_str: &str) -> Result<crate::model::Track> {
    let path = Path::new(path_str);
    let file = lofty::read_from_path(path)?;
    let tags = file.primary_tag().or_else(|| file.first_tag());
    let tag_map = tags.cloned();
    let title = tags
        .and_then(|t| t.get_string(ItemKey::TrackTitle))
        .map(|s| s.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());
    let artists = tag_map
        .as_ref()
        .map(|t| {
            t.get_strings(ItemKey::TrackArtist)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let album = tag_map
        .as_ref()
        .and_then(|t| t.get_string(ItemKey::AlbumTitle))
        .map(|s| s.to_string());
    let album_artist = tag_map
        .as_ref()
        .and_then(|t| t.get_string(ItemKey::AlbumArtist))
        .map(|s| s.to_string());
    let year = tag_map
        .as_ref()
        .and_then(|t| t.get_string(ItemKey::RecordingDate))
        .and_then(|s| s.split('-').next())
        .and_then(|s| s.parse::<i32>().ok());
    let genre = tag_map
        .as_ref()
        .and_then(|t| t.get_string(ItemKey::Genre))
        .map(|s| s.to_string());
    let duration_ms = file.properties().duration().as_millis() as i64;

    Ok(crate::model::Track::local(
        path_str,
        &title,
        artists,
        album,
        album_artist,
        Some(duration_ms),
        year,
        genre,
    ))
}
