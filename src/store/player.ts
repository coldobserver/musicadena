import { create } from "zustand";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Track } from "../model";
import { api } from "../lib/ipc";

let audio: HTMLAudioElement | null = null;
if (typeof window !== "undefined") {
  audio = new Audio();
  audio.preload = "auto";
}

export type RepeatMode = "off" | "all" | "one";

interface PlayerStore {
  queue: Track[];
  playOrder: number[];
  orderPos: number;
  active: Track | null;
  playing: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
  radio: boolean;
  radioLoading: boolean;
  history: Track[];

  playCollection: (tracks: Track[], startIndex?: number) => Promise<void>;
  playTrack: (track: Track, context?: Track[]) => Promise<void>;
  next: () => Promise<void>;
  prev: () => Promise<void>;
  toggle: () => void;
  seek: (seconds: number) => void;
  setVolume: (v: number) => void;
  setShuffle: (v: boolean) => void;
  setRepeat: (m: RepeatMode) => void;
  setRadio: (v: boolean) => void;
  addToQueue: (tracks: Track[]) => void;
  removeFromQueue: (indexInOrder: number) => void;
  clearQueue: () => void;
}

function shuffleOrder(n: number): number[] {
  const order = Array.from({ length: n }, (_, i) => i);
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  return order;
}

function trackKey(t: Track): string {
  return t.source === "local" && t.path ? t.path : t.id;
}

async function resolveUrl(track: Track): Promise<string> {
  if (track.source === "local" && track.path) {
    return convertFileSrc(track.path);
  }
  if (track.stream_url) return track.stream_url;
  const resolved = await api.resolveStream(track);
  track.stream_url = resolved.url;
  return resolved.url;
}

function initListeners(next: () => Promise<void>) {
  if (!audio) return;
  audio.addEventListener("timeupdate", () => {
    usePlayer.setState({ currentTime: audio!.currentTime });
  });
  audio.addEventListener("durationchange", () => {
    const d = audio!.duration;
    usePlayer.setState({ duration: Number.isFinite(d) ? d : usePlayer.getState().duration });
  });
  audio.addEventListener("ended", () => void next());
  audio.addEventListener("error", () => void next());
}

