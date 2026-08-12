import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, Playlist, ResolvedStream, Track } from "../model";

export const api = {
  search: (query: string, sources: string[], limit = 50): Promise<Track[]> =>
    invoke("search_all", { query, sources, limit }),

  getLibrary: (): Promise<Track[]> => invoke("get_library"),

  scanLibrary: (dir?: string): Promise<number> =>
    invoke("scan_library", { dir }),

  resolveStream: (track: Track): Promise<ResolvedStream> =>
    invoke("resolve_stream", { track }),

  getPlaylists: (): Promise<Playlist[]> => invoke("get_playlists"),

  getPlaylist: (id: string): Promise<Playlist | null> =>
    invoke("get_playlist", { id }),

  createPlaylist: (name: string): Promise<Playlist> =>
    invoke("create_playlist", { name }),

  addToPlaylist: (playlistId: string, tracks: Track[]): Promise<Playlist> =>
    invoke("add_to_playlist", { playlistId, tracks }),

  removeFromPlaylist: (
    playlistId: string,
    trackId: string,
  ): Promise<Playlist> =>
    invoke("remove_from_playlist", { playlistId, trackId }),

  deletePlaylist: (playlistId: string): Promise<void> =>
    invoke("delete_playlist", { playlistId }),

  getPlaylistTracks: (playlistId: string): Promise<Track[]> =>
    invoke("get_playlist_tracks", { playlistId }),

  getHistory: (limit = 100): Promise<Track[]> =>
    invoke("get_history", { limit }),

  clearHistory: (): Promise<void> => invoke("clear_history"),

  radioSuggestions: (track: Track, count = 15): Promise<Track[]> =>
    invoke("radio_suggestions", { track, count }),

  recordPlayback: (track: Track): Promise<void> =>
    invoke("record_playback", { track }),

  getSettings: (): Promise<AppSettings> => invoke("get_settings"),

  setSettings: (settings: AppSettings): Promise<AppSettings> =>
    invoke("set_settings", { settings }),

  spotifyAuthUrl: (): Promise<string> => invoke("spotify_auth_url"),

  spotifyCallback: (code: string): Promise<boolean> =>
    invoke("spotify_callback", { code }),

  spotifyStatus: (): Promise<{ connected: boolean }> =>
    invoke("spotify_status"),
};
