# Changelog

All notable changes to the VongKimCo desktop app will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-20

### Added
- Cross-platform desktop app (Windows, macOS Apple Silicon, Linux) built with Tauri 2 + Svelte + Rust
- Google OAuth login flow via system browser
- Session start/stop with global hotkeys
- Periodic screenshot capture with JPEG compression
- Idle/active monitoring via keyboard + mouse detection
- Running application snapshot
- Offline-first SQLite storage with background sync to backend
- Auto-start on login option
- Dark mode UI with sidebar navigation
- Desktop OTA auto-update via Tauri Updater with Ed25519 signature verification
- Periodic background update check (every 4 hours)
- Update banner with download progress, install, and error states
- GitHub Actions CI workflow (lint frontend + Rust, build Linux on main push)
- GitHub Actions release workflow with 3-platform matrix (Linux, macOS ARM, Windows)
- Version sync script (`desktop/scripts/bump-version.cjs`)
- Automatic `latest.json` manifest generation for Tauri Updater
- Platform-aware download buttons on home page (detects OS, suggests correct installer)
- Backend release proxy (`/api/v1/desktop/latest`) with 5-minute cache
- Docker image pushed to ghcr.io by GitHub Actions (Coolify pulls pre-built image)
- Backend CI: lint + clippy + Docker push to GHCR
- App identifier: `com.hoctuthien.vongkimco`

### Changed
- productName set to `VongKimCo` (ASCII, no spaces/diacritics) for clean build filenames
- Window title and notifications use Vietnamese display name `Vòng Kim Cô`
- Removed MSI installer target — only NSIS setup.exe for Windows (avoids user confusion)
- Build Linux job only runs on main push (not PRs) to save CI minutes
