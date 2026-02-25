# Redlib Desktop Distribution Plan (Implemented + Remaining)

This repo now ships two desktop-oriented entry points:

- `redlib` (backend server binary)
- `redlib-desktop` (launcher binary that starts `redlib` on `127.0.0.1` and opens the browser)

## Current desktop behavior

`redlib-desktop` will:

- create a per-user config directory:
  - Linux: `~/.config/redlibe`
  - macOS: `~/Library/Application Support/redlibe`
  - Windows: `%APPDATA%\\redlibe`
- create `session_secret` on first run and reuse it
- create a starter `redlib.toml` if missing
- choose a local port starting at `18080`
- start the backend with `--address 127.0.0.1 --port <port>`
- open the system browser to the local URL

## Local browser import support

Implemented in `/login`:

- Firefox local import (Linux/Windows/macOS profile discovery)
- LibreWolf local import (Linux/Windows/macOS profile discovery)
- Chrome/Edge local import with:
  - profile discovery
  - plaintext cookie support
  - encrypted-cookie decryption attempts:
    - modern AES-GCM + `Local State` key (where available)
    - legacy AES-CBC + Linux Secret Service / `peanuts` fallback
    - legacy AES-CBC + macOS Keychain password lookup (`security`)
    - Windows DPAPI decrypt path for `Local State` key via PowerShell

Notes:

- Chromium encryption behavior varies by browser version and OS setup.
- Firefox-family local import is currently the most reliable path.

## Release artifacts (current CI scaffold)

GitHub Actions workflow: `.github/workflows/release-build.yml`

Artifacts per OS currently package:

- `redlib`
- `redlib-desktop`

Formats:

- Linux/macOS: `.tar.gz`
- Windows: `.zip`

## cargo-dist scaffold

`dist-workspace.toml` is included as a starting point for adopting `cargo-dist`.

It is not fully wired to produce desktop installers for the launcher/Tauri wrapper yet.

## Tauri wrapper scaffold

A non-default Tauri wrapper scaffold lives in `desktop/tauri/`.

Purpose:

- launch backend locally
- present a desktop app shell
- keep local HTTP architecture unchanged

Status:

- scaffolded only (not integrated into CI/release builds)
- current production desktop path remains `redlib-desktop`

Update:

- release CI now also emits a `redlib-tauri-scaffold-<platform>` artifact that includes
  the Tauri scaffold plus a platform-built `redlib` backend under `src-tauri/binaries/<platform>/`
- native Tauri bundle CI scaffold is now included at `.github/workflows/tauri-bundle.yml`
  and prepares a Tauri sidecar binary named `src-tauri/binaries/redlib-<target-triple>[.exe]`

Signing/notarization:

- still requires repository secrets and platform certificates (not configured by code alone)
- see `docs/ci-signing-secrets.md` for recommended secret names and setup checklists
