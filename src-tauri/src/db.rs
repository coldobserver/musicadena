use crate::model::{AppSettings, AudioFeatures, Playlist, Track, TrackSource};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS tracks (
                 id TEXT PRIMARY KEY,
                 source TEXT NOT NULL,
                 source_id TEXT NOT NULL,
                 title TEXT NOT NULL,
                 artists TEXT NOT NULL,
                 album TEXT,
                 album_artist TEXT,
                 artwork TEXT,
                 duration_ms INTEGER,
                 path TEXT,
                 isrc TEXT,
                 year INTEGER,
                 genre TEXT,
                 energy REAL,
                 danceability REAL,
                 valence REAL,
                 acousticness REAL,
                 tempo REAL,
                 pkey INTEGER,
                 pmode INTEGER,
                 resolvable INTEGER NOT NULL DEFAULT 1,
                 updated_at INTEGER
             );
             CREATE TABLE IF NOT EXISTS playlists (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 description TEXT,
                 artwork TEXT,
                 created_at INTEGER,
                 updated_at INTEGER
             );
             CREATE TABLE IF NOT EXISTS playlist_tracks (
                 playlist_id TEXT NOT NULL,
                 track_id TEXT NOT NULL,
                 position INTEGER NOT NULL,
                 PRIMARY KEY (playlist_id, track_id)
             );
             CREATE TABLE IF NOT EXISTS history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 track_id TEXT NOT NULL,
                 played_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS stream_cache (
                 track_id TEXT PRIMARY KEY,
                 video_id TEXT,
                 resolved_url TEXT,
                 expires_at INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
             CREATE INDEX IF NOT EXISTS idx_history_played ON history(played_at DESC);",
        )?;
        Ok(Db(Mutex::new(conn)))
    }

    fn row_to_track(row: &rusqlite::Row) -> rusqlite::Result<Track> {
        let source_str: String = row.get("source")?;
        let artists_json: String = row.get("artists")?;
        let features = AudioFeatures {
            energy: row.get("energy")?,
            danceability: row.get("danceability")?,
            valence: row.get("valence")?,
            acousticness: row.get("acousticness")?,
            tempo: row.get("tempo")?,
            key: row.get("pkey")?,
            mode: row.get("pmode")?,
        };
        let has_features = features.energy.is_some()
            || features.danceability.is_some()
            || features.valence.is_some()
            || features.acousticness.is_some()
            || features.tempo.is_some()
            || features.key.is_some()
            || features.mode.is_some();
        let resolvable_int: i64 = row.get("resolvable")?;
        Ok(Track {
            id: row.get("id")?,
            source: match source_str.as_str() {
                "spotify" => TrackSource::Spotify,
                "youtube" => TrackSource::Youtube,
                "deezer" => TrackSource::Deezer,
                _ => TrackSource::Local,
            },
            source_id: row.get("source_id")?,
            title: row.get("title")?,
            artists: serde_json::from_str(&artists_json).unwrap_or_default(),
            album: row.get("album")?,
            album_artist: row.get("album_artist")?,
            artwork: row.get("artwork")?,
            duration_ms: row.get("duration_ms")?,
            path: row.get("path")?,
            isrc: row.get("isrc")?,
            year: row.get("year")?,
            genre: row.get("genre")?,
            features: if has_features { Some(features) } else { None },
            stream_url: None,
            resolvable: resolvable_int != 0,
        })
    }

    pub fn upsert_track(&self, t: &Track) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        let artists = serde_json::to_string(&t.artists).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT INTO tracks (
                 id, source, source_id, title, artists, album, album_artist, artwork,
                 duration_ms, path, isrc, year, genre,
                 energy, danceability, valence, acousticness, tempo, pkey, pmode,
                 resolvable, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
             ON CONFLICT(id) DO UPDATE SET
                 source=excluded.source, source_id=excluded.source_id,
                 title=excluded.title, artists=excluded.artists,
                 album=excluded.album, album_artist=excluded.album_artist,
                 artwork=excluded.artwork, duration_ms=excluded.duration_ms,
                 path=excluded.path, isrc=excluded.isrc, year=excluded.year,
                 genre=excluded.genre, energy=excluded.energy,
                 danceability=excluded.danceability, valence=excluded.valence,
                 acousticness=excluded.acousticness, tempo=excluded.tempo,
                 pkey=excluded.pkey, pmode=excluded.pmode,
                 resolvable=excluded.resolvable, updated_at=excluded.updated_at",
            params![
                t.id,
                source_str(t.source),
                t.source_id,
                t.title,
                artists,
                t.album,
                t.album_artist,
                t.artwork,
                t.duration_ms,
                t.path,
                t.isrc,
                t.year,
                t.genre,
                t.features.as_ref().and_then(|f| f.energy),
                t.features.as_ref().and_then(|f| f.danceability),
                t.features.as_ref().and_then(|f| f.valence),
                t.features.as_ref().and_then(|f| f.acousticness),
                t.features.as_ref().and_then(|f| f.tempo),
                t.features.as_ref().and_then(|f| f.key),
                t.features.as_ref().and_then(|f| f.mode),
                if t.resolvable { 1 } else { 0 },
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn get_track(&self, id: &str) -> anyhow::Result<Option<Track>> {
        let conn = self.0.lock().unwrap();
        let row = conn
            .query_row("SELECT * FROM tracks WHERE id = ?1", params![id], |r| {
                Self::row_to_track(r)
            })
            .optional()?;
        Ok(row)
    }

    pub fn get_tracks(&self, ids: &[String]) -> anyhow::Result<Vec<Track>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.0.lock().unwrap();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT * FROM tracks WHERE id IN ({placeholders})");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(ids.iter()))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Self::row_to_track(&row)?);
        }
        Ok(out)
    }

    pub fn all_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tracks ORDER BY title")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Self::row_to_track(&row)?);
        }
        Ok(out)
    }

    pub fn local_tracks(&self) -> anyhow::Result<Vec<Track>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tracks WHERE source = 'local' ORDER BY title")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Self::row_to_track(&row)?);
        }
        Ok(out)
    }

    pub fn search_tracks(&self, q: &str) -> anyhow::Result<Vec<Track>> {
        let conn = self.0.lock().unwrap();
        let like = format!("%{q}%");
        let mut stmt = conn.prepare(
            "SELECT * FROM tracks
             WHERE title LIKE ?1 OR artists LIKE ?2 OR album LIKE ?3
             ORDER BY title LIMIT 200",
        )?;
        let mut rows = stmt.query(params![like, like, like])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Self::row_to_track(&row)?);
        }
        Ok(out)
    }

    pub fn delete_track(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        conn.execute("DELETE FROM history WHERE track_id = ?1", params![id])?;
        Ok(())
    }

    pub fn create_playlist(&self, name: &str) -> anyhow::Result<Playlist> {
        let id = format!("pl:{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().timestamp();
        let conn = self.0.lock().unwrap();
        conn.execute(
            "INSERT INTO playlists (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, name, now],
        )?;
        Ok(Playlist {
            id,
            name: name.to_string(),
            description: None,
            artwork: None,
            track_ids: Vec::new(),
        })
    }

    pub fn get_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM playlists ORDER BY created_at DESC")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get("id")?;
            let mut stmt2 = conn.prepare(
                "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position",
            )?;
            let track_ids = stmt2
                .query_map(params![id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            out.push(Playlist {
                id,
                name: row.get("name")?,
                description: row.get("description")?,
                artwork: row.get("artwork")?,
                track_ids,
            });
        }
        Ok(out)
    }

    pub fn get_playlist(&self, id: &str) -> anyhow::Result<Option<Playlist>> {
        let conn = self.0.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT * FROM playlists WHERE id = ?1",
                params![id],
                |r| {
                    let id: String = r.get("id")?;
                    let name: String = r.get("name")?;
                    let description: Option<String> = r.get("description")?;
                    let artwork: Option<String> = r.get("artwork")?;
                    Ok((id, name, description, artwork))
                },
            )
            .optional()?;
        let Some((id, name, description, artwork)) = row else {
            return Ok(None);
        };
        let mut stmt =
            conn.prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position")?;
        let track_ids = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Playlist {
            id,
            name,
            description,
            artwork,
            track_ids,
        }))
    }

    pub fn add_to_playlist(&self, playlist_id: &str, track_ids: &[String]) -> anyhow::Result<Playlist> {
        let conn = self.0.lock().unwrap();
        let next: i64 = conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |r| r.get(0),
        )?;
        for (i, tid) in track_ids.iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
                params![playlist_id, tid, next + i as i64],
            )?;
        }
        conn.execute(
            "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().timestamp(), playlist_id],
        )?;
        drop(conn);
        Ok(self.get_playlist(playlist_id)?.unwrap())
    }

    pub fn remove_from_playlist(&self, playlist_id: &str, track_id: &str) -> anyhow::Result<Playlist> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        conn.execute(
            "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().timestamp(), playlist_id],
        )?;
        drop(conn);
        Ok(self.get_playlist(playlist_id)?.unwrap())
    }

    pub fn delete_playlist(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        conn.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", params![id])?;
        Ok(())
    }

    pub fn record_play(&self, track_id: &str) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "INSERT INTO history (track_id, played_at) VALUES (?1, ?2)",
            params![track_id, chrono::Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn history_ids(&self, limit: usize) -> anyhow::Result<Vec<String>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT track_id FROM history ORDER BY played_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn history_tracks(&self, limit: usize) -> anyhow::Result<Vec<Track>> {
        let ids = self.history_ids(limit)?;
        let mut seen = Vec::new();
        for id in &ids {
            if !seen.contains(id) {
                seen.push(id.clone());
            }
        }
        self.get_tracks(&seen)
    }

    pub fn clear_history(&self) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn get_stream_cache(
        &self,
        track_id: &str,
    ) -> anyhow::Result<Option<(Option<String>, Option<String>, Option<i64>)>> {
        let conn = self.0.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT video_id, resolved_url, expires_at FROM stream_cache WHERE track_id = ?1",
                params![track_id],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn set_stream_cache(
        &self,
        track_id: &str,
        video_id: Option<&str>,
        resolved_url: Option<&str>,
        expires_at: Option<i64>,
    ) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "INSERT INTO stream_cache (track_id, video_id, resolved_url, expires_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(track_id) DO UPDATE SET
                 video_id = excluded.video_id,
                 resolved_url = excluded.resolved_url,
                 expires_at = excluded.expires_at",
            params![track_id, video_id, resolved_url, expires_at],
        )?;
        Ok(())
    }

    pub fn get_settings(&self) -> anyhow::Result<AppSettings> {
        let mut s = AppSettings::default();
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            match key.as_str() {
                "spotify_client_id" => s.spotify_client_id = Some(value),
                "spotify_client_secret" => s.spotify_client_secret = Some(value),
                "lastfm_api_key" => s.lastfm_api_key = Some(value),
                "library_dirs" => s.library_dirs = serde_json::from_str(&value).unwrap_or_default(),
                "piped_instances" => s.piped_instances = serde_json::from_str(&value).unwrap_or_default(),
                "auto_radio" => s.auto_radio = value == "true",
                "crossfade_seconds" => s.crossfade_seconds = value.parse().unwrap_or(0.0),
                "download_dir" => s.download_dir = Some(value),
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn set_settings(&self, s: &AppSettings) -> anyhow::Result<()> {
        let conn = self.0.lock().unwrap();
        let vals: Vec<(&str, String)> = vec![
            ("spotify_client_id", s.spotify_client_id.clone().unwrap_or_default()),
            ("spotify_client_secret", s.spotify_client_secret.clone().unwrap_or_default()),
            ("lastfm_api_key", s.lastfm_api_key.clone().unwrap_or_default()),
            ("library_dirs", serde_json::to_string(&s.library_dirs)?),
            ("piped_instances", serde_json::to_string(&s.piped_instances)?),
            ("auto_radio", s.auto_radio.to_string()),
            ("crossfade_seconds", s.crossfade_seconds.to_string()),
            ("download_dir", s.download_dir.clone().unwrap_or_default()),
        ];
        for (k, v) in vals {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![k, v],
            )?;
        }
        Ok(())
    }
}

pub fn source_str(s: TrackSource) -> &'static str {
    match s {
        TrackSource::Local => "local",
        TrackSource::Spotify => "spotify",
        TrackSource::Youtube => "youtube",
        TrackSource::Deezer => "deezer",
    }
}
