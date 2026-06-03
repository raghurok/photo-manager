# Photo Manager

A desktop app for browsing and cleaning up your **Google Photos Takeout** archive — offline, with no cloud account required.

When you export your Google Photos library via Takeout, you get thousands of files scattered across dated folders with `.json` sidecars holding the real metadata (albums, people tags, GPS, timestamps). This app reads all of that and gives you a fast, searchable gallery.

> **Windows only** — macOS support is planned.

## Download

Grab the latest installer from the [Releases](../../releases/latest) page:

- **Windows** — `photo-manager_x.x.x_x64_en-US.msi`

## Features

- Browse photos and videos in a fast scrollable grid
- Filter by person, album, date range, media type, or file size
- See full metadata — GPS location, camera, EXIF, album, and tagged people
- Detect and clean up duplicate files (exact matches and near-duplicates by EXIF fingerprint)
- Everything stays local — no accounts, no uploads

## Getting Started

1. Install the app from the [Releases](../../releases/latest) page
2. Open the app and click **Re-index Library**
3. Select the root of your extracted Takeout folder (e.g. `Takeout/Google Photos`)
4. Wait for indexing to finish — progress is shown in the header
5. Browse, filter, and clean up

Re-indexing is safe to run again at any time; it won't delete or move your original files.

## Building from Source

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
