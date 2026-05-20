# Changelog

All notable changes to the Vòng Kim Cô desktop app will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-05-20

### Added
- Complete CI/CD pipeline with GitHub Actions (desktop-ci.yml + desktop-release.yml)
- Version verification step in release workflow (ensures consistency across package.json, Cargo.toml, tauri.conf.json)
- Periodic background update check (every 4 hours) with auto-download option
- Dynamic version display in sidebar (reads from Tauri API instead of hardcoded value)
- macOS notarization and Windows code signing support in CI workflow
- Version bump script (`desktop/scripts/bump-version.cjs`) for synchronized versioning
- CHANGELOG.md for automated release notes extraction
- Prerelease detection (alpha/beta/rc tags automatically marked as prerelease)

### Changed
- Enhanced release workflow with concurrency control and workflow_dispatch support
- Updater config (pubkey + endpoint) injected at CI build time for security
- Tauri signing keypair regenerated and stored as GitHub Secrets

### Fixed
- Hardcoded version string in App.svelte sidebar replaced with dynamic API call


## [0.2.0] - 2026-01-15

### Added
- Desktop OTA auto-update via Tauri Updater with Ed25519 signature verification
- Periodic background update check (every 4 hours)
- Update banner with download progress, install, and error states
- GitHub Actions CI workflow for desktop (lint frontend + Rust + build)
- GitHub Actions release workflow with 4-platform matrix (Linux, macOS Intel, macOS ARM, Windows)
- Version sync script (`desktop/scripts/bump-version.cjs`)
- macOS notarization and Windows code signing support in CI
- Automatic `latest.json` manifest generation for Tauri Updater

### Changed
- Version display in sidebar now reads from Tauri API instead of hardcoded value
- Updater config (pubkey + endpoint) injected at CI build time

## [0.1.0] - 2025-12-01

### Added
- Initial desktop app release
- Google OAuth login flow via system browser
- Session start/stop with global hotkeys
- Periodic screenshot capture with JPEG compression
- Idle/active monitoring via keyboard + mouse detection
- Running application snapshot
- Offline-first SQLite storage with background sync
- Auto-start on login option
- Dark mode UI with sidebar navigation
