# Musicadena

[![Release](https://img.shields.io/github/v/release/coldobserver/musicadena?label=latest%20release)](https://github.com/coldobserver/musicadena/releases/latest)

A cross-platform music player built with Tauri 2, React, and TypeScript. Search and stream music from multiple sources, manage your local library, and keep play history — all in one desktop app.

> Built with assistance from [opencode](https://opencode.ai).

## Features

- **Search across sources** — Spotify, YouTube, and your local library in one query
- **Streaming playback** — resolves and streams audio via InnerTube, Piped, and optionally yt-dlp (no accounts needed)
- **Local library** — scan directories and play your own files (MP3, FLAC, M4A, OGG, WAV, OPUS, AAC, WMA)
- **Playlists** — create, add to, and manage playlists
- **Play history** — tracks recently played
- **Radio mode** — auto-suggests related tracks when the queue ends
- **Queue management** — shuffle, repeat, add to queue, next/previous

## Tech Stack

- **Tauri 2** — Rust backend (SQLite via rusqlite, reqwest, lofty for tag reading)
- **React 19 + TypeScript** — Vite frontend
- **Tailwind CSS 4** — UI styling
- **Zustand** — player state management

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- OS-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

**Optional:** [yt-dlp](https://github.com/yt-dlp/yt-dlp) — used as an extra stream resolver when available. Without it, InnerTube/Piped handle resolution.

## Development

```bash
npm install
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

Output installers land in `src-tauri/target/release/bundle/`:
- **Windows**: `.msi` + `.exe` (NSIS)
- **Linux**: `.deb`, `.rpm`, `.AppImage` (built on Linux)

### GitHub Releases

Push a version tag to trigger automated builds via `.github/workflows/release.yml`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

A GitHub Release is created with installers for Windows and Linux. Download them from the [Releases page](https://github.com/coldobserver/musicadena/releases).

## License

Private project.
