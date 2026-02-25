# CI Signing / Notarization Secrets (Tauri Desktop Builds)

This document defines the repository secrets/variables expected by `.github/workflows/tauri-bundle.yml`.

## Scope

- Windows code signing (optional)
- macOS signing + notarization (optional but recommended for distribution)
- Tauri updater signing key (optional, only if enabling updater)

The workflow can still build unsigned bundles if these are not configured.

## GitHub repository secrets (recommended names)

### macOS signing / notarization

- `APPLE_CERTIFICATE`
  - Base64-encoded `.p12` signing certificate file contents.
- `APPLE_CERTIFICATE_PASSWORD`
  - Password for the `.p12` certificate.
- `APPLE_SIGNING_IDENTITY`
  - Code signing identity string, e.g. `Developer ID Application: Example Corp (TEAMID)`.
- `APPLE_ID`
  - Apple ID email used for notarization.
- `APPLE_PASSWORD`
  - App-specific password for the Apple ID notarization account.
- `APPLE_TEAM_ID`
  - Apple Developer Team ID.

### Windows signing (optional)

Use one of these patterns depending on your signing approach:

- `WINDOWS_CERTIFICATE`
  - Base64-encoded `.pfx` certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`
  - Password for the `.pfx`.

If you use an external signing service (e.g. Azure Trusted Signing / EV token), replace the workflow step with that provider’s action and add provider-specific secrets instead.

### Tauri updater signing (optional)

Only required if you enable Tauri auto-updates and signed update manifests:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## GitHub repository variables (optional)

- `TAURI_BUNDLE_ENABLED`
  - Set to `1` to indicate the workflow should attempt signing/notarization-specific paths in future extensions.
- `TAURI_PRODUCT_NAME`
  - Override product name if you want CI metadata to differ from the scaffold default.

## How to create values

### Base64 encode a file

macOS/Linux:

```bash
base64 -i cert.p12 | tr -d '\n'
```

Windows PowerShell:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.p12"))
```

## macOS checklist (Developer ID + notarization)

1. Create/export a `Developer ID Application` certificate as `.p12`.
2. Add `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD`.
3. Add `APPLE_SIGNING_IDENTITY`.
4. Create an app-specific password for the Apple ID.
5. Add `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
6. Run `.github/workflows/tauri-bundle.yml` on a tag and verify notarization/stapling logs.

## Windows checklist (certificate signing)

1. Export signing certificate as `.pfx` (if using file-based signing).
2. Add `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD`.
3. Extend `.github/workflows/tauri-bundle.yml` with your signing step (provider-specific).
4. Verify the `.msi/.exe` signatures in CI artifacts.

## Security notes

- Prefer short-lived/revocable credentials where possible.
- Restrict repository admin access (secrets are high impact).
- Rotate secrets after contractor/vendor access changes.
- Avoid checking any signing material into git (even encrypted blobs).

## Current workflow behavior

- `tauri-bundle.yml` builds bundles without requiring secrets.
- Signing/notarization env vars are passed through only when configured.
- Missing secrets should not break local Rust builds or the non-Tauri release workflow.
