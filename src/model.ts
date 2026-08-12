export type TrackSource = "local" | "spotify" | "youtube" | "deezer";

export interface AudioFeatures {
  energy?: number;
  danceability?: number;
  valence?: number;
  acousticness?: number;
  tempo?: number;
  key?: number;
  mode?: number;
}

export interface Track {
  id: string;
  source: TrackSource;
  source_id: string;
  title: string;
  artists: string[];
  album?: string;
  album_artist?: string;
  artwork?: string;
  duration_ms?: number;
  path?: string;
  isrc?: string;
  year?: number;
  genre?: string;
  features?: AudioFeatures;
  stream_url?: string;
  resolvable: boolean;
}

export interface Playlist {
  id: string;
  name: string;
  description?: string;
  artwork?: string;
  track_ids: string[];
}

export interface ResolvedStream {
  url: string;
  expires_at?: number;
  via: string;
}

export interface AppSettings {
  spotify_client_id?: string;
  spotify_client_secret?: string;
  lastfm_api_key?: string;
  library_dirs: string[];
  piped_instances: string[];
  auto_radio: boolean;
  crossfade_seconds: number;
  download_dir?: string;
}

export interface SettingsState {
  connected: boolean;
  spotify_connected: boolean;
  lastfm_configured: boolean;
  radio_enabled: boolean;
}

export const sourceLabel: Record<TrackSource, string> = {
  local: "Local",
  spotify: "Spotify",
  youtube: "YouTube",
  deezer: "Deezer",
};

export function formatDuration(ms?: number): string {
  if (!ms || ms <= 0) return "--:--";
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}
