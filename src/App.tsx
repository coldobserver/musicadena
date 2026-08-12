import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { usePlayer } from "./store/player";
import { api } from "./lib/ipc";
import type { Track, TrackSource, Playlist } from "./model";
import { formatDuration } from "./model";
import {
  Search,
  Music,
  ListMusic,
  Radio,
  Library,
  Settings,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  Repeat,
  Shuffle,
  Plus,
  ChevronRight,
  ChevronLeft,
  X,
  Trash2,
  FolderPlus,
  ArrowLeft,
  ListPlus,
} from "lucide-react";

type View = "home" | "search" | "library" | "playlists" | "playlist-detail" | "settings" | "history";

export default function App() {
  const [view, setView] = useState<View>("home");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Track[]>([]);
  const [library, setLibrary] = useState<Track[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState("");
  const [queueOpen, setQueueOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [playlistId, setPlaylistId] = useState<string | null>(null);
  const player = usePlayer();

  useEffect(() => {
    api.getLibrary().then(setLibrary).catch(() => {});
  }, []);

  const doSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    setSearchError("");
    try {
      const res = await api.search(query, ["spotify", "youtube", "local"], 40);
      setResults(res);
    } catch (e) {
      setSearchError(String(e));
    }
    setSearching(false);
  };

  const playAll = (tracks: Track[], idx = 0) => {
    player.playCollection(tracks, idx);
  };

  return (
    <div className="flex flex-col h-screen bg-gray-950 text-gray-100 select-none overflow-hidden">
      <div className="flex flex-1 min-h-0">
        {sidebarOpen && (
          <aside className="w-56 flex-shrink-0 bg-gray-900 border-r border-gray-800 flex flex-col">
            <div className="px-4 pt-5 pb-2 text-lg font-semibold tracking-tight text-emerald-400">
              Musicadena
            </div>
            <nav className="flex-1 px-2 py-2 space-y-0.5">
              <NavBtn active={view === "home"} onClick={() => setView("home")}>
                <Library size={18} /> Home
              </NavBtn>
              <NavBtn active={view === "search"} onClick={() => setView("search")}>
                <Search size={18} /> Search
              </NavBtn>
              <NavBtn active={view === "library"} onClick={() => setView("library")}>
                <Music size={18} /> Local Library
              </NavBtn>
              <NavBtn active={view === "playlists"} onClick={() => setView("playlists")}>
                <ListMusic size={18} /> Playlists
              </NavBtn>
              <NavBtn active={view === "history"} onClick={() => setView("history")}>
                <Radio size={18} /> History
              </NavBtn>
              <div className="border-t border-gray-800 my-2" />
              <NavBtn active={view === "settings"} onClick={() => setView("settings")}>
                <Settings size={18} /> Settings
              </NavBtn>
            </nav>
          </aside>
        )}

        <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <header className="flex items-center gap-2 px-4 py-3 border-b border-gray-800 bg-gray-900/80 backdrop-blur-sm flex-shrink-0">
          <button onClick={() => setSidebarOpen(!sidebarOpen)} className="p-1 hover:bg-gray-800 rounded">
            {sidebarOpen ? <ChevronLeft size={18} /> : <ChevronRight size={18} />}
          </button>
          {view === "search" && (
            <form
              className="flex-1 flex gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                doSearch();
              }}
            >
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search songs, artists, albums..."
                className="flex-1 bg-gray-800 border border-gray-700 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500"
              />
              <button type="submit" className="bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 rounded-md text-sm font-medium">
                Search
              </button>
            </form>
          )}
          {view === "home" && <span className="text-lg font-medium">Home</span>}
          {view === "library" && <span className="text-lg font-medium">Local Library</span>}
          {view === "playlists" && <span className="text-lg font-medium">Playlists</span>}
          {view === "history" && <span className="text-lg font-medium">Play History</span>}
          {view === "settings" && <span className="text-lg font-medium">Settings</span>}
        </header>

        <div className="flex-1 overflow-y-auto p-4">
          {view === "home" && (
            <HomeView library={library} onPlayAll={playAll} />
          )}
          {view === "search" && (
            <>
              {searching && (
                <div className="flex-shrink-0 h-1 bg-gray-800 overflow-hidden">
                  <div className="h-full w-1/3 bg-emerald-500 rounded-full animate-[searchbar_1s_ease-in-out_infinite]" />
                </div>
              )}
              <div className="flex-1 overflow-y-auto p-4">
                <TrackList
                  tracks={results}
                  onPlayAll={playAll}
                  empty={searchError ? `Search failed: ${searchError}` : searching ? "Searching..." : "No results. Try a search."}
                />
              </div>
            </>
          )}
          {view === "library" && (
            <TrackList
              tracks={library}
              onPlayAll={playAll}
              empty="No local tracks. Add a directory in Settings and scan."
            />
          )}
          {view === "history" && <HistoryView />}
          {view === "settings" && <SettingsView onLibraryChanged={() => api.getLibrary().then(setLibrary).catch(() => {})} />}
          {view === "playlists" &&
            (playlistId ? (
              <PlaylistDetailView playlistId={playlistId} onBack={() => setPlaylistId(null)} />
            ) : (
              <PlaylistsView onOpen={(id) => setPlaylistId(id)} />
            ))}
        </div>
      </main>

      {queueOpen && (
        <aside className="w-72 flex-shrink-0 bg-gray-900 border-l border-gray-800 flex flex-col overflow-hidden">
          <div className="flex items-center justify-between px-3 py-2 border-b border-gray-800">
            <span className="text-sm font-medium">Queue</span>
            <button onClick={() => setQueueOpen(false)} className="p-1 hover:bg-gray-800 rounded">
              <X size={14} />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto px-2 py-1 space-y-0.5">
            {player.queue.map((t, i) => (
              <div
                key={`${t.id}-${i}`}
                onClick={() => player.playTrack(t, player.queue)}
                className={`flex items-center gap-2 px-2 py-1 rounded cursor-pointer text-xs ${
                  player.active?.id === t.id ? "bg-emerald-900/40 text-emerald-300" : "hover:bg-gray-800"
                }`}
              >
                <span className="truncate flex-1">
                  {t.title} <span className="text-gray-500">- {t.artists.join(", ")}</span>
                </span>
                <span className="text-gray-500 text-[10px]">{formatDuration(t.duration_ms)}</span>
              </div>
            ))}
          </div>
        </aside>
      )}
      </div>

      <PlayerBar onToggleQueue={() => setQueueOpen(!queueOpen)} />
    </div>
  );
}

function NavBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 w-full px-3 py-1.5 rounded-md text-sm transition-colors ${
        active
          ? "bg-gray-800 text-emerald-400 font-medium"
          : "text-gray-400 hover:bg-gray-800 hover:text-gray-100"
      }`}
    >
      {children}
    </button>
  );
}

function HomeView({
  library,
  onPlayAll,
}: {
  library: Track[];
  onPlayAll: (tracks: Track[], idx: number) => void;
}) {
  return (
    <div className="space-y-6">
      <section>
        <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wide mb-3">
          Your Library ({library.length} tracks)
        </h2>
        {library.length > 0 ? (
          <TrackList tracks={library.slice(0, 20)} onPlayAll={onPlayAll} />
        ) : (
          <p className="text-sm text-gray-500">Add your music directory in Settings to get started.</p>
        )}
      </section>
    </div>
  );
}

function HistoryView() {
  const [tracks, setTracks] = useState<Track[]>([]);
  const player = usePlayer();

  useEffect(() => {
    api.getHistory(100).then(setTracks).catch(() => {});
  }, []);

  return (
    <TrackList
      tracks={tracks}
      onPlayAll={(t, i) => player.playCollection(t, i)}
      empty="No play history yet."
    />
  );
}

function PlaylistsView({ onOpen }: { onOpen: (id: string) => void }) {
  const [playlists, setPlaylists] = useState<Playlist[]>([]);

  const refresh = () => {
    api.getPlaylists().then(setPlaylists).catch(() => {});
  };

  useEffect(refresh, []);

  const create = async () => {
    const name = prompt("Playlist name:");
    if (!name?.trim()) return;
    try {
      await api.createPlaylist(name.trim());
      refresh();
    } catch {}
  };

  const remove = async (id: string) => {
    if (!confirm("Delete this playlist?")) return;
    try {
      await api.deletePlaylist(id);
      refresh();
    } catch {}
  };

  return (
    <div className="max-w-2xl">
      <button
        onClick={create}
        className="mb-3 bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 rounded-md text-sm font-medium inline-flex items-center gap-1.5"
      >
        <FolderPlus size={14} /> New Playlist
      </button>
      {playlists.length === 0 ? (
        <p className="text-sm text-gray-500">No playlists yet. Create one to get started.</p>
      ) : (
        <div className="space-y-0.5">
          {playlists.map((p) => (
            <div
              key={p.id}
              onClick={() => onOpen(p.id)}
              className="flex items-center gap-3 px-3 py-2 rounded-md cursor-pointer hover:bg-gray-800/60 group"
            >
              <div className="w-9 h-9 rounded bg-gray-800 flex items-center justify-center flex-shrink-0 text-gray-500">
                <ListMusic size={14} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm truncate">{p.name}</div>
                <div className="text-xs text-gray-500">{p.track_ids.length} tracks</div>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  void remove(p.id);
                }}
                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-700 rounded text-gray-500 hover:text-red-400"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function PlaylistDetailView({ playlistId, onBack }: { playlistId: string; onBack: () => void }) {
  const [playlist, setPlaylist] = useState<Playlist | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const player = usePlayer();

  const refresh = () => {
    api.getPlaylist(playlistId).then(setPlaylist).catch(() => {});
    api.getPlaylistTracks(playlistId).then(setTracks).catch(() => {});
  };

  useEffect(refresh, [playlistId]);

  return (
    <div>
      <button
        onClick={onBack}
        className="mb-2 inline-flex items-center gap-1 text-sm text-gray-400 hover:text-gray-100"
      >
        <ArrowLeft size={14} /> All playlists
      </button>
      <h2 className="text-lg font-medium mb-3">{playlist?.name ?? "Playlist"}</h2>
      {tracks.length > 0 ? (
        <>
          <button
            onClick={() => player.playCollection(tracks, 0)}
            className="mb-3 bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 rounded-md text-sm font-medium inline-flex items-center gap-1.5"
          >
            <Play size={14} fill="currentColor" /> Play All
          </button>
          <div className="space-y-0.5">
            {tracks.map((t) => (
              <PlaylistTrackRow
                key={t.id}
                track={t}
                tracks={tracks}
                onRemove={async () => {
                  try {
                    await api.removeFromPlaylist(playlistId, t.id);
                    refresh();
                  } catch {}
                }}
              />
            ))}
          </div>
        </>
      ) : (
        <p className="text-sm text-gray-500">Empty playlist. Add tracks via the + button on any song.</p>
      )}
    </div>
  );
}

function PlaylistTrackRow({
  track,
  tracks,
  onRemove,
}: {
  track: Track;
  tracks: Track[];
  onRemove: () => void;
}) {
  const player = usePlayer();
  return (
    <div
      onClick={() => player.playTrack(track, tracks)}
      className="flex items-center gap-3 px-3 py-2 rounded-md cursor-pointer group hover:bg-gray-800/60"
    >
      <Music size={14} className="text-gray-500 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm truncate">{track.title}</div>
        <div className="text-xs text-gray-500 truncate">
          {track.artists.join(", ")}
          {track.album ? ` · ${track.album}` : ""}
        </div>
      </div>
      <span className="text-xs text-gray-500">{formatDuration(track.duration_ms)}</span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        className="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-700 rounded text-gray-500 hover:text-red-400"
      >
        <Trash2 size={14} />
      </button>
    </div>
  );
}

function TrackList({
  tracks,
  onPlayAll,
  empty = "No tracks.",
}: {
  tracks: Track[];
  onPlayAll: (tracks: Track[], idx: number) => void;
  empty?: string;
}) {
  const [visible, setVisible] = useState(100);
  useEffect(() => setVisible(100), [tracks]);
  if (tracks.length === 0) {
    return <p className="text-sm text-gray-500">{empty}</p>;
  }
  const shown = tracks.slice(0, visible);
  return (
    <div>
      <button
        onClick={() => onPlayAll(tracks, 0)}
        className="mb-3 bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 rounded-md text-sm font-medium inline-flex items-center gap-1.5"
      >
        <Play size={14} fill="currentColor" /> Play All
      </button>
      <div className="space-y-0.5">
        {shown.map((t, i) => (
          <TrackRow key={`${t.id}-${i}`} track={t} tracks={tracks} />
        ))}
      </div>
      {visible < tracks.length && (
        <button
          onClick={() => setVisible(visible + 200)}
          className="mt-3 text-sm text-emerald-400 hover:text-emerald-300"
        >
          Show more ({tracks.length - visible} remaining)
        </button>
      )}
    </div>
  );
}

function TrackRow({
  track,
  tracks,
}: {
  track: Track;
  tracks: Track[];
}) {
  const player = usePlayer();
  const isActive = player.active?.id === track.id;
  const [plMenu, setPlMenu] = useState(false);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);

  const togglePlMenu = () => {
    if (!plMenu) api.getPlaylists().then(setPlaylists).catch(() => {});
    setPlMenu(!plMenu);
  };

  const sourceBadge: Record<TrackSource, string> = {
    local: "bg-gray-700 text-gray-300",
    spotify: "bg-green-800 text-green-300",
    youtube: "bg-red-800 text-red-300",
    deezer: "bg-blue-800 text-blue-300",
  };

  return (
    <div
      onClick={() => player.playTrack(track, tracks)}
      className={`flex items-center gap-3 px-3 py-2 rounded-md cursor-pointer group ${
        isActive ? "bg-emerald-900/30" : "hover:bg-gray-800/60"
      }`}
    >
      {track.artwork ? (
        <img src={track.artwork} className="w-9 h-9 rounded object-cover flex-shrink-0" alt="" />
      ) : (
        <div className="w-9 h-9 rounded bg-gray-800 flex items-center justify-center flex-shrink-0 text-gray-500">
          <Music size={14} />
        </div>
      )}
      <div className="flex-1 min-w-0">
        <div className={`text-sm truncate ${isActive ? "text-emerald-300" : "text-gray-100"}`}>
          {track.title}
        </div>
        <div className="text-xs text-gray-500 truncate">
          {track.artists.join(", ")}
          {track.album ? ` · ${track.album}` : ""}
        </div>
      </div>
      <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${sourceBadge[track.source]}`}>
        {track.source}
      </span>
      <span className="text-xs text-gray-500 w-10 text-right">{formatDuration(track.duration_ms)}</span>
      <div className="relative flex-shrink-0 group/queue">
        <button
          onClick={(e) => {
            e.stopPropagation();
            player.addToQueue([track]);
          }}
          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-700 rounded transition-opacity"
        >
          <Plus size={12} />
        </button>
        <span className="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-1.5 px-2 py-1 text-[10px] bg-gray-700 text-gray-100 rounded opacity-0 group-hover/queue:opacity-100 transition-opacity whitespace-nowrap z-30">
          Add to queue
        </span>
      </div>
      <div className="relative flex-shrink-0 group/pl">
        <button
          onClick={(e) => {
            e.stopPropagation();
            togglePlMenu();
          }}
          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-700 rounded transition-opacity text-gray-400"
        >
          <ListPlus size={12} />
        </button>
        <span className="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-1.5 px-2 py-1 text-[10px] bg-gray-700 text-gray-100 rounded opacity-0 group-hover/pl:opacity-100 transition-opacity whitespace-nowrap z-30">
          Add to playlist
        </span>
        {plMenu && (
          <div
            onClick={(e) => e.stopPropagation()}
            className="absolute right-0 mt-1 w-44 bg-gray-800 border border-gray-700 rounded-md shadow-lg z-20 py-1"
          >
            {playlists.length === 0 ? (
              <div className="px-3 py-1.5 text-xs text-gray-500">No playlists</div>
            ) : (
              playlists.map((p) => (
                <button
                  key={p.id}
                  onClick={() => {
                    api.addToPlaylist(p.id, [track]).catch(() => {});
                    setPlMenu(false);
                  }}
                  className="w-full text-left px-3 py-1.5 text-xs text-gray-200 hover:bg-gray-700 truncate"
                >
                  {p.name}
                </button>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function PlayerBar({ onToggleQueue }: { onToggleQueue: () => void }) {
  const player = usePlayer();
  const pct = player.duration > 0 ? (player.currentTime / player.duration) * 100 : 0;

  return (
    <div className="h-20 flex-shrink-0 bg-gray-900 border-t border-gray-800 flex items-center px-4 gap-4">
      <div className="flex items-center gap-3 w-64 min-w-0">
        {player.active?.artwork ? (
          <img
            src={player.active.artwork}
            className="w-12 h-12 rounded object-cover flex-shrink-0"
            alt=""
          />
        ) : (
          <div className="w-12 h-12 rounded bg-gray-800 flex items-center justify-center text-gray-500 flex-shrink-0">
            <Music size={18} />
          </div>
        )}
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{player.active?.title || "Nothing playing"}</div>
          <div className="text-xs text-gray-500 truncate">{player.active?.artists?.join(", ") || ""}</div>
        </div>
      </div>

      <div className="flex-1 flex flex-col items-center gap-1 max-w-2xl">
        <div className="flex items-center gap-3">
          <button
            onClick={() => player.setShuffle(!player.shuffle)}
            className={`p-1.5 rounded ${player.shuffle ? "text-emerald-400" : "text-gray-500 hover:text-gray-300"}`}
          >
            <Shuffle size={16} />
          </button>
          <button onClick={() => void player.prev()} className="p-1.5 text-gray-400 hover:text-white">
            <SkipBack size={18} />
          </button>
          <button
            onClick={() => player.toggle()}
            className="w-9 h-9 bg-white text-gray-900 rounded-full flex items-center justify-center hover:scale-105 transition-transform"
          >
            {player.playing ? <Pause size={18} fill="currentColor" /> : <Play size={18} fill="currentColor" className="ml-0.5" />}
          </button>
          <button onClick={() => void player.next()} className="p-1.5 text-gray-400 hover:text-white">
            <SkipForward size={18} />
          </button>
          <button
            onClick={() => player.setRepeat(player.repeat === "off" ? "all" : player.repeat === "all" ? "one" : "off")}
            className={`p-1.5 rounded ${player.repeat !== "off" ? "text-emerald-400" : "text-gray-500 hover:text-gray-300"}`}
          >
            <Repeat size={16} />
            {player.repeat === "one" && (
              <span className="absolute text-[8px] font-bold">1</span>
            )}
          </button>
        </div>
        <div className="flex items-center gap-2 w-full">
          <span className="text-[10px] text-gray-500 w-8 text-right">{formatDuration(player.currentTime * 1000)}</span>
          <div className="flex-1 h-1.5 bg-gray-700 rounded-full overflow-hidden relative">
            <div
              className="absolute inset-y-0 left-0 bg-emerald-500 rounded-full"
              style={{ width: `${pct}%` }}
            />
            <input
              type="range"
              min={0}
              max={Math.floor(player.duration)}
              value={Math.floor(player.currentTime)}
              onChange={(e) => player.seek(Number(e.target.value))}
              className="absolute inset-0 w-full opacity-0 cursor-pointer"
            />
          </div>
          <span className="text-[10px] text-gray-500 w-8">{formatDuration(player.duration * 1000)}</span>
        </div>
      </div>

      <div className="w-40 flex items-center gap-2">
        <Volume2 size={14} className="text-gray-500" />
        <input
          type="range"
          min={0}
          max={1}
          step={0.01}
          value={player.volume}
          onChange={(e) => player.setVolume(Number(e.target.value))}
          className="flex-1 accent-emerald-500 h-1"
        />
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={() => player.setRadio(!player.radio)}
          className={`text-xs px-2 py-1 rounded ${
            player.radio
              ? "bg-emerald-800 text-emerald-300"
              : "bg-gray-800 text-gray-500"
          }`}
        >
          Radio {player.radio ? "ON" : "OFF"}
        </button>
        <button onClick={onToggleQueue} className="text-xs px-2 py-1 rounded bg-gray-800 text-gray-400 hover:text-gray-200">
          Queue ({player.queue.length})
        </button>
      </div>
    </div>
  );
}

function SettingsView({ onLibraryChanged }: { onLibraryChanged: () => void }) {
  const [settings, setSettings] = useState<any>({
    library_dirs: [],
    piped_instances: [],
    auto_radio: true,
    crossfade_seconds: 0,
  });
  const [msg, setMsg] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanCount, setScanCount] = useState(0);
  const [scanFile, setScanFile] = useState("");

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => {});
    const un = listen<{ count: number; file: string }>("scan_progress", (e) => {
      setScanCount(e.payload.count);
      setScanFile(e.payload.file);
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  const save = async () => {
    try {
      await api.setSettings(settings);
      setMsg("Saved");
      setTimeout(() => setMsg(""), 2000);
    } catch {}
  };

  const saveDirs = async (dirs: string[]) => {
    const next = { ...settings, library_dirs: dirs };
    setSettings(next);
    try {
      await api.setSettings(next);
    } catch {}
  };

  const addDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && selected) {
      await saveDirs([...settings.library_dirs, selected]);
    }
  };

  const scanNow = async () => {
    try {
      setScanning(true);
      setScanCount(0);
      setScanFile("");
      const count = await api.scanLibrary(undefined);
      setMsg(`Scanned ${count} files`);
      onLibraryChanged();
      setTimeout(() => setMsg(""), 3000);
    } catch {
      setMsg("Scan failed");
    } finally {
      setScanning(false);
    }
  };

  return (
    <div className="max-w-xl space-y-6">
      <section>
        <h3 className="text-sm font-semibold text-gray-300 mb-2">Library Directories</h3>
        <div className="space-y-1">
          {settings.library_dirs.map((d: string, i: number) => (
            <div key={i} className="flex items-center gap-2 text-sm text-gray-400 bg-gray-800 rounded px-3 py-1.5">
              <span className="flex-1 truncate">{d}</span>
              <button
                onClick={() => {
                  const dirs = settings.library_dirs.filter((_: string, j: number) => j !== i);
                  void saveDirs(dirs);
                }}
                className="text-gray-500 hover:text-red-400 text-xs"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
        <div className="flex gap-2 mt-2">
          <button onClick={addDir} className="bg-gray-800 hover:bg-gray-700 px-3 py-1.5 rounded text-sm">Add Directory</button>
          <button
            onClick={() => void scanNow()}
            disabled={scanning}
            className="bg-emerald-700 hover:bg-emerald-600 disabled:opacity-50 px-3 py-1.5 rounded text-sm"
          >
            {scanning ? "Scanning..." : "Scan Now"}
          </button>
        </div>
        {scanning && (
          <div className="mt-2 text-xs text-gray-400">
            <span className="text-emerald-400">{scanCount} tracks</span> ·{" "}
            <span className="truncate inline-block max-w-xs align-bottom">{scanFile}</span>
          </div>
        )}
      </section>

      <section>
        <h3 className="text-sm font-semibold text-gray-300 mb-2">Piped Instances</h3>
        <textarea
          value={settings.piped_instances.join("\n")}
          onChange={(e) => setSettings({ ...settings, piped_instances: e.target.value.split("\n").filter(Boolean) })}
          className="w-full bg-gray-800 border border-gray-700 rounded p-2 text-sm h-24 font-mono"
          placeholder={"https://pipedapi.kavin.rocks\nhttps://pipedapi.adminforge.de"}
        />
      </section>

      <section>
        <h3 className="text-sm font-semibold text-gray-300 mb-2">Spotify (for metadata + recommendations)</h3>
        <input
          value={settings.spotify_client_id || ""}
          onChange={(e) => setSettings({ ...settings, spotify_client_id: e.target.value || undefined })}
          placeholder="Client ID"
          className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm mb-2"
        />
        <input
          value={settings.spotify_client_secret || ""}
          onChange={(e) => setSettings({ ...settings, spotify_client_secret: e.target.value || undefined })}
          placeholder="Client Secret"
          type="password"
          className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm"
        />
        <p className="text-xs text-gray-500 mt-1">
          Create an app at{" "}
          <a href="https://developer.spotify.com/dashboard" target="_blank" className="text-emerald-400 underline">
            developer.spotify.com
          </a>
          {" "}and set redirect URI to <code className="bg-gray-800 px-1 rounded">http://localhost:29171/callback</code>
        </p>
      </section>

      <section>
        <h3 className="text-sm font-semibold text-gray-300 mb-2">Last.fm API Key</h3>
        <input
          value={settings.lastfm_api_key || ""}
          onChange={(e) => setSettings({ ...settings, lastfm_api_key: e.target.value || undefined })}
          placeholder="API Key"
          className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm"
        />
      </section>

      <section>
        <label className="flex items-center gap-2 text-sm text-gray-300">
          <input
            type="checkbox"
            checked={settings.auto_radio}
            onChange={(e) => setSettings({ ...settings, auto_radio: e.target.checked })}
            className="accent-emerald-500"
          />
          Auto-continue with similar songs when queue ends (Radio Mode)
        </label>
      </section>

      <div className="flex gap-3 items-center">
        <button onClick={save} className="bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 rounded text-sm font-medium">Save</button>
        {msg && <span className="text-sm text-emerald-400">{msg}</span>}
      </div>
    </div>
  );
}
