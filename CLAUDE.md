# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```powershell
# Dev server (hot reload for frontend; Rust recompiles on change)
pnpm tauri dev

# Production build → src-tauri\target\release\bundle\msi\
pnpm tauri build

# Frontend only (no Tauri)
pnpm dev

# Type-check frontend
pnpm build   # runs tsc && vite build
```

There are no automated tests in this project.

## CI

Two GitHub Actions workflows build release artifacts on every push to `main`:

| Workflow | Runner | Output |
|---|---|---|
| `build-windows.yml` | `windows-latest` | `.msi` installer (x64) |
| `build-macos.yml` | `macos-latest` (ARM64) | `.dmg` (arm64 only — universal build dropped due to libheif cross-compilation limitations) |

**libheif on Windows CI**: `libheif-sys` requires vcpkg. The workflow installs `libheif:x64-windows-static-md` via the runner's pre-installed vcpkg (`C:\vcpkg`) and passes `VCPKG_ROOT` + `VCPKGRS_TRIPLET` to the Rust build. `C:\vcpkg\installed` is cached to avoid recompiling on every run.

**pnpm workspace**: `pnpm-workspace.yaml` must keep `packages: ['.']` — without it pnpm errors with `packages field missing or empty` during install.

## Architecture

This is a **Tauri v2** desktop app (Windows-first). The frontend is React + TypeScript + Tailwind, served by Vite. The backend is Rust.

### IPC boundary

All frontend→backend calls go through `invoke()` from `@tauri-apps/api/core`. Every backend command must be:
1. Declared as `#[tauri::command]` in `src-tauri/src/commands.rs`
2. Registered in the `invoke_handler!` macro in `src-tauri/src/lib.rs`
3. Permitted via `src-tauri/capabilities/main.json` (Tauri v2 requires explicit capability grants; missing entries cause silent failures)

Plugin commands (e.g., `dialog:allow-open`, `shell:allow-open`) also require entries in that capabilities file.

### Rust backend (`src-tauri/src/`)

| File | Role |
|---|---|
| `lib.rs` | Entry point, plugin registration, command handler list |
| `commands.rs` | All `#[tauri::command]` functions + `AppState` |
| `indexer.rs` | File walk, sidecar/EXIF parsing, thumbnail generation, duplicate detection |
| `db.rs` | SQLite open/migrate, `upsert_album`, `upsert_person`, `clear_duplicate_groups` |
| `models.rs` | Serde structs shared between DB queries and the IPC layer |

**Indexing flow**: `scan_library` command validates state, then spawns a thread that calls `indexer::run_scan`. Progress is communicated via `AppState.progress` (atomic counters + a mutex-guarded phase string), polled from the frontend every 600 ms via `get_index_progress`.

**Database**: SQLite at `%APPDATA%\photo-manager\library.db`, WAL mode. Schema is applied idempotently in `db::migrate` on every `db::open()` call — no migration versioning. Each command opens its own connection (no connection pool).

**Thumbnails**: Written to `%APPDATA%\photo-manager\thumbs\<md5>.jpg`, 256×256. HEIC/HEIF files are decoded via `libheif-rs` (`indexer::heic_to_dynamic_image`) before resizing. Video thumbnails are skipped. Thumbnails are served to the frontend via Tauri's asset protocol (`convertFileSrc`).

**Duplicate detection** runs at the end of every scan. Phase 1: exact MD5 hash matches. Phase 2: EXIF fingerprint (same timestamp + camera make + dimensions).

### Frontend (`src/`)

| File/dir | Role |
|---|---|
| `App.tsx` | Root layout, view routing (`gallery`/`duplicates`), shared state |
| `hooks/useTauriCommand.ts` | `useTauriCommand<T>` (one-shot invoke) and `usePolling<T>` (interval polling) |
| `types.ts` | TypeScript interfaces mirroring `models.rs` structs |
| `components/Gallery.tsx` | Virtualized grid (`react-window` `FixedSizeGrid`), infinite scroll via sentinel row |
| `components/StatsBar.tsx` | Header stats + Re-index Library button (opens native folder picker) |
| `components/FilterSidebar.tsx` | Filter controls that feed into `SearchFilters` |
| `components/IndexProgress.tsx` | Progress bar shown during active scan |
| `components/PhotoDetail.tsx` | Right-panel detail view for a selected photo |
| `components/DuplicatesView.tsx` | Duplicate groups UI with per-item delete |

**State flow**: `App.tsx` owns `filters` (passed down to `FilterSidebar` and `Gallery`) and `selectedId`. Gallery manages its own paginated item list; filter changes reset and reload from offset 0. `stats` is fetched once and refetched after indexing or deletion. `isIndexing` is derived from the polling hook.

## Key constraints

- **Capabilities file is required**: `src-tauri/capabilities/main.json` must list any plugin permission used. Omitting an entry silently blocks the call on the frontend (the promise resolves `null` for dialogs, or throws for commands).
- **`indexed` flag**: `LibraryStats.indexed` is `total > 0` — the StatsBar only renders (and shows the Re-index button) when `stats` is non-null, which requires at least one successful `get_stats` call.
- **turbojpeg for JPEG thumbnails**: `indexer.rs` uses `turbojpeg` (which builds libjpeg-turbo from bundled C source via cmake) for DCT-scaled JPEG decoding. For a large image it decodes at 1/8 resolution before resizing to 256×256, making thumbnail generation ~10× faster than full decode. cmake is required at build time (provided by VS Build Tools). NASM is optional — without it the build still succeeds but SIMD is disabled.
- **No connection pool**: Every Rust command that needs DB access calls `db::open()` independently. Schema migration runs on each open, which is safe but adds a small overhead.
- **People filter is AND logic**: Filtering by multiple people returns only media tagged with all of them (see `query_media` in `commands.rs`).
