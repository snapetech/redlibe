# redlib-extended

> A fork of [Redlib](https://github.com/redlib-org/redlib) extending the privacy-first Reddit front-end with user authentication, OAuth, a REST API, voting, and more.

---

## Overview

**redlib-extended** builds on top of Redlib's fast, private, Rust-based Reddit front-end to add first-class support for authenticated Reddit accounts. The goal is to preserve everything that makes Redlib great — no tracking, no ads, no bloat — while enabling users to interact with Reddit through their own accounts when they choose to.

---

## Extended Features

### User Authentication
- Reddit OAuth 2.0 login flow (authorization code grant)
- Secure session management via signed, server-side cookies
- Login / logout UI integrated into the existing Redlib interface
- Optional: anonymous browsing remains fully supported without an account

### REST API
- JSON API layer over the existing Redlib rendering logic
- Endpoints mirror standard Reddit API paths (`/r/{sub}`, `/u/{user}`, `/comments/{id}`, etc.)
- Bearer token auth for programmatic access
- Rate limiting and per-token scoping

### OAuth Integration
- Register a Reddit OAuth app and configure client ID/secret via environment variables or config file
- Scopes: `identity`, `read`, `vote`, `submit`, `subscribe`, `history`
- Token refresh handled transparently on behalf of the user

### Voting
- Upvote / downvote posts and comments through the proxied server
- Vote state reflected in the UI (no direct browser-to-Reddit requests)
- Requires a logged-in session

### Planned / In Progress
- Comment submission and reply
- Post submission (text, link, image)
- Subreddit subscription management
- User flair display
- Saved posts and comments
- Moderation queue (basic)

---

## Architecture

redlib-extended is written in Rust and inherits Redlib's stack:

| Layer | Technology |
|---|---|
| Web framework | Hyper 0.14 |
| Templating | Askama |
| Async runtime | Tokio |
| TLS | Rustls / hyper-rustls |
| Serialization | Serde / serde_json |
| Session storage | Server-side (configurable backend) |
| OAuth tokens | Encrypted at rest, stored server-side |

---

## Getting Started

### Prerequisites

- Rust 1.81+
- A Reddit OAuth application ([create one here](https://www.reddit.com/prefs/apps))

### Environment Variables

```env
# Inherited from Redlib
REDLIB_DEFAULT_THEME=system
REDLIB_DEFAULT_SHOW_NSFW=false

# Extended: OAuth / Auth
REDLIB_OAUTH_CLIENT_ID=your_client_id
REDLIB_OAUTH_CLIENT_SECRET=your_client_secret
REDLIB_OAUTH_REDIRECT_URI=https://your-instance.example.com/auth/callback
REDLIB_SESSION_SECRET=a_long_random_secret_key

# Extended: API
REDLIB_API_ENABLED=true
REDLIB_API_RATE_LIMIT=100   # requests per minute per token
```

### Build & Run

```bash
git clone https://github.com/your-org/redlib-extended
cd redlib-extended
cargo build --release
./target/release/redlib
```

### Docker

```bash
docker compose up -d
```

---

## API Reference

All API endpoints are prefixed with `/api/v1`.

| Method | Path | Auth Required | Description |
|---|---|---|---|
| GET | `/api/v1/r/{sub}` | No | Subreddit posts |
| GET | `/api/v1/r/{sub}/comments/{id}` | No | Post comments |
| GET | `/api/v1/u/{user}` | No | User profile |
| POST | `/api/v1/vote` | Yes | Vote on a post or comment |
| GET | `/api/v1/me` | Yes | Current user info |
| POST | `/api/v1/subscribe` | Yes | Subscribe/unsubscribe from a sub |

Full OpenAPI spec: `docs/openapi.yaml` *(in progress)*

---

## Auth Flow

```
User clicks "Login"
  → Redirected to Reddit OAuth consent screen
  → Reddit redirects back to /auth/callback with code
  → Server exchanges code for access + refresh tokens
  → Tokens stored server-side, session cookie issued to browser
  → All subsequent Reddit API calls made server-side using stored tokens
```

No Reddit tokens are ever sent to the browser.

---

## Privacy Model

redlib-extended preserves Redlib's core privacy guarantees:

- All requests to Reddit are made **server-side**
- User IP addresses are **never** forwarded to Reddit
- No third-party JavaScript or tracking pixels
- OAuth tokens are stored **server-side only**, never in the browser
- Anonymous browsing requires **no account** and remains fully private

When a user logs in, their Reddit identity is used only for actions they explicitly take (voting, subscribing, etc.). The server does not log or retain activity history.

---

## Configuration

Full configuration reference: see `contrib/` and `redlib.toml.example` *(coming soon)*.

---

## Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Write tests for new behavior
4. Submit a pull request

Please read `CONTRIBUTING.md` before opening a PR.

---

## License

AGPL-3.0-only — same as upstream Redlib.

---

## Credits

- Upstream [Redlib](https://github.com/redlib-org/redlib) by Matthew Esposito and contributors
- Original [Libreddit](https://github.com/libreddit/libreddit) by spikecodes
