use crate::db::Db;
use crate::lastfm::LastFm;
use crate::model::{Track, TrackSource};
use crate::spotify::SpotifyClient;
use crate::stream::StreamManager;

pub struct RadioEngine<'a> {
    pub spotify: &'a SpotifyClient,
    pub lastfm: &'a LastFm,
    pub stream: &'a StreamManager,
    pub db: &'a Db,
}

impl<'a> RadioEngine<'a> {
    pub async fn suggest(&self, track: &Track, count: usize) -> Vec<Track> {
        let mut candidates: Vec<Track> = Vec::new();
        let mut seen_ids: Vec<String> = Vec::new();

        if self.spotify.connected() {
            if let Ok(seed) = self.seed_from_track(track).await {
                if let Ok(mut recs) = self
                    .spotify
                    .recommendations(self.db, &seed, (count * 2) as u32, track.features.as_ref())
                    .await
                {
                    for r in recs.drain(..) {
                        if !seen_ids.contains(&r.id) {
                            seen_ids.push(r.id.clone());
                            candidates.push(r);
                        }
                    }
                }
            }
        }

        let similar = self
            .lastfm
            .similar_tracks(self.db, &track.title, &track.artists.join(" "))
            .await;
        for t in similar {
            if !seen_ids.contains(&t.id) {
                seen_ids.push(t.id.clone());
                candidates.push(t);
            }
        }

        let related_ids = self.stream.related_video_ids(self.db, track).await;
        for vid in related_ids {
            let tid = format!("yt:{vid}");
            if seen_ids.contains(&tid) {
                continue;
            }
            seen_ids.push(tid);
            candidates.push(Track::youtube(&vid, "Related track", None, None, None));
        }

        let history_ids = self.db.history_ids(500).unwrap_or_default();
        candidates.retain(|t| !history_ids.contains(&t.id));
        candidates.truncate(count);
        candidates
    }

    async fn seed_from_track(&self, track: &Track) -> anyhow::Result<Vec<String>> {
        if track.source == TrackSource::Spotify {
            return Ok(vec![track.source_id.clone()]);
        }
        let query = format!("{} {}", track.title, track.artists.join(" "));
        if let Ok(results) = self.spotify.search(self.db, &query, 1).await {
            if let Some(r) = results.first() {
                let seeds = vec![r.source_id.clone()];
                if r.features.is_none() {
                    if let Ok(f) = self
                        .spotify
                        .get_audio_features(self.db, &r.source_id)
                        .await
                    {
                        let _ = self.db.upsert_track(&Track {
                            features: Some(f),
                            ..r.clone()
                        });
                    }
                }
                return Ok(seeds);
            }
        }
        anyhow::bail!("no seed found")
    }
}
