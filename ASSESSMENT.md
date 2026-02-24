# redlib-extended — Implementation Assessment

Assessment of the extended implementation (auth, OAuth, API, voting) with suggestions, fixes, and oversights.

---

## Overall quality

**Strengths**

- **Auth model** is clear: four modes (raw token, browser token, session cookie, anonymous) with a documented resolution order. Session crypto (AES-256-GCM, HKDF key derivation) is sound; CSRF for forms is present.
- **OAuth flow** is correct: state for CSRF, code exchange, token storage in encrypted cookie, no tokens sent to the browser.
- **SSH import** is a distinctive feature: validation of `ssh_host`/`ssh_user`, token extraction via remote sqlite, User-Agent sync with the imported browser.
- **API** is minimal and consistent: Bearer/cookie auth, whitelisted query params on subreddit listing, JSON error shape.
- **Voting/save** validate `thing_id` prefix (t1_/t3_), `dir`, and use `safe_return_to()` to avoid open redirects.
- **Config** is well extended with REDLIB_* and legacy LIBREDDIT_* and is documented in README.

**Weak spots**

- User session **access token refresh** is not implemented (see below).
- HTML pages (subreddit, post, user) do **not** use the logged-in user’s token for fetching; they still use the anonymous `json()` / `Post::fetch()`, so vote state and other user-specific data are not shown in the UI until after a vote.
- A few consistency and doc issues (query param filtering, save `thing_id` validation, README/app.json, hardcoded defaults).

---

## Critical: user session token refresh

**Issue:** `refresh_access_token()` exists in `auth.rs` but is never called. Logged-in users have an access token that expires in about an hour. After that, `AuthContext::from_request` treats the session as invalid (expires_at check) and the user is effectively logged out until they log in again, even though a valid `refresh_token` is stored in the cookie.

**Expected behavior:** When a request uses a session whose access token is expired (or when Reddit returns 401), the server should use `refresh_access_token(refresh_token)`, replace the access token (and optionally `expires_at`) in the session, set an updated session cookie, and retry the request (or proceed with the new token).

**Recommendation:** Implement “refresh on 401” in the client layer:

- In `authed_json` and `authed_post`, when the response is 401 and `auth` is `UserSession(s)` with non-empty `refresh_token`, call `auth::refresh_access_token(&s.refresh_token)`. On success, build updated `SessionData`, retry the request once, and return the result together with the updated session so the handler can call `auth::update_session_cookie(response, &new_session)`.
- Adjust return types so handlers can attach the updated cookie (e.g. return `(Value, Option<SessionData>)` from `authed_json` and the moral equivalent from `authed_post`), and update API and vote handlers to call `update_session_cookie` when they receive an updated session.

This keeps refresh logic in one place and avoids changing `from_request` to async or touching every call site in a heavy way.

---

## Other suggestions and fixes

### 1. API `post_comments` — query parameter filtering

**Issue:** `api::subreddit_listing` forwards only `ALLOWED_QUERY_PARAMS`. `api::post_comments` forwards the full query string to Reddit (`format!("?{query}&raw_json=1")`), which can allow arbitrary Reddit API parameters.

**Suggestion:** Reuse the same allow-list for comments (e.g. `depth`, `limit`, `sort`, etc. as needed) or a shared helper so both endpoints only forward allowed params.

### 2. Vote save — `thing_id` validation

**Issue:** `vote::submit` checks `thing_id` has prefix `t1_` or `t3_`. `vote::save` does not; it sends whatever `thing_id` is provided to Reddit.

**Suggestion:** Validate `thing_id` in `save()` the same way (t1_/t3_) for consistency and to avoid unnecessary Reddit calls for bad input.

### 3. HTML pages not using user auth for data

**Issue:** Subreddit, post, user, and search pages use `Post::fetch()` / `json()` (anonymous client). Logged-in users therefore do not get Reddit’s user-specific fields (e.g. `likes` for vote state, subscription state) in the initial render.

**Suggestion:** For pages that benefit from user context (at least subreddit and post), resolve `AuthContext::from_request(req)` and, when `auth.is_authenticated()`, use `authed_json` (or an equivalent authed `Post::fetch`) so the UI can show correct vote state and other per-user data. This is a larger change but aligns behavior with the “logged-in” experience described in the README.

### 4. README / docs

- **OpenAPI:** README points to `docs/openapi.yaml (in progress)`. The file is missing; either add a minimal spec or change the wording (e.g. “planned” or remove the link).
- **Clone URL:** Getting started still says `git clone https://github.com/your-org/redlib-extended`; consider updating to the real repo URL (e.g. GitLab).

### 5. `app.json` (e.g. Heroku/deploy)

**Issue:** `app.json` does not declare the extended env vars (`REDLIB_OAUTH_*`, `REDLIB_SESSION_SECRET`, `REDLIB_RAW_TOKEN`, etc.), so deploy UIs won’t prompt for them.

**Suggestion:** Add the extended variables to `app.json` with `"required": false` (and `true` only where deployment should fail without them, if desired).

### 6. Login page defaults

**Issue:** Defaults for SSH import are hardcoded (`kspld0`, `keith`) in `auth.rs` when config is missing. README documents them as defaults, but they are very instance-specific.

**Suggestion:** Prefer config-only (no fallback) so missing `REDLIB_SSH_HOST` / `REDLIB_SSH_USER` leaves the form empty or shows a placeholder like “hostname” / “username”. If you keep defaults, document them clearly as “example defaults” so operators know to set their own.

### 7. Rate limiting and API

README mentions “Rate limiting and per-token scoping” and `REDLIB_API_RATE_LIMIT`. The codebase does not yet enforce per-token rate limits for the REST API; only the anonymous `OAUTH_CLIENT` rate limit is tracked. Worth adding later if you offer programmatic access.

### 8. Minor code quality

- **auth.rs:** `secure_cookies()` could be a small helper that takes `Option<&str>` and returns `bool` to keep the logic in one place and ease testing.
- **api.rs:** Duplication between subreddit_listing and post_comments (building path, choosing authed vs anon) could be reduced with a small helper that takes path + auth and returns `Value`.
- **vote.rs:** Doc comment says `POST /r/:sub/comments/:id/save` but the route is `POST /save` with `thing_id` in the body; consider aligning the comment with the actual route.

---

## Summary

| Area              | Verdict   | Notes                                                |
|-------------------|-----------|------------------------------------------------------|
| Auth / session    | Good      | Add refresh on 401 and set updated cookie            |
| OAuth / SSH import| Good      | Solid design and validation                          |
| API               | Good      | Add query filtering to post_comments                 |
| Voting / save     | Good      | Add thing_id validation in save                       |
| HTML + auth       | Oversight | Use authed fetch for subreddit/post when logged in    |
| Docs / deploy     | Minor     | OpenAPI, clone URL, app.json, SSH defaults            |

Implementing user session refresh (and optionally the small API/vote/doc fixes above) will address the main functional gap and harden consistency.
