# Photo Manager — Setup Guide

## Prerequisites (Windows — run from PowerShell)

```powershell
# 1. Install Rust
winget install Rustlang.Rustup
# Then restart PowerShell. Verify:
rustc --version

# 2. Install Node.js + pnpm
winget install OpenJS.NodeJS
npm install -g pnpm
# Verify:
pnpm --version

# 3. Install Visual Studio Build Tools (C++ workload required by Rust/Tauri)
winget install Microsoft.VisualStudio.2022.BuildTools
# In the installer, select "Desktop development with C++"

# 4. WebView2 runtime — usually pre-installed on Windows 10/11
# If missing: https://developer.microsoft.com/microsoft-edge/webview2/

# 5. Install NASM (optional — enables SIMD in libjpeg-turbo for faster thumbnail generation)
winget install nasm.nasm
# Restart PowerShell after so nasm.exe is on PATH.

# 6. Install cargo-audit for dependency security checks
cargo install cargo-audit
```

## Development

```powershell
cd D:\Projects\photo-manager

# Install JS dependencies (first time)
pnpm install

# Run security audits
pnpm audit --audit-level=high
cargo audit --manifest-path src-tauri/Cargo.toml

# Start dev server (hot reload)
pnpm tauri dev
```

## Production Build (.msi installer)

```powershell
cd D:\Projects\photo-manager
pnpm tauri build
# Output: src-tauri\target\release\bundle\msi\Photo Manager_0.1.0_x64_en-US.msi
```

## First Use

1. Launch the app
2. Click **Re-index Library** (top right)
3. Select your Google Photos folder: `D:\Photos\Google Photos`
4. Wait for indexing to complete (progress bar at top — ~15 min for 44K files)
5. Use the filter sidebar to search by people, type, size, or album
6. Switch to **Duplicates** tab to review and clean up duplicate photos

## Notes

- Database stored at: `%APPDATA%\photo-manager\library.db`
- Thumbnails stored at: `%APPDATA%\photo-manager\thumbs\`
- Deleting a photo moves it to the Windows Recycle Bin (safe, recoverable)
- HEIC thumbnails are not generated (requires libheif) — EXIF data is still indexed
