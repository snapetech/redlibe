# redlib-extended features

This document describes the features added by redlib-extended on top of upstream [Redlib](https://github.com/redlib-org/redlib). For a **feature gap vs full Reddit clients (Sync/Hydra)** and a prioritized backlog, see [docs/FEATURE_GAP_BACKLOG.md](docs/FEATURE_GAP_BACKLOG.md).

---

## User authentication

- **Reddit OAuth 2.0** — Log in with a Reddit account via the standard OAuth authorization code flow. Configure `REDLIB_OAUTH_CLIENT_ID`, `REDLIB_OAUTH_CLIENT_SECRET`, and `REDLIB_OAUTH_REDIRECT_URI` (e.g. `https://your-instance.example.com/auth/callback`).
- **Session management** — Encrypted server-side sessions (AES-256-GCM) keyed by `REDLIB_SESSION_SECRET`. No Reddit tokens are sent to the browser.
- **SSH session import** — Sign in without OAuth by importing an existing Reddit session from a browser on a remote Linux host over SSH. See [SSH session import](#ssh-session-import) below.
- **Anonymous browsing** — Unauthenticated use is fully supported; login is optional.

---

## SSH session import

Import the active Reddit session (cookie `token_v2`) from a Firefox or LibreWolf profile on a machine you can SSH into. No Reddit app registration is required.

### How it works

1. You open the login page and choose **Import session via SSH**.
2. You provide **SSH host**, **SSH user**, and at least one of:
   - **Your SSH private key** (paste into the textarea) — the same key you use to log into that host from your machine (e.g. `ssh user@host`); the redlibe server uses it once to SSH from the pod to the host, or  
   - **SSH password** (the password for that user on the host).
3. You select the browser (**LibreWolf** or **Firefox**).
4. The server runs `ssh` (or `sshpass` + `ssh` for password auth) to the remote host and executes a small script that:
   - Finds `cookies.sqlite` in the browser profile directory.
   - Reads the `token_v2` cookie for `.reddit.com` with `sqlite3`.
   - Reads browser version and architecture for a matching User-Agent.
5. The server decodes the token, creates a session, and sets a session cookie. You are logged in.

### Requirements

- **Remote host (the machine you SSH into)**  
  - **Linux** — The import script and paths are written for Linux. Other OSes are not supported.
  - **SSH** — `sshd` running; the redlib-extended server must be able to open an SSH connection to this host.
  - **Browser** — One of:
    - **LibreWolf** — Profile data must live under `~/.librewolf` (default LibreWolf profile path).
    - **Firefox** — Profile data must live under `~/.mozilla/firefox`.
  - **sqlite3** — Installed on the remote host (used to query `cookies.sqlite`).
  - **Reddit session** — You must have logged into Reddit in that browser at least once so the `token_v2` cookie exists.

- **Server (where redlib-extended runs)**  
  - **Key auth:** `openssh-client` (the container image includes it).  
  - **Password auth:** `sshpass` (the container image includes it).  
  - Either paste the private key in the form or provide the SSH password; the server uses it only for that request and does not store it.

### Security notes

- The private key or password is sent over HTTPS to the server and used once for the SSH call; it is not stored.
- For key auth, the key is written to a temporary file with mode `0600` and deleted after the request.
- For password auth, the password is passed to `sshpass`; avoid using this on untrusted networks or shared machines.

---

## REST API

- JSON API over Redlib’s data and rendering logic.
- Endpoints follow Reddit-style paths: `/api/v1/r/{sub}`, `/api/v1/r/{sub}/comments/{id}`, `/api/v1/u/{user}`, etc.
- Bearer token authentication for programmatic access.
- Enable with `REDLIB_API_ENABLED=true`; optional rate limiting.

---

## Voting

- Upvote / downvote posts and comments through the server.
- Vote state is reflected in the UI.
- Requires a logged-in session (OAuth or imported SSH session).

---

## Configuration

- **OAuth:** `REDLIB_OAUTH_CLIENT_ID`, `REDLIB_OAUTH_CLIENT_SECRET`, `REDLIB_OAUTH_REDIRECT_URI`, `REDLIB_SESSION_SECRET`.
- **SSH (optional, for key auth without pasting):** `REDLIB_SSH_HOST`, `REDLIB_SSH_USER`, `REDLIB_SSH_KEY` (path to private key, supports `~`).
- **API:** `REDLIB_API_ENABLED`, `REDLIB_API_RATE_LIMIT`.
- All other Redlib settings (theme, NSFW, etc.) continue to apply.

---

## Comment reply and custom feeds

- **Comment reply** — Reply to comments and top-level “Add a comment” on posts; submitted via Reddit API (`submit` scope).
- **Custom feeds** — Named multireddits stored in a cookie; manage at `/feeds`, linked from Feeds menu and `/feed/:name`.

---

## Privacy

- All Reddit requests are made server-side; user IPs are not sent to Reddit.
- No third-party JS or tracking.
- OAuth and imported tokens stay server-side only.
- Anonymous use remains fully supported with no account.