export const usePlayer = create<PlayerStore>((set, get) => {
  const setPos = (orderPos: number) => {
    const { queue, playOrder } = get();
    const track = queue[playOrder[orderPos]] ?? null;
    set({ orderPos, active: track });
  };

  const loadAndPlay = async (track: Track) => {
    if (!audio) return;
    const url = await resolveUrl(track);
    audio.src = url;
    audio.volume = get().volume;
    await audio.play();
    const known = track.duration_ms ? track.duration_ms / 1000 : 0;
    set({ playing: true, currentTime: 0, duration: known });
    if (track.id) {
      api.recordPlayback(track).catch(() => {});
    }
  };

  const next = async () => {
    const { queue, playOrder, orderPos, repeat, radio, shuffle, active } = get();
    if (repeat === "one" && active) {
      audio?.play();
      return;
    }
    if (shuffle) {
      const pool = queue.map((_, i) => i);
      const rest = pool.filter((i) => i !== playOrder[orderPos]);
      const pick = rest.length > 0 ? rest[Math.floor(Math.random() * rest.length)] : pool[0];
      const pos = playOrder.indexOf(pick);
      setPos(pos);
      await loadAndPlay(queue[pick]);
      return;
    }
    if (orderPos + 1 < playOrder.length) {
      const pos = orderPos + 1;
      setPos(pos);
      await loadAndPlay(queue[playOrder[pos]]);
      return;
    }
    if (radio) {
      set({ radioLoading: true });
      try {
        const seed = active ?? queue[playOrder[orderPos]];
        const suggestions = await api.radioSuggestions(seed, 12);
        const fresh = suggestions.filter(
          (t) => !get().queue.some((q) => trackKey(q) === trackKey(t)) &&
            !get().history.some((h) => trackKey(h) === trackKey(t)),
        );
        if (fresh.length === 0) {
          set({ playing: false, radioLoading: false });
          return;
        }
        const newQueue = [...get().queue, ...fresh];
        const newOrder = Array.from({ length: newQueue.length }, (_, i) => i);
        set({ queue: newQueue, playOrder: newOrder, orderPos: newQueue.length - fresh.length, radioLoading: false });
        await loadAndPlay(fresh[0]);
        return;
      } catch {
        set({ playing: false, radioLoading: false });
        return;
      }
    }
    if (repeat === "all") {
      setPos(0);
      await loadAndPlay(queue[playOrder[0]]);
      return;
    }
    set({ playing: false, orderPos: playOrder.length - 1 });
  };

  const prev = async () => {
    const { playOrder, orderPos, queue, history, active } = get();
    if (audio && audio.currentTime > 3) {
      audio.currentTime = 0;
      return;
    }
    if (orderPos > 0) {
      const pos = orderPos - 1;
      setPos(pos);
      await loadAndPlay(queue[playOrder[pos]]);
    } else {
      await loadAndPlay(queue[playOrder[0]]);
    }
    if (active) {
      const seen = history.some((h) => trackKey(h) === trackKey(active));
      set({ history: seen ? history : [...history.slice(-49), active] });
    }
  };

  initListeners(next);

  return {
    queue: [],
    playOrder: [],
    orderPos: -1,
    active: null,
    playing: false,
    currentTime: 0,
    duration: 0,
    volume: 0.8,
    shuffle: false,
    repeat: "off",
    radio: true,
    radioLoading: false,
    history: [],

    playCollection: async (tracks, startIndex = 0) => {
      if (tracks.length === 0) return;
      const sequential = Array.from({ length: tracks.length }, (_, i) => i);
      const order = get().shuffle ? shuffleOrder(tracks.length) : sequential;
      const startPos = Math.max(0, order.indexOf(startIndex));
      const first = tracks[order[startPos]];
      set({
        queue: tracks,
        playOrder: order,
        orderPos: startPos,
        active: first,
        history: [],
      });
      await loadAndPlay(first);
    },

    playTrack: async (track, context) => {
      if (context && context.length > 0) {
        await get().playCollection(context, context.findIndex((t) => t.id === track.id));
        return;
      }
      const exists = get().queue.some((q) => trackKey(q) === trackKey(track));
      if (!exists) {
        set({ queue: [...get().queue, track], playOrder: [...get().playOrder, get().queue.length], orderPos: get().queue.length, active: track });
      } else {
        const idx = get().queue.findIndex((q) => trackKey(q) === trackKey(track));
        const pos = get().playOrder.indexOf(idx);
        setPos(pos);
      }
      await loadAndPlay(track);
    },

    next,
    prev,

    toggle: () => {
      if (!audio || !get().active) return;
      if (audio.paused) void audio.play();
      else audio.pause();
      set({ playing: !audio.paused });
    },

    seek: (seconds) => {
      if (audio) audio.currentTime = seconds;
    },

    setVolume: (v) => {
      if (audio) audio.volume = v;
      set({ volume: v });
    },

    setShuffle: (v) => {
      const { queue, orderPos, active } = get();
      if (v) {
        const rest = Array.from({ length: queue.length }, (_, i) => i).filter(
          (i) => i !== (active ? queue.findIndex((q) => trackKey(q) === trackKey(active)) : orderPos),
        );
        const start = active ? queue.findIndex((q) => trackKey(q) === trackKey(active)) : orderPos;
        const order = [start, ...shuffleOrder(rest.length).map((r) => rest[r])].filter((i) => i >= 0);
        set({ shuffle: true, playOrder: order, orderPos: 0 });
      } else {
        set({ shuffle: false, playOrder: Array.from({ length: queue.length }, (_, i) => i), orderPos: active ? queue.findIndex((q) => trackKey(q) === trackKey(active)) : orderPos });
      }
    },

    setRepeat: (m) => set({ repeat: m }),
    setRadio: (v) => set({ radio: v }),

    addToQueue: (tracks) => {
      const { queue, playOrder } = get();
      const start = queue.length;
      set({ queue: [...queue, ...tracks], playOrder: [...playOrder, ...tracks.map((_, i) => start + i)] });
    },

    removeFromQueue: (indexInOrder) => {
      const { queue, playOrder, orderPos, active } = get();
      const queueIdx = playOrder[indexInOrder];
      if (queueIdx === undefined) return;
      const newQueue = queue.filter((_, i) => i !== queueIdx);
      const activeIdx = active
        ? newQueue.findIndex((q) => trackKey(q) === trackKey(active))
        : -1;
      set({
        queue: newQueue,
        playOrder: Array.from({ length: newQueue.length }, (_, i) => i),
        orderPos:
          activeIdx >= 0
            ? activeIdx
            : Math.min(orderPos, Math.max(0, newQueue.length - 1)),
      });
    },

    clearQueue: () => {
      if (audio) audio.pause();
      set({ queue: [], playOrder: [], orderPos: -1, active: null, playing: false, currentTime: 0, duration: 0 });
    },
  };
});

if (audio) audio.volume = usePlayer.getState().volume;
