# Building from Source

## Prerequisites

| Tool | Notes |
|------|-------|
| [Rust](https://rustup.rs/) stable | `rustup update stable` |
| [Node.js](https://nodejs.org/) ≥ 18 | |
| [pnpm](https://pnpm.io/) ≥ 8 | `npm i -g pnpm` |
| [VS Build Tools 2019/2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/) | Select **Desktop development with C++** — required to compile libjpeg-turbo |
| [CMake](https://cmake.org/) ≥ 3.14 | Must be on `PATH`; included with VS Build Tools or install separately |
| [NASM](https://nasm.us/) (optional) | Enables SIMD in libjpeg-turbo for faster thumbnail generation |

## Steps

```powershell
git clone <repo-url>
cd photo-manager
pnpm install

# Dev server with hot reload
pnpm tauri dev

# Production build → src-tauri\target\release\bundle\msi\
pnpm tauri build
```
