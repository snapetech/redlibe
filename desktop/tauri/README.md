# Tauri Desktop Wrapper (Scaffold)

This is a scaffold for replacing `src/bin/redlib-desktop.rs` with a Tauri app while keeping the backend architecture unchanged.

The wrapper starts the `redlib` backend on `127.0.0.1`, waits for it to accept connections, and then opens a Tauri window pointed at the local HTTP endpoint.

## Status

- Scaffold only
- Not part of the root Cargo build
- Not part of CI artifacts yet

## Expected layout

- `desktop/tauri/src-tauri` contains the Rust/Tauri app
- backend binary `redlib` should be distributed alongside the Tauri app bundle
- CI now packages a scaffold artifact that places backend binaries under
  `src-tauri/binaries/<platform>/`
- CI now also includes a native Tauri bundle workflow scaffold (`.github/workflows/tauri-bundle.yml`)
  using Tauri sidecar naming: `src-tauri/binaries/redlib-<target-triple>[.exe]`

## Next integration steps

1. Install Tauri prerequisites for your platform.
2. Build and validate the wrapper locally.
3. Add Tauri bundling to release CI.
4. Decide whether to retire or keep `redlib-desktop`.

Note:

- Code can scaffold bundling, but signing/notarization still requires CI secrets/certificates.
- Configure those via the names documented in `docs/ci-signing-secrets.md`.
