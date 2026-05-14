# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Async void boundaries

## Silent catch or lossy exception boundaries

## Callback/event invocation boundaries

## Remote/user text in diagnostics or HTTP errors
src/client.rs:150:			 Please retry shortly or log in so requests can use your account token. | {path}"
src/auth.rs:187:	/// A logged-in Reddit user with a real OAuth access token.
src/auth.rs:279:	/// Return the logged-in username, if any.
src/auth.rs:606:async fn complete_browser_import_login(bearer_token: String, user_agent: Option<String>) -> Result<Response<Body>, String> {
src/auth.rs:639:/// `POST /login/reddit` — generate a CSRF state token, then redirect to Reddit's
src/auth.rs:679:/// `POST /login/ssh-import` — extract a Reddit bearer token from a Firefox or
src/auth.rs:798:		Ok((bearer_token, user_agent)) => complete_browser_import_login(bearer_token, Some(user_agent)).await,
src/auth.rs:802:/// `POST /login/local-import` — import a browser token from a local browser profile.
src/auth.rs:821:		Ok(imported) => complete_browser_import_login(imported.bearer_token, imported.user_agent).await,
src/auth.rs:829:/// Re-render the login page with an inline error message.
src/auth.rs:947:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:1058:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:1162:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:1241:/// `POST /logout` — validate CSRF token, remove active session from vault.
src/auth.rs:1349:		log::error!("OAuth token exchange failed: HTTP {status} — {}", String::from_utf8_lossy(&bytes));
src/auth.rs:1355:		log::error!("OAuth token exchange: failed to parse Reddit token response");
src/settings.rs:337:		if key == "available_themes" || key == "logged_in" || key == "username" || key == "csrf_token" {
src/utils.rs:666:	/// Whether the current request is authenticated (user login or raw token).
src/inbox.rs:146:		return Err("You must be logged in to send a message.".to_string());
src/inbox.rs:211:		return Err("You must be logged in to read messages.".to_string());
src/inbox.rs:239:		return Err("You must be logged in to read messages.".to_string());

## Red-team abuse lens
scripts/run-council-active-bughunt.sh:25:    rg -n -U --with-filename --pcre2 --hidden --glob '!.git/**' --glob '!.council/**' "$pattern" "$@" || true
scripts/run-council-active-bughunt.sh:41:# Replace paths and patterns for your repo. Add narrow sections whenever a
scripts/run-council-active-bughunt.sh:61:  '(log|logger|Diagnostic|Console\.WriteLine|StatusCode\(|BadRequest\()[^;\n]*(username|query|filename|directory|token|message)' \
scripts/run-council-active-bughunt.sh:66:  '(token|secret|password|authorization|cookie|api[-_]?key|session|redirect|proxy|forwarded|path|filename|exec|spawn|shell|http://|https://)' \
docs/dev/bug-council-active-backlog.md:34:| `Red-team abuse lens` | 0 | Open | Required recurring attacker-view review across secrets, identity, redirects, paths, process launch, and downgrade risks. | Turn accepted hypotheses into behavior tests plus remediation anchors; add preservation tests for normal functionality. |
tests/token_import_local.rs:2:use std::path::{Path, PathBuf};
tests/token_import_local.rs:4:use redlib::token_import;
tests/token_import_local.rs:9:fn firefox_discover_profiles_and_import_local_token() {
tests/token_import_local.rs:13:	create_firefox_cookie_db(&profile.join("cookies.sqlite"), "header.payload.sig");
tests/token_import_local.rs:15:	let profiles = token_import::firefox::discover_profiles_in_base("firefox", &root);
tests/token_import_local.rs:20:	// Use manual path for deterministic import in CI/tests.
tests/token_import_local.rs:21:	let imported = token_import::import_local("firefox", None, Some(profile.to_str().unwrap())).unwrap();
tests/token_import_local.rs:22:	assert_eq!(imported.bearer_token, "header.payload.sig");
tests/token_import_local.rs:26:fn firefox_import_errors_when_no_cookie_present() {
tests/token_import_local.rs:30:	create_firefox_cookie_db_without_token(&profile.join("cookies.sqlite"));
tests/token_import_local.rs:32:	let err = token_import::import_local("firefox", None, Some(profile.to_str().unwrap())).unwrap_err();
tests/token_import_local.rs:33:	assert!(err.contains("token_v2"), "unexpected error: {err}");
tests/token_import_local.rs:37:fn chromium_discover_profiles_and_import_plaintext_cookie() {
tests/token_import_local.rs:41:	create_chromium_cookie_db(&profile.join("Cookies"), "jwt.part.sig", &[]);
tests/token_import_local.rs:43:	let profiles = token_import::chromium::discover_profiles_in_base("chrome", &root);
tests/token_import_local.rs:48:	let imported = token_import::import_local("chrome", None, Some(profile.to_str().unwrap())).unwrap();
tests/token_import_local.rs:49:	assert_eq!(imported.bearer_token, "jwt.part.sig");
tests/token_import_local.rs:53:fn chromium_import_returns_clear_error_for_undecryptable_cookie() {
tests/token_import_local.rs:57:	create_chromium_cookie_db(&profile.join("Cookies"), "", b"v10notreallyencrypted");
tests/token_import_local.rs:59:	let err = token_import::import_local("chrome", None, Some(profile.to_str().unwrap())).unwrap_err();
tests/token_import_local.rs:65:	let err = token_import::import_local("chrome", Some("chrome:missing"), None).unwrap_err();
tests/token_import_local.rs:72:fn create_firefox_cookie_db(path: &Path, token: &str) {
tests/token_import_local.rs:73:	let conn = Connection::open(path).unwrap();
tests/token_import_local.rs:75:		.execute_batch(
tests/token_import_local.rs:76:			"CREATE TABLE moz_cookies (
tests/token_import_local.rs:82:			path TEXT,
tests/token_import_local.rs:96:		.execute(
tests/token_import_local.rs:97:			"INSERT INTO moz_cookies (name, value, host, path, expiry, lastAccessed, creationTime, isSecure, isHttpOnly)
tests/token_import_local.rs:99:			("token_v2", token),
tests/token_import_local.rs:104:fn create_firefox_cookie_db_without_token(path: &Path) {
tests/token_import_local.rs:105:	let conn = Connection::open(path).unwrap();
tests/token_import_local.rs:107:		.execute_batch(
tests/token_import_local.rs:108:			"CREATE TABLE moz_cookies (
tests/token_import_local.rs:114:			path TEXT,
tests/token_import_local.rs:124:		.execute(
tests/token_import_local.rs:125:			"INSERT INTO moz_cookies (name, value, host, path, expiry, lastAccessed, creationTime, isSecure, isHttpOnly)
tests/token_import_local.rs:132:fn create_chromium_cookie_db(path: &Path, value: &str, encrypted_value: &[u8]) {
tests/token_import_local.rs:133:	let conn = Connection::open(path).unwrap();
tests/token_import_local.rs:135:		.execute_batch(
tests/token_import_local.rs:136:			"CREATE TABLE cookies (
tests/token_import_local.rs:143:			path TEXT NOT NULL DEFAULT '/',
tests/token_import_local.rs:160:		.execute(
tests/token_import_local.rs:161:			"INSERT INTO cookies (host_key, name, value, encrypted_value, last_access_utc)
tests/token_import_local.rs:163:			(".reddit.com", "token_v2", value, encrypted_value, 10_i64),
scripts/check-bug-council-all-phases.sh:26:  printf 'Council all-phases runner is missing or not executable: %s\n' "${runner#$repo_root/}" >&2
docs/dev/bug-council-negative-space.md:19:| _replace_with_your_boundary_ | _network input_ | `src/path/to/sink.ext` | `ValidateInputName` |
src/api.rs:6://!   - Header: `Authorization: Bearer <access_token>` (takes priority over cookie)
src/api.rs:7://!   - Cookie: `rl_session` encrypted session (fallback)
src/api.rs:17:use crate::auth::{update_session_cookie, AuthContext};
src/api.rs:25:/// Allowed query parameter names forwarded to Reddit to prevent parameter injection.
src/api.rs:47:/// header before falling back to the cookie session.
src/api.rs:49:	// Check for `Authorization: Bearer <token>` header first
src/api.rs:52:			if let Some(token) = value.strip_prefix("Bearer ") {
src/api.rs:53:				if !token.is_empty() {
src/api.rs:54:					return AuthContext::RawBearer(token.to_string());
src/api.rs:59:	// Fall back to cookie-based session
src/api.rs:84:	let path = format!("/r/{sub}/hot.json{qs}");
src/api.rs:86:	let (data, session_updated) = match auth.bearer_token() {
src/api.rs:87:		Some(_) => authed_json(path, false, &auth).await?,
src/api.rs:88:		None => (anon_json(path, false).await?, None),
src/api.rs:92:	if let Some(s) = session_updated {
src/api.rs:93:		update_session_cookie(&mut res, &s);
src/api.rs:105:	let path = format!("/r/{sub}/comments/{id}.json{qs}");
src/api.rs:107:	let (data, session_updated) = match auth.bearer_token() {
src/api.rs:108:		Some(_) => authed_json(path, false, &auth).await?,
src/api.rs:109:		None => (anon_json(path, false).await?, None),
src/api.rs:113:	if let Some(s) = session_updated {
src/api.rs:114:		update_session_cookie(&mut res, &s);
src/api.rs:126:		return json_error(401, "Authentication required — provide Authorization: Bearer <token> header or log in via /login");
src/api.rs:129:	let (data, session_updated) = authed_json("/api/v1/me.json?raw_json=1".to_string(), false, &auth).await?;
src/api.rs:132:	if let Some(s) = session_updated {
src/api.rs:133:		update_session_cookie(&mut res, &s);
src/api.rs:143:/// Requires authentication. No CSRF token required for API calls
src/api.rs:144:/// (Bearer token acts as the auth proof).
src/api.rs:184:	let (_, session_updated) = authed_post("/api/vote".to_string(), body_str, &auth).await?;
src/api.rs:187:	if let Some(s) = session_updated {
src/api.rs:188:		update_session_cookie(&mut res, &s);
scripts/check-council-negative-space.sh:65:#   "src/path/to/sink.ext" \
docs/dev/bug-council-severity-schema.md:12:| Low | Defensive-depth gap: code path is currently unreachable from untrusted input, but the absence of the guard is itself a hazard if a refactor exposes it. |
docs/dev/bug-council-severity-schema.md:15:Pick the **worst plausible** severity given current code paths. If the same code is reachable from two boundaries with different severities, take the higher.
src/go.rs:3:use crate::utils::{param, redirect, template, Preferences};
src/go.rs:8:#[template(path = "go.html")]
src/go.rs:14:/// GET /go?r=subname → redirect to /r/subname. GET /go → show "Go to subreddit" form.
src/go.rs:17:	let path = format!("?{query}");
src/go.rs:18:	let r_param = param(&path, "r").unwrap_or_default();
src/go.rs:24:			return Ok(redirect(&format!("/r/{sub}")));
src/edit.rs:3://! POST /edit — form: thing_id (fullname t1_/t3_), action (edit|delete), text (for edit), csrf_token, return_to.
src/edit.rs:9:use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
src/edit.rs:11:use crate::utils::redirect;
src/edit.rs:37:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/edit.rs:38:	validate_csrf_token(&auth, submitted_csrf)?;
src/edit.rs:56:		let (value, session_updated) = authed_post("/api/del".to_string(), body_str, &auth).await?;
src/edit.rs:60:		let mut res = redirect(return_to);
src/edit.rs:61:		if let Some(s) = session_updated {
src/edit.rs:62:			update_session_cookie(&mut res, &s);
src/edit.rs:76:	let (value, session_updated) = authed_post("/api/editusertext".to_string(), body_str, &auth).await?;
src/edit.rs:90:	let mut res = redirect(return_to);
src/edit.rs:91:	if let Some(s) = session_updated {
src/edit.rs:92:		update_session_cookie(&mut res, &s);
src/submit.rs:10:use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
src/submit.rs:13:use crate::utils::{error, redirect, template, Preferences};
src/submit.rs:21:#[template(path = "submit.html")]
src/submit.rs:33:		return Ok(redirect("/login"));
src/submit.rs:65:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/submit.rs:66:	validate_csrf_token(&auth, submitted_csrf)?;
src/submit.rs:87:		if !url_link.starts_with("http://") && !url_link.starts_with("https://") {
src/submit.rs:88:			return Err("URL must start with http:// or https://.".to_string());
src/submit.rs:106:	let (value, session_updated) = authed_post("/api/submit".to_string(), body_str, &auth).await?;
src/submit.rs:124:	let redirect_url: String = if let Some(data) = json.and_then(|j| j.get("data")).and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
src/submit.rs:134:	let mut res = redirect(&redirect_url);
src/submit.rs:135:	if let Some(s) = session_updated {
src/submit.rs:136:		update_session_cookie(&mut res, &s);
docs/dev/bug-council-scan-registry.md:39:| Untrusted-string-to-path | Find file-system operations on caller-supplied strings without containment. |
docs/dev/bug-council-scan-registry.md:40:| Security-sensitive material | Find high-confidence private keys and token patterns. |
docs/dev/bug-council-scan-registry.md:41:| Red-team abuse lens | Re-check accepted fixes from an attacker viewpoint: spoofed identity, secret disclosure, confused deputy, replay, SSRF/path/process escape, and operational downgrade. |
src/feeds.rs:1://! Custom feeds: named multireddits stored in a cookie.
src/feeds.rs:5://! - GET /feed/:name — redirect to /r/sub1+sub2+...
src/feeds.rs:9:use cookie::Cookie;
src/feeds.rs:14:use crate::utils::{parse_custom_feeds_cookie, redirect, template, CustomFeed, Preferences};
src/feeds.rs:21:fn set_custom_feeds_cookie(response: &mut Response<Body>, feeds: &[CustomFeed]) {
src/feeds.rs:24:	response.insert_cookie(
src/feeds.rs:26:			.path("/")
src/feeds.rs:70:#[template(path = "feeds.html")]
src/feeds.rs:84:	let feeds = parse_custom_feeds_cookie(&req);
src/feeds.rs:104:	let mut feeds = parse_custom_feeds_cookie(&req);
src/feeds.rs:156:		redirect("/feeds")
src/feeds.rs:166:	set_custom_feeds_cookie(&mut res, &feeds);
src/feeds.rs:170:/// GET /feed/:name — redirect to /r/sub1+sub2+...
src/feeds.rs:171:pub async fn redirect_to_feed(req: Request<Body>) -> Result<Response<Body>, String> {
src/feeds.rs:174:	let feeds = parse_custom_feeds_cookie(&req);
src/feeds.rs:178:			let path = format!("/r/{}", f.subreddits);
src/feeds.rs:179:			Ok(redirect(&path))
docs/dev/bug-council-roslyn-analyzers.md:23:| CSL0004 | TaintToFilePath | High | Network-derived file/directory path without sanctioned containment validation. This catches hostile paths before filesystem sinks trust them. |
src/comment.rs:3://! `POST /comment` — submit a reply. Form: parent (fullname t1_ or t3_), text, csrf_token, return_to.
src/comment.rs:10:use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
src/comment.rs:12:use crate::utils::redirect;
src/comment.rs:39:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/comment.rs:40:	validate_csrf_token(&auth, submitted_csrf)?;
src/comment.rs:61:	let (value, session_updated) = authed_post("/api/comment".to_string(), body_str, &auth).await?;
src/comment.rs:81:	let mut res = redirect(return_to);
src/comment.rs:82:	if let Some(s) = session_updated {
src/comment.rs:83:		update_session_cookie(&mut res, &s);
docs/dev/bug-council-phases.md:8:| 2 | Semantic analyzer beachhead | _Pending / In progress / Done_ | _agent_ | One language-appropriate semantic analyzer (Roslyn / Clippy / ESLint) implementing a taint-to-allocation or taint-to-path lens, with tests. |
docs/dev/bug-council-phases.md:16:| 10 | Additional semantic lens batch | _Pending / In progress / Done_ | _agent_ | Add several distinct semantic lenses in one batch, such as tainted protocol offsets, paths, timeouts, endpoints, enum/status conversions, slice bounds, diagnostic/log-line text, outbound messages, cache keys, crypto trust material, dynamic execution, parser runtimes, resource capacities, and buffer operations, with unit tests and calibration. |
src/vote.rs:8://!   - `csrf_token` — must match the session's stored CSRF token
src/vote.rs:9://!   - `return_to`  — URL to redirect back to after voting (must be a relative path; defaults to `/`)
src/vote.rs:18:use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
src/vote.rs:20:use crate::utils::redirect;
src/vote.rs:25:/// Validate and sanitize a `return_to` redirect target.
src/vote.rs:27:/// Only allows relative paths starting with `/` (but not `//`, which could
src/vote.rs:28:/// be interpreted as a protocol-relative URL and used for open redirect attacks).
src/vote.rs:55:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/vote.rs:56:	validate_csrf_token(&auth, submitted_csrf)?;
src/vote.rs:82:	let (_, session_updated) = authed_post("/api/vote".to_string(), body_str, &auth).await?;
src/vote.rs:84:	let mut res = redirect(return_to);
src/vote.rs:85:	if let Some(s) = session_updated {
src/vote.rs:86:		update_session_cookie(&mut res, &s);
src/vote.rs:105:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/vote.rs:106:	validate_csrf_token(&auth, submitted_csrf)?;
src/vote.rs:116:	let (_, session_updated) = authed_post(endpoint.to_string(), body_str, &auth).await?;
src/vote.rs:118:	let mut res = redirect(return_to);
src/vote.rs:119:	if let Some(s) = session_updated {
src/vote.rs:120:		update_session_cookie(&mut res, &s);
scripts/scan-bug-council-candidates.sh:24:  rg -n --with-filename --pcre2 --hidden --glob '!.git/**' "$pattern" "$@" || true
scripts/scan-bug-council-candidates.sh:33:  'PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{36,}|xox[baprs]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|(?i)(api[_-]?key|access[_-]?token|client[_-]?secret)' \
scripts/scan-bug-council-candidates.sh:57:#   'tokio::spawn|select!|timeout\(|sleep\(|interval\(|mpsc|broadcast|oneshot' \
scripts/load_test.py:5:base_url = "http://localhost:8080"
scripts/load_test.py:7:full_path = f"{base_url}/r/politics"
scripts/load_test.py:18:while full_path:
scripts/load_test.py:19:    response = requests.get(full_path)
scripts/load_test.py:25:    with ThreadPoolExecutor(max_workers=10) as executor:
scripts/load_test.py:26:        executor.map(fetch_url, comment_urls)
scripts/load_test.py:29:        full_path = base_url + next_link['href']
scripts/check-remediation-baseline.sh:24:  local path="$1"
scripts/check-remediation-baseline.sh:27:  if [[ -f "$path" ]]; then
scripts/check-remediation-baseline.sh:30:    fail "$label: missing $path"
scripts/check-remediation-baseline.sh:36:  local path="$2"
scripts/check-remediation-baseline.sh:39:  if rg -n -U --pcre2 --hidden --glob '!.git/**' "$pattern" "$path" >/dev/null; then
scripts/check-remediation-baseline.sh:48:  local path="$2"
scripts/check-remediation-baseline.sh:54:  if rg -n -U --pcre2 --hidden --glob '!.git/**' "$pattern" "$path" >"$hit_file" 2>/dev/null; then
scripts/check-remediation-baseline.sh:109:# require_pattern "ValidateInputName" "src/path/to/sink" "input validator wired"
scripts/check-remediation-baseline.sh:110:# require_pattern "MaxRequestSize" "src/path/to/limit" "request size bound declared"
scripts/check-remediation-baseline.sh:113:secret_pattern='-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9_]{36,}|xox[baprs]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|(?i)(api[_-]?key|access[_-]?token|client[_-]?secret)["'\'']?\s*[:=]\s*["'\''][A-Za-z0-9_./+=-]{24,}["'\'']'
scripts/check-remediation-baseline.sh:114:require_absent_pattern "$secret_pattern" "." "tracked text files do not contain high-confidence secret patterns"
scripts/check-council-sweep-counts.sh:82:#   "secret-pattern sweep count matches scanner"
scripts/check-local-identity-leaks.sh:17:tmp_tokens="$(mktemp)"
scripts/check-local-identity-leaks.sh:20:trap 'rm -f "$tmp_tokens" "$tmp_commits" "$tmp_files"' EXIT
scripts/check-local-identity-leaks.sh:22:add_token() {
scripts/check-local-identity-leaks.sh:23:  local token="$1"
scripts/check-local-identity-leaks.sh:24:  token="${token//$'\n'/}"
scripts/check-local-identity-leaks.sh:25:  token="${token//$'\r'/}"
scripts/check-local-identity-leaks.sh:26:  [[ ${#token} -ge 3 ]] || return 0
scripts/check-local-identity-leaks.sh:27:  case "$token" in
scripts/check-local-identity-leaks.sh:32:  printf '%s\n' "$token" >>"$tmp_tokens"
scripts/check-local-identity-leaks.sh:35:add_token "${LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:36:add_token "${SLSKDN_LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:37:add_token "${SLSKDN_FORBIDDEN_LOCAL_HOSTNAME:-}"
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:39:add_token "${USER:-}"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:43:read_csv_tokens() {
scripts/check-local-identity-leaks.sh:46:  IFS=',' read -ra tokens <<<"$value"
scripts/check-local-identity-leaks.sh:47:  for token in "${tokens[@]}"; do
scripts/check-local-identity-leaks.sh:48:    add_token "$token"
scripts/check-local-identity-leaks.sh:52:read_csv_tokens "${LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:53:read_csv_tokens "${SLSKDN_LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:58:  while IFS= read -r token; do
scripts/check-local-identity-leaks.sh:59:    [[ "$token" =~ ^[[:space:]]*# ]] && continue
scripts/check-local-identity-leaks.sh:60:    add_token "$token"
scripts/check-local-identity-leaks.sh:67:sort -u "$tmp_tokens" -o "$tmp_tokens"
scripts/check-local-identity-leaks.sh:68:if [[ ! -s "$tmp_tokens" ]]; then
scripts/check-local-identity-leaks.sh:69:  echo "No local identity tokens configured for scanning."
scripts/check-local-identity-leaks.sh:77:  local path="$2"
scripts/check-local-identity-leaks.sh:78:  local display_path="${3:-$path}"
scripts/check-local-identity-leaks.sh:81:  [[ -f "$path" ]] || return 0
scripts/check-local-identity-leaks.sh:83:    rg --json --fixed-strings --ignore-case --file "$tmp_tokens" "$path" |
scripts/check-local-identity-leaks.sh:84:      jq -r --arg label "$label" --arg display_path "$display_path" 'select(.type == "match") | "\($label): \($display_path):\(.data.line_number)"' |
scripts/check-local-identity-leaks.sh:96:  trap 'rm -f "$tmp_tokens" "$tmp_commits" "$tmp_files" "$tmp_unreleased"' EXIT
scripts/check-local-identity-leaks.sh:117:  -path './.git' -prune -o \
scripts/check-local-identity-leaks.sh:118:  -path './node_modules' -prune -o \
scripts/check-local-identity-leaks.sh:119:  -path './vendor' -prune -o \
scripts/check-local-identity-leaks.sh:120:  -path './target' -prune -o \
scripts/check-local-identity-leaks.sh:121:  -path './dist' -prune -o \
scripts/check-local-identity-leaks.sh:122:  -path './build' -prune -o \
scripts/check-local-identity-leaks.sh:123:  -path './zeek/pkg' -prune -o \
scripts/check-local-identity-leaks.sh:125:    -path './.github/release-notes/*' -o \
scripts/check-local-identity-leaks.sh:126:    -path './docs/dev/release-copy.md' -o \
scripts/check-local-identity-leaks.sh:127:    -path './docs/release*.md' -o \
scripts/check-local-identity-leaks.sh:128:    -path './docs/RELEASE*.md' -o \
scripts/check-local-identity-leaks.sh:129:    -path './packaging/winget/*' \
scripts/check-local-identity-leaks.sh:132:while IFS= read -r path; do
scripts/check-local-identity-leaks.sh:133:  [[ -n "$path" ]] || continue
scripts/check-local-identity-leaks.sh:134:  check_file "$path" "$path"
src/user.rs:4:use crate::auth::{update_session_cookie, AuthContext};
src/user.rs:9:	get_read_ids, nsfw_landing, param, redirect, setting, template, Post, Preferences, User,
src/user.rs:20:#[template(path = "user.html")]
src/user.rs:30:	redirect_url: String,
src/user.rs:51:			return Ok(redirect("/login"));
src/user.rs:61:		return Ok(redirect(&to));
src/user.rs:65:	let path = format!("/user/{}/{listing}.json?{}&raw_json=1", username, req.uri().query().unwrap_or_default(),);
src/user.rs:66:	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));
src/user.rs:67:	let redirect_url = url[1..].replace('?', "%3F").replace('&', "%26");
src/user.rs:68:	let sort = param(&path, "sort").unwrap_or_default();
src/user.rs:81:			sort: (sort.clone(), param(&path, "t").unwrap_or_default()),
src/user.rs:82:			ends: (param(&path, "after").unwrap_or_default(), String::new()),
src/user.rs:86:			redirect_url,
src/user.rs:96:		return Ok(redirect("/login"));
src/user.rs:100:		let ((mut p, a), session_updated) = Post::fetch_authed(&path, false, &auth).await.map_err(|msg| msg.to_string())?;
src/user.rs:115:			sort: (sort, param(&path, "t").unwrap_or_default()),
src/user.rs:116:			ends: (param(&path, "after").unwrap_or_default(), a),
src/user.rs:120:			redirect_url,
src/user.rs:126:		if let Some(s) = session_updated {
src/user.rs:127:			update_session_cookie(&mut res, &s);
src/user.rs:131:		match Post::fetch(&path, false).await {
src/user.rs:151:		sort: (sort, param(&path, "t").unwrap_or_default()),
src/user.rs:152:		ends: (param(&path, "after").unwrap_or_default(), after),
src/user.rs:156:		redirect_url,
src/user.rs:166:	// Build the Reddit JSON API path
src/user.rs:167:	let path: String = format!("/user/{name}/about.json?raw_json=1");
src/user.rs:170:	json(path, false).await.map(|res| {
src/user.rs:205:	// Get path
src/user.rs:206:	let path = format!("/user/{user_str}/{listing}.json?{}&raw_json=1", req.uri().query().unwrap_or_default(),);
src/user.rs:212:	let (posts, _) = Post::fetch(&path, false).await?;
scripts/update_hls_js.sh:4:LATEST_TAG=$(curl -s https://api.github.com/repos/video-dev/hls.js/releases/latest | jq -r '.tag_name')
scripts/update_hls_js.sh:11:LICENSE="// @license http://www.apache.org/licenses/LICENSE-2.0 Apache-2.0
scripts/update_hls_js.sh:12:// @source  https://github.com/video-dev/hls.js/tree/$LATEST_TAG"
scripts/update_hls_js.sh:16:curl -s https://cdn.jsdelivr.net/npm/hls.js@${LATEST_TAG}/dist/hls.min.js >> ../static/hls.min.js
src/duplicates.rs:25:#[template(path = "duplicates.html")]
src/duplicates.rs:56:	let path: String = format!("{}.json?{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default());
src/duplicates.rs:65:	match json(path, quarantined).await {
src/duplicates.rs:176:					let new_path: String = format!(
src/duplicates.rs:178:						req.uri().path(),
src/duplicates.rs:182:					match json(new_path, true).await {
src/instance_info.rs:57:		// https://github.com/ietf-wg-httpapi/mediatypes/blob/main/draft-ietf-httpapi-yaml-mediatypes.md#media-type-applicationyaml-application-yaml
src/instance_info.rs:229:#[template(path = "message.html")]
scripts/update_oauth_resources.sh:9:ios_version_list=$(curl -s "https://ipaarchive.com/app/usa/1064216828" | rg "(20\d{2}\.\d+.\d+) / (\d+)" --only-matching -r "Version \$1/Build \$2" | sort | uniq)
scripts/update_oauth_resources.sh:17:# Specify the filename as a variable
scripts/update_oauth_resources.sh:18:filename="src/oauth_resources.rs"
scripts/update_oauth_resources.sh:21:echo "// This file was generated by scripts/update_oauth_resources.sh" > "$filename"
scripts/update_oauth_resources.sh:22:echo "// Rerun scripts/update_oauth_resources.sh to update this file" >> "$filename"
scripts/update_oauth_resources.sh:23:echo "// Please do not edit manually" >> "$filename"
scripts/update_oauth_resources.sh:24:echo "// Filled in with real app versions" >> "$filename"
scripts/update_oauth_resources.sh:27:echo "pub const _IOS_APP_VERSION_LIST: &[&str; $ios_app_count] = &[" >> "$filename"
scripts/update_oauth_resources.sh:34:  echo "	\"$line\"," >> "$filename"
scripts/update_oauth_resources.sh:39:echo "];" >> "$filename"
scripts/update_oauth_resources.sh:42:page_1=$(curl -s "https://apkcombo.com/reddit/com.reddit.frontpage/old-versions/" | rg "<a class=\"ver-item\" href=\"(/reddit/com\.reddit\.frontpage/download/phone-20\d{2}\.\d+\.\d+-apk)\" rel=\"nofollow\">" -r "https://apkcombo.com\$1" | sort | uniq | sed 's/      //g')
scripts/update_oauth_resources.sh:44:page_2=$(curl -s "https://apkcombo.com/reddit/com.reddit.frontpage/old-versions?page=2" | rg "<a class=\"ver-item\" href=\"(/reddit/com\.reddit\.frontpage/download/phone-20\d{2}\.\d+\.\d+-apk)\" rel=\"nofollow\">" -r "https://apkcombo.com\$1" | sort | uniq | sed 's/      //g')
scripts/update_oauth_resources.sh:45:page_3=$(curl -s "https://apkcombo.com/reddit/com.reddit.frontpage/old-versions?page=3" | rg "<a class=\"ver-item\" href=\"(/reddit/com\.reddit\.frontpage/download/phone-20\d{2}\.\d+\.\d+-apk)\" rel=\"nofollow\">" -r "https://apkcombo.com\$1" | sort | uniq | sed 's/      //g')
scripts/update_oauth_resources.sh:46:page_4=$(curl -s "https://apkcombo.com/reddit/com.reddit.frontpage/old-versions?page=4" | rg "<a class=\"ver-item\" href=\"(/reddit/com\.reddit\.frontpage/download/phone-20\d{2}\.\d+\.\d+-apk)\" rel=\"nofollow\">" -r "https://apkcombo.com\$1" | sort | uniq | sed 's/      //g')
scripts/update_oauth_resources.sh:47:page_5=$(curl -s "https://apkcombo.com/reddit/com.reddit.frontpage/old-versions?page=5" | rg "<a class=\"ver-item\" href=\"(/reddit/com\.reddit\.frontpage/download/phone-20\d{2}\.\d+\.\d+-apk)\" rel=\"nofollow\">" -r "https://apkcombo.com\$1" | sort | uniq | sed 's/      //g')
scripts/update_oauth_resources.sh:66:echo "pub const ANDROID_APP_VERSION_LIST: &[&str; $android_count] = &[" >> "$filename"
scripts/update_oauth_resources.sh:76:  echo "	\"Version $version/Build $build\"," >> "$filename"
scripts/update_oauth_resources.sh:81:echo "];" >> "$filename"
scripts/update_oauth_resources.sh:84:table=$(curl -s "https://en.wikipedia.org/w/api.php?action=parse&page=IOS_17&prop=wikitext&section=31&format=json" | jq ".parse.wikitext.\"*\"" | rg "(17\.[\d\.]*)\\\n\|(\w*)\\\n\|" --only-matching -r "Version \$1 (Build \$2)")
scripts/update_oauth_resources.sh:92:echo "pub const _IOS_OS_VERSION_LIST: &[&str; $ios_count] = &[" >> "$filename"
scripts/update_oauth_resources.sh:99:  echo "	\"$line\"," >> "$filename"
scripts/update_oauth_resources.sh:104:echo "];" >> "$filename"
src/bin/redlib-desktop.rs:7:use std::path::{Path, PathBuf};
src/bin/redlib-desktop.rs:18:	let session_secret = ensure_session_secret(&cfg_dir)?;
src/bin/redlib-desktop.rs:22:	let url = format!("http://127.0.0.1:{port}/");
src/bin/redlib-desktop.rs:23:	let mut child = spawn_server(&cfg_dir, &session_secret, port)?;
src/bin/redlib-desktop.rs:42:fn spawn_server(cfg_dir: &Path, session_secret: &str, port: u16) -> Result<Child, Box<dyn std::error::Error>> {
src/bin/redlib-desktop.rs:51:		.env("REDLIB_SESSION_SECRET", session_secret)
src/bin/redlib-desktop.rs:56:	Ok(cmd.spawn()?)
src/bin/redlib-desktop.rs:106:fn ensure_session_secret(cfg_dir: &Path) -> io::Result<String> {
src/bin/redlib-desktop.rs:107:	let path = cfg_dir.join("session_secret");
src/bin/redlib-desktop.rs:108:	if let Ok(existing) = fs::read_to_string(&path) {
src/bin/redlib-desktop.rs:117:	let secret = hex_encode(&bytes);
src/bin/redlib-desktop.rs:118:	fs::write(&path, format!("{secret}\n"))?;
src/bin/redlib-desktop.rs:119:	Ok(secret)
src/bin/redlib-desktop.rs:123:	let path = cfg_dir.join("redlib.toml");
src/bin/redlib-desktop.rs:124:	if path.exists() {
src/bin/redlib-desktop.rs:132:	fs::write(path, contents)
src/bin/redlib-desktop.rs:158:		Command::new("cmd").args(["/C", "start", "", url]).spawn()?.wait()?;
src/bin/redlib-desktop.rs:163:		Command::new("open").arg(url).spawn()?.wait()?;
src/bin/redlib-desktop.rs:168:		Command::new("xdg-open").arg(url).spawn()?.wait()?;
src/post.rs:13:use cookie::Cookie;
src/post.rs:22:#[template(path = "post.html")]
src/post.rs:38:	// Build Reddit API path
src/post.rs:39:	let mut path: String = format!("{}.json?{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default());
src/post.rs:45:	let sort = param(&path, "sort").unwrap_or_else(|| {
src/post.rs:53:			path = format!("{}.json?{}&sort={}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default(), default_sort);
src/post.rs:66:	match json(path, quarantined).await {
src/post.rs:112:			let path_for_param = format!("?{}", req.uri().query().unwrap_or_default());
src/post.rs:113:			let reader_mode = param(&path_for_param, "reader").map(|s| s == "1" || s == "on").unwrap_or(false);
src/post.rs:140:/// POST /comment-collapse: body id=t1_xxx&action=collapse|expand — persist collapsed comment state in cookie.
src/post.rs:164:	response.insert_cookie(
src/post.rs:166:			.path("/")
src/post.rs:257:			"<div class=\"md\"><p>[removed] — <a href=\"https://{}{post_link}{id}\">view removed comment</a></p></div>",
src/search.rs:6:	get_filter_keywords, get_filters, get_read_ids, get_recent_searches, get_saved_searches, param, recent_searches_cookie_value, redirect,
src/search.rs:7:	saved_searches_cookie_value_after_save, saved_searches_cookie_value_after_unsave, setting, template, val, Post, Preferences, SavedSearch,
src/search.rs:15:use cookie::Cookie;
src/search.rs:51:#[template(path = "search.html")]
src/search.rs:82:	let uri_path = req.uri().path().replace("+", "%2B");
src/search.rs:83:	let path = format!("{}.json?{}{}&raw_json=1", uri_path, req.uri().query().unwrap_or_default(), nsfw_results);
src/search.rs:84:	let mut query = param(&path, "q").unwrap_or_default();
src/search.rs:88:		return Ok(redirect("/"));
src/search.rs:92:		return Ok(redirect(&format!("/{query}")));
src/search.rs:96:		return Ok(redirect(&format!("/r{}", &query[1..])));
src/search.rs:100:		return Ok(redirect(&format!("/user{}", &query[1..])));
src/search.rs:110:	let typed = param(&path, "type").unwrap_or_default();
src/search.rs:112:	let sort = param(&path, "sort").unwrap_or_else(|| "relevance".to_string());
src/search.rs:123:	let subreddits = if param(&path, "restrict_sr").is_none() {
src/search.rs:131:	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));
src/search.rs:144:				t: param(&path, "t").unwrap_or_default(),
src/search.rs:145:				before: param(&path, "after").unwrap_or_default(),
src/search.rs:147:				restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
src/search.rs:160:		match Post::fetch(&path, quarantined).await {
src/search.rs:180:						t: param(&path, "t").unwrap_or_default(),
src/search.rs:181:						before: param(&path, "after").unwrap_or_default(),
src/search.rs:183:						restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
src/search.rs:196:					let val = recent_searches_cookie_value(&req, &query);
src/search.rs:198:						res.insert_cookie(
src/search.rs:200:								.path("/")
src/search.rs:223:	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit={limit}", q.replace(' ', "+"));
src/search.rs:226:	json(subreddit_search_path, false).await.unwrap_or_default()["data"]["children"]
src/search.rs:255:	let cookie_val = saved_searches_cookie_value_after_save(&req2, &name, &q);
src/search.rs:256:	let redirect_url = if q.is_empty() {
src/search.rs:261:	let mut res = redirect(&redirect_url);
src/search.rs:262:	res.insert_cookie(
src/search.rs:263:		Cookie::build(("saved_searches", cookie_val))
src/search.rs:264:			.path("/")
src/search.rs:279:	let cookie_val = saved_searches_cookie_value_after_unsave(&req2, &q);
src/search.rs:280:	let redirect_url = if q.is_empty() {
src/search.rs:285:	let mut res = redirect(&redirect_url);
src/search.rs:286:	res.insert_cookie(
src/search.rs:287:		Cookie::build(("saved_searches", cookie_val))
src/search.rs:288:			.path("/")
src/settings.rs:8:use crate::utils::{deflate_decompress, redirect, template, Preferences};
src/settings.rs:10:use cookie::Cookie;
src/settings.rs:19:#[template(path = "settings.html")]
src/settings.rs:60:/// Retrieve cookies from request "Cookie" header
src/settings.rs:69:/// Set cookies using response "Set-Cookie" header
src/settings.rs:74:	// Grab existing cookies
src/settings.rs:75:	let _cookies: Vec<Cookie<'_>> = parts
src/settings.rs:94:	let mut response = redirect("/settings");
src/settings.rs:98:			Some(value) => response.insert_cookie(
src/settings.rs:100:					.path("/")
src/settings.rs:105:			None => response.remove_cookie(name.to_string()),
src/settings.rs:112:fn set_cookies_method(req: Request<Body>, remove_cookies: bool) -> Response<Body> {
src/settings.rs:116:	// Grab existing cookies
src/settings.rs:117:	let _cookies: Vec<Cookie<'_>> = parts
src/settings.rs:128:	let path = match form.get("redirect") {
src/settings.rs:140:	let mut response = redirect(&path);
src/settings.rs:144:			Some(value) => response.insert_cookie(
src/settings.rs:146:					.path("/")
src/settings.rs:152:				if remove_cookies {
src/settings.rs:153:					response.remove_cookie(name.to_string());
src/settings.rs:163:	// We can't search through the cookies directly like in subreddit.rs, so instead we have to make a string out of the request's headers to search through
src/settings.rs:164:	let cookies_string = parts
src/settings.rs:166:		.get("cookie")
src/settings.rs:170:	// If there are subscriptions to restore set them and delete any old subscriptions cookies, otherwise delete them all
src/settings.rs:174:		// Start at 0 to keep track of what number we need to start deleting old subscription cookies from
src/settings.rs:177:		// Starting at 0 so we handle the subscription cookie without a number first
src/settings.rs:179:			let subscriptions_cookie = if subscriptions_number == 0 {
src/settings.rs:185:			response.insert_cookie(
src/settings.rs:186:				Cookie::build((subscriptions_cookie, list))
src/settings.rs:187:					.path("/")
src/settings.rs:196:		// While subscriptionsNUMBER= is in the string of cookies add a response removing that cookie
src/settings.rs:197:		while cookies_string.contains(&format!("subscriptions{subscriptions_number_to_delete_from}=")) {
src/settings.rs:198:			// Remove that subscriptions cookie
src/settings.rs:199:			response.remove_cookie(format!("subscriptions{subscriptions_number_to_delete_from}"));
src/settings.rs:201:			// Increment subscriptions cookie number
src/settings.rs:205:		// Remove unnumbered subscriptions cookie
src/settings.rs:206:		response.remove_cookie("subscriptions".to_string());
src/settings.rs:208:		// Starts at one to deal with the first numbered subscription cookie and onwards
src/settings.rs:211:		// While subscriptionsNUMBER= is in the string of cookies add a response removing that cookie
src/settings.rs:212:		while cookies_string.contains(&format!("subscriptions{subscriptions_number_to_delete_from}=")) {
src/settings.rs:213:			// Remove that subscriptions cookie
src/settings.rs:214:			response.remove_cookie(format!("subscriptions{subscriptions_number_to_delete_from}"));
src/settings.rs:216:			// Increment subscriptions cookie number
src/settings.rs:221:	// If there are filters to restore set them and delete any old filters cookies, otherwise delete them all
src/settings.rs:225:		// Start at 0 to keep track of what number we need to start deleting old subscription cookies from
src/settings.rs:228:		// Starting at 0 so we handle the subscription cookie without a number first
src/settings.rs:230:			let filters_cookie = if filters_number == 0 {
src/settings.rs:236:			response.insert_cookie(
src/settings.rs:237:				Cookie::build((filters_cookie, list))
src/settings.rs:238:					.path("/")
src/settings.rs:247:		// While filtersNUMBER= is in the string of cookies add a response removing that cookie
src/settings.rs:248:		while cookies_string.contains(&format!("filters{filters_number_to_delete_from}=")) {
src/settings.rs:249:			// Remove that filters cookie
src/settings.rs:250:			response.remove_cookie(format!("filters{filters_number_to_delete_from}"));
src/settings.rs:252:			// Increment filters cookie number
src/settings.rs:256:		// Remove unnumbered filters cookie
src/settings.rs:257:		response.remove_cookie("filters".to_string());
src/settings.rs:259:		// Starts at one to deal with the first numbered subscription cookie and onwards
src/settings.rs:262:		// While filtersNUMBER= is in the string of cookies add a response removing that cookie
src/settings.rs:263:		while cookies_string.contains(&format!("filters{filters_number_to_delete_from}=")) {
src/settings.rs:264:			// Remove that sfilters cookie
src/settings.rs:265:			response.remove_cookie(format!("filters{filters_number_to_delete_from}"));
src/settings.rs:267:			// Increment filters cookie number
src/settings.rs:275:/// Set cookies using response "Set-Cookie" header
src/settings.rs:277:	Ok(set_cookies_method(req, true))
src/settings.rs:281:	Ok(set_cookies_method(req, false))
src/settings.rs:313:	Ok(redirect(&url))
src/settings.rs:324:			.header("Content-Disposition", "attachment; filename=\"redlib-settings.json\"")
src/settings.rs:335:	out.push_str("# Import manually by mapping values into cookies or use /settings restore URL/JSON import.\n");
src/settings.rs:337:		if key == "available_themes" || key == "logged_in" || key == "username" || key == "csrf_token" {
src/settings.rs:349:			.header("Content-Disposition", "attachment; filename=\"redlib-user-prefs.env\"")
src/settings.rs:355:/// POST /settings/import-json: body is JSON preferences (or form field "json"). Redirects to restore to set cookies.
src/settings.rs:379:	Ok(redirect(&url))
src/inbox.rs:12:use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
src/inbox.rs:14:use crate::utils::{param, redirect, template, Preferences};
src/inbox.rs:72:#[template(path = "inbox.html")]
src/inbox.rs:82:#[template(path = "inbox_compose.html")]
src/inbox.rs:98:		return Ok(redirect("/login"));
src/inbox.rs:102:	let path = match tab.as_str() {
src/inbox.rs:108:	let path = format!("{}?limit=25&raw_json=1&{}", path, query);
src/inbox.rs:109:	let (json, session_updated) = authed_json(path, false, &auth).await?;
src/inbox.rs:119:	if let Some(s) = session_updated {
src/inbox.rs:120:		update_session_cookie(&mut res, &s);
src/inbox.rs:129:		return Ok(redirect("/login"));
src/inbox.rs:154:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/inbox.rs:155:	validate_csrf_token(&auth, submitted_csrf)?;
src/inbox.rs:174:	let (value, session_updated) = authed_post("/api/compose".to_string(), body_str, &auth).await?;
src/inbox.rs:193:			if let Some(s) = session_updated {
src/inbox.rs:194:				update_session_cookie(&mut res, &s);
src/inbox.rs:199:	let mut res = redirect("/inbox");
src/inbox.rs:200:	if let Some(s) = session_updated {
src/inbox.rs:201:		update_session_cookie(&mut res, &s);
src/inbox.rs:226:	let (_, session_updated) = authed_post("/api/read_message".to_string(), body_str, &auth).await?;
src/inbox.rs:228:	let mut res = redirect("/inbox");
src/inbox.rs:229:	if let Some(s) = session_updated {
src/inbox.rs:230:		update_session_cookie(&mut res, &s);
src/inbox.rs:242:	let (_, session_updated) = authed_post("/api/read_all_messages".to_string(), String::new(), &auth).await?;
src/inbox.rs:244:	let mut res = redirect("/inbox");
src/inbox.rs:245:	if let Some(s) = session_updated {
src/inbox.rs:246:		update_session_cookie(&mut res, &s);
src/token_import/firefox.rs:3:use std::path::{Path, PathBuf};
src/token_import/firefox.rs:27:		let path = entry.path();
src/token_import/firefox.rs:28:		if !path.is_dir() {
src/token_import/firefox.rs:31:		let cookies = path.join("cookies.sqlite");
src/token_import/firefox.rs:32:		if !cookies.is_file() {
src/token_import/firefox.rs:35:		let profile_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("profile");
src/token_import/firefox.rs:42:			path,
src/token_import/firefox.rs:49:pub fn import_token(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<ImportedBrowserSession, String> {
src/token_import/firefox.rs:50:	let profile = resolve_profile(browser, profile_id, profile_path)?;
src/token_import/firefox.rs:51:	let db_path = profile.join("cookies.sqlite");
src/token_import/firefox.rs:52:	if !db_path.is_file() {
src/token_import/firefox.rs:53:		return Err(format!("cookies.sqlite not found in {}", profile.display()));
src/token_import/firefox.rs:55:	let tmp = copy_to_temp(&db_path)?;
src/token_import/firefox.rs:57:	let conn = Connection::open(&tmp).map_err(|e| format!("Failed to open cookie DB: {e}"))?;
src/token_import/firefox.rs:58:	let token = read_firefox_token(&conn)?;
src/token_import/firefox.rs:61:		bearer_token: token,
src/token_import/firefox.rs:73:fn copy_to_temp(path: &Path) -> Result<PathBuf, String> {
src/token_import/firefox.rs:74:	let tmp = env::temp_dir().join(format!("redlib-firefox-cookies-{}.sqlite", Uuid::new_v4()));
src/token_import/firefox.rs:75:	fs::copy(path, &tmp).map_err(|e| format!("Failed to copy cookie DB: {e}"))?;
src/token_import/firefox.rs:79:fn read_firefox_token(conn: &Connection) -> Result<String, String> {
src/token_import/firefox.rs:83:			 FROM moz_cookies
src/token_import/firefox.rs:84:			 WHERE name = 'token_v2'
src/token_import/firefox.rs:89:		.map_err(|e| format!("Failed to prepare cookie query: {e}"))?;
src/token_import/firefox.rs:93:		.map_err(|_| "No reddit token_v2 cookie found in selected Firefox profile".to_string())
src/token_import/firefox.rs:96:fn resolve_profile(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<PathBuf, String> {
src/token_import/firefox.rs:97:	if let Some(path) = profile_path.map(str::trim).filter(|s| !s.is_empty()) {
src/token_import/firefox.rs:98:		return Ok(PathBuf::from(path));
src/token_import/firefox.rs:104:				return Ok(p.path);
src/token_import/firefox.rs:113:		.map(|p| p.path)
src/token_import/firefox.rs:114:		.ok_or_else(|| format!("No {} profiles with cookies.sqlite were found", display_browser(browser)))
scripts/extract_firefox_token.py:3:extract_firefox_token.py — pull a Reddit bearer token (and matching User-Agent)
scripts/extract_firefox_token.py:7:    python3 extract_firefox_token.py [SSH_HOST] [OPTIONS]
scripts/extract_firefox_token.py:11:    python3 extract_firefox_token.py user@192.168.1.50
scripts/extract_firefox_token.py:13:    # Specify a profile path explicitly:
scripts/extract_firefox_token.py:14:    python3 extract_firefox_token.py user@192.168.1.50 --profile ~/.librewolf/default/
scripts/extract_firefox_token.py:17:    python3 extract_firefox_token.py --local
scripts/extract_firefox_token.py:19:    # Emit shell export lines ready to paste or eval:
scripts/extract_firefox_token.py:20:    python3 extract_firefox_token.py user@192.168.1.50 --export
scripts/extract_firefox_token.py:21:    eval "$(python3 extract_firefox_token.py user@192.168.1.50 --export)"
scripts/extract_firefox_token.py:24:    REDLIB_RAW_TOKEN=<bearer_token>
scripts/extract_firefox_token.py:41:from pathlib import Path
scripts/extract_firefox_token.py:52:def decode_token_v2(raw: str) -> str | None:
scripts/extract_firefox_token.py:54:    Attempt to extract a bearer token from a Reddit token_v2 JWT.
scripts/extract_firefox_token.py:56:    Reddit's token_v2 cookie is a JWT whose payload may contain one of:
scripts/extract_firefox_token.py:57:      - "access_token"
scripts/extract_firefox_token.py:58:      - "token"
scripts/extract_firefox_token.py:61:    Returns the bearer token string, or None if decoding fails.
scripts/extract_firefox_token.py:69:        for field in ("access_token", "token", "accessToken"):
scripts/extract_firefox_token.py:117:                    profile_path = (p / rel) if not rel.startswith("/") else Path(rel)
scripts/extract_firefox_token.py:118:                    if profile_path.is_dir():
scripts/extract_firefox_token.py:119:                        candidates.append(profile_path)
scripts/extract_firefox_token.py:121:            # Fallback: any subdir containing cookies.sqlite
scripts/extract_firefox_token.py:123:                if subdir.is_dir() and (subdir / "cookies.sqlite").exists():
scripts/extract_firefox_token.py:142:# ── cookie extraction ─────────────────────────────────────────────────────────
scripts/extract_firefox_token.py:144:def extract_reddit_token_from_db(cookies_db: Path) -> str | None:
scripts/extract_firefox_token.py:146:    Query a Firefox cookies.sqlite for Reddit's token_v2 cookie value.
scripts/extract_firefox_token.py:147:    Returns the raw cookie value string, or None if not found.
scripts/extract_firefox_token.py:149:    if not cookies_db.exists():
scripts/extract_firefox_token.py:154:        shutil.copy2(cookies_db, tmp.name)
scripts/extract_firefox_token.py:159:            cur.execute(
scripts/extract_firefox_token.py:160:                "SELECT value FROM moz_cookies "
scripts/extract_firefox_token.py:162:                "  AND name = 'token_v2' "
scripts/extract_firefox_token.py:171:        print(f"  [warn] Could not read {cookies_db}: {e}", file=sys.stderr)
scripts/extract_firefox_token.py:178:    """Run a shell command on a remote host via ssh(1) and return stdout."""
scripts/extract_firefox_token.py:189:def scp_file(host: str, remote_path: str, local_dest: Path) -> None:
scripts/extract_firefox_token.py:193:         f"{host}:{remote_path}", str(local_dest)],
scripts/extract_firefox_token.py:244:    Extract token and UA from the local machine.
scripts/extract_firefox_token.py:245:    Returns (bearer_token, user_agent) — either may be None on failure.
scripts/extract_firefox_token.py:255:    token_raw = None
scripts/extract_firefox_token.py:257:        cookies_db = profile / "cookies.sqlite"
scripts/extract_firefox_token.py:258:        val = extract_reddit_token_from_db(cookies_db)
scripts/extract_firefox_token.py:260:            token_raw = val
scripts/extract_firefox_token.py:261:            print(f"  Found token_v2 in: {cookies_db}", file=sys.stderr)
scripts/extract_firefox_token.py:264:    if not token_raw:
scripts/extract_firefox_token.py:265:        print("Reddit token_v2 cookie not found in any profile.", file=sys.stderr)
scripts/extract_firefox_token.py:268:    bearer = decode_token_v2(token_raw)
scripts/extract_firefox_token.py:270:        print("  Could not decode token_v2 as JWT; treating raw value as bearer token.", file=sys.stderr)
scripts/extract_firefox_token.py:271:        bearer = token_raw
scripts/extract_firefox_token.py:282:    Extract token and UA from a remote machine over SSH.
scripts/extract_firefox_token.py:283:    Returns (bearer_token, user_agent) — either may be None on failure.
scripts/extract_firefox_token.py:298:    token_raw = None
scripts/extract_firefox_token.py:300:        remote_db = f"{remote_profile}/cookies.sqlite"
scripts/extract_firefox_token.py:302:            tmp_path = Path(tmp.name)
scripts/extract_firefox_token.py:305:            scp_file(host, remote_db, tmp_path)
scripts/extract_firefox_token.py:306:            val = extract_reddit_token_from_db(tmp_path)
scripts/extract_firefox_token.py:308:                token_raw = val
scripts/extract_firefox_token.py:309:                print(f"  Found token_v2 in: {remote_db}", file=sys.stderr)
scripts/extract_firefox_token.py:314:            if tmp_path.exists():
scripts/extract_firefox_token.py:315:                os.unlink(tmp_path)
scripts/extract_firefox_token.py:317:    if not token_raw:
scripts/extract_firefox_token.py:318:        print("Reddit token_v2 cookie not found in any remote profile.", file=sys.stderr)
scripts/extract_firefox_token.py:321:    bearer = decode_token_v2(token_raw)
scripts/extract_firefox_token.py:323:        print("  Could not decode token_v2 as JWT; treating raw value as bearer token.", file=sys.stderr)
scripts/extract_firefox_token.py:324:        bearer = token_raw
scripts/extract_firefox_token.py:335:        description="Extract Reddit bearer token from a Firefox/LibreWolf install (local or SSH).",
scripts/extract_firefox_token.py:359:        help="Output shell export lines (eval-safe). Suitable for: eval \"$(...)\"",
scripts/extract_firefox_token.py:362:        "--token-only",
scripts/extract_firefox_token.py:364:        help="Print only the bearer token, nothing else.",
scripts/extract_firefox_token.py:369:        token, ua = extract_local(args.profile)
scripts/extract_firefox_token.py:371:        token, ua = extract_remote(args.host, args.profile)
scripts/extract_firefox_token.py:373:    if not token:
scripts/extract_firefox_token.py:374:        print("ERROR: Could not extract a Reddit bearer token.", file=sys.stderr)
scripts/extract_firefox_token.py:377:    if args.token_only:
scripts/extract_firefox_token.py:378:        print(token)
scripts/extract_firefox_token.py:382:        print(f"export REDLIB_RAW_TOKEN='{token}'")
scripts/extract_firefox_token.py:386:        print(f"REDLIB_RAW_TOKEN={token}")
src/server.rs:6:use cookie::Cookie;
src/server.rs:140:	/// in [RFC 7231](https://datatracker.ietf.org/doc/html/rfc7231#section-5.3.4)
src/server.rs:173:	path: String,
src/server.rs:200:	fn cookies(&self) -> Vec<Cookie<'_>>;
src/server.rs:201:	fn cookie(&self, name: &str) -> Option<Cookie<'_>>;
src/server.rs:205:	fn cookies(&self) -> Vec<Cookie<'_>>;
src/server.rs:206:	fn insert_cookie(&mut self, cookie: Cookie<'_>);
src/server.rs:207:	fn remove_cookie(&mut self, name: String);
src/server.rs:227:	fn cookies(&self) -> Vec<Cookie<'_>> {
src/server.rs:233:				.map(|cookie| Cookie::parse(cookie).unwrap_or_else(|_| Cookie::from("")))
src/server.rs:238:	fn cookie(&self, name: &str) -> Option<Cookie<'_>> {
src/server.rs:239:		self.cookies().into_iter().find(|c| c.name() == name)
src/server.rs:244:	fn cookies(&self) -> Vec<Cookie<'_>> {
src/server.rs:250:				.map(|cookie| Cookie::parse(cookie).unwrap_or_else(|_| Cookie::from("")))
src/server.rs:255:	fn insert_cookie(&mut self, cookie: Cookie<'_>) {
src/server.rs:256:		if let Ok(val) = header::HeaderValue::from_str(&cookie.to_string()) {
src/server.rs:261:	fn remove_cookie(&mut self, name: String) {
src/server.rs:262:		let removal_cookie = Cookie::build(name).path("/").http_only(true).expires(OffsetDateTime::now_utc());
src/server.rs:263:		if let Ok(val) = header::HeaderValue::from_str(&removal_cookie.to_string()) {
src/server.rs:271:		self.router.add(&format!("/{}{}", method.as_str(), self.path), dest);
src/server.rs:300:	pub fn at(&mut self, path: &str) -> Route<'_> {
src/server.rs:302:			path: path.to_owned(),
src/server.rs:340:					let mut path = req.uri().path().replace("//", "/").replace("%2F", "/");
src/server.rs:342:					// Strip reverse-proxy path prefix (e.g. X-Forwarded-Prefix: /redlibe)
src/server.rs:343:					if let Some(prefix) = req.headers().get("x-forwarded-prefix").and_then(|v| v.to_str().ok()) {
src/server.rs:347:							if path == with_slash || path.starts_with(&format!("{with_slash}/")) {
src/server.rs:348:								if let Some(stripped) = path.as_str().strip_prefix(&with_slash) {
src/server.rs:349:									path = if stripped.is_empty() {
src/server.rs:362:					if path != "/" && path.ends_with('/') {
src/server.rs:363:						path.pop();
src/server.rs:372:					// Match the visited path with an added route
src/server.rs:373:					match router.recognize(&format!("/{}{}", method.as_str(), path)) {
src/server.rs:374:						// If a route was configured for this path
src/server.rs:465:/// This function will honor the [q-value](https://developer.mozilla.org/en-US/docs/Glossary/Quality_values)
src/server.rs:470:/// Here are [examples](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Encoding#examples)
src/server.rs:729:// execute again.
src/token_import/mod.rs:1:use std::path::PathBuf;
src/token_import/mod.rs:8:	pub bearer_token: String,
src/token_import/mod.rs:17:	pub path: PathBuf,
src/token_import/mod.rs:30:pub fn import_local(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<ImportedBrowserSession, String> {
src/token_import/mod.rs:32:		"firefox" | "librewolf" => firefox::import_token(browser, profile_id, profile_path),
src/token_import/mod.rs:33:		"chrome" | "edge" => chromium::import_token(browser, profile_id, profile_path),
src/oauth.rs:16:const AUTH_ENDPOINT: &str = "https://www.reddit.com";
src/oauth.rs:23:	pub token: String,
src/oauth.rs:77:			warn!("[⚠️] Skipping anonymous OAuth client creation; account/session auth remains available");
src/oauth.rs:137:			headers_map.insert("Authorization".to_owned(), format!("Bearer {}", response.token));
src/oauth.rs:173:pub async fn token_daemon() {
src/oauth.rs:174:	// Monitor for refreshing token
src/oauth.rs:182:		info!("[⏳] Waiting for {duration:?} seconds before refreshing OAuth token...");
src/oauth.rs:186:		info!("[⌛] {duration:?} Elapsed! Refreshing OAuth token...");
src/oauth.rs:188:		// Refresh token - in its own scope
src/oauth.rs:190:			force_refresh_token().await;
src/oauth.rs:195:pub async fn force_refresh_token() {
src/oauth.rs:197:		trace!("Skipping refresh token roll over, already in progress");
src/oauth.rs:201:	trace!("Rolling over refresh token. Current rate limit: {}", OAUTH_RATELIMIT_REMAINING.load(Ordering::SeqCst));
src/oauth.rs:234:		// Construct URL for OAuth token
src/oauth.rs:235:		let url = format!("{AUTH_ENDPOINT}/auth/v2/oauth/access-token/loid");
src/oauth.rs:242:		// Set up HTTP Basic Auth - basically just the const OAuth ID's with no password,
src/oauth.rs:243:		// Base64-encoded. https://en.wikipedia.org/wiki/Basic_access_authentication
src/oauth.rs:258:		trace!("Sending token request...\n\n{request:?}");
src/oauth.rs:267:		// Parse headers - loid header _should_ be saved sent on subsequent token refreshes.
src/oauth.rs:271:		// and really only as privacy-concerning as the OAuth token itself.
src/oauth.rs:276:		// Same with x-reddit-session
src/oauth.rs:277:		if let Some(header) = resp.headers().get("x-reddit-session").and_then(|h| h.to_str().ok()) {
src/oauth.rs:278:			self.additional_headers.insert("x-reddit-session".to_owned(), header.to_string());
src/oauth.rs:289:		// Save token and expiry
src/oauth.rs:290:		let token = json
src/oauth.rs:291:			.get("access_token")
src/oauth.rs:292:			.ok_or_else(|| AuthError::Field((json.clone(), "access_token")))?
src/oauth.rs:294:			.ok_or_else(|| AuthError::Field((json.clone(), "access_token: as_str")))?
src/oauth.rs:302:		info!("[✅] Success - Retrieved token \"{}...\", expires in {}", &token[..32], expires_in);
src/oauth.rs:305:			token,
src/oauth.rs:353:		// Construct URL for OAuth token
src/oauth.rs:354:		let url = "https://www.reddit.com/api/v1/access_token";
src/oauth.rs:375:		trace!("Sending GenericWebAuth token request...\n\n{request:?}");
src/oauth.rs:384:		// Parse headers - loid header _should_ be saved sent on subsequent token refreshes.
src/oauth.rs:388:		// and really only as privacy-concerning as the OAuth token itself.
src/oauth.rs:393:		// Same with x-reddit-session
src/oauth.rs:394:		if let Some(header) = resp.headers().get("x-reddit-session").and_then(|h| h.to_str().ok()) {
src/oauth.rs:395:			self.additional_headers.insert("x-reddit-session".to_owned(), header.to_string());
src/oauth.rs:406:		// Parse response - access_token, token_type, device_id, expires_in, scope
src/oauth.rs:407:		let token = json
src/oauth.rs:408:			.get("access_token")
src/oauth.rs:409:			.ok_or_else(|| AuthError::Field((json.clone(), "access_token")))?
src/oauth.rs:411:			.ok_or_else(|| AuthError::Field((json.clone(), "access_token: as_str")))?
src/oauth.rs:420:			"[✅] GenericWebAuth success - Retrieved token \"{}...\", expires in {}",
src/oauth.rs:421:			&token[..32.min(token.len())],
src/oauth.rs:426:		self.additional_headers.insert("Origin".to_owned(), "https://www.reddit.com".to_owned());
src/oauth.rs:430:			token,
src/oauth.rs:466:		// Android-specific headers are kept so the token acquisition request looks
src/oauth.rs:467:		// like a normal Android client session.
src/oauth.rs:489:		// See https://github.com/redlib-org/redlib/issues/8
src/oauth.rs:505:	assert!(!response.token.is_empty());
src/oauth.rs:518:	assert!(!response.token.is_empty());
src/oauth.rs:531:	force_refresh_token().await;
src/oauth.rs:535:async fn test_oauth_token_exists() {
src/lib.rs:22:pub mod token_import;
src/auth.rs:4://!   1. `REDLIB_RAW_TOKEN` — raw bearer token from environment (headless/script use)
src/auth.rs:5://!   2. `rl_session` cookie — real Reddit OAuth session (user login flow)
src/auth.rs:8://! Session cookies are encrypted with AES-256-GCM. The key is derived via
src/auth.rs:23:use cookie::{Cookie, SameSite};
src/auth.rs:36:use crate::token_import;
src/auth.rs:37:use crate::utils::{redirect, template, Preferences};
src/auth.rs:42:#[template(path = "login.html")]
src/auth.rs:62:/// Holds path to a temporary private key file; removes it on drop.
src/auth.rs:63:struct TempKeyFile(std::path::PathBuf);
src/auth.rs:75:		Using a random ephemeral key — all sessions will be invalidated on restart. \
src/auth.rs:76:		Set REDLIB_SESSION_SECRET to a 32+ byte random string for persistent sessions."
src/auth.rs:84:pub fn session_cookie_name() -> &'static str {
src/auth.rs:85:	if secure_cookies() {
src/auth.rs:86:		"__Host-rl_session"
src/auth.rs:88:		"rl_session"
src/auth.rs:93:pub fn active_session_cookie_name() -> &'static str {
src/auth.rs:94:	if secure_cookies() {
src/auth.rs:101:pub fn csrf_cookie_name() -> &'static str {
src/auth.rs:102:	if secure_cookies() {
src/auth.rs:109:pub fn subscriptions_cookie_name() -> &'static str {
src/auth.rs:110:	if secure_cookies() {
src/auth.rs:122:/// Serializable session payload, stored AES-256-GCM encrypted in `rl_session` cookie.
src/auth.rs:125:	/// Reddit OAuth access token (short-lived, ~1 hour).
src/auth.rs:126:	pub access_token: String,
src/auth.rs:127:	/// Reddit OAuth refresh token (long-lived; used to get new access tokens).
src/auth.rs:128:	pub refresh_token: String,
src/auth.rs:131:	/// Unix timestamp at which the access token expires.
src/auth.rs:133:	/// Per-session CSRF token embedded in HTML forms to prevent CSRF attacks.
src/auth.rs:134:	pub csrf_token: String,
src/auth.rs:137:/// Serializable session vault — holds multiple user sessions for account switching.
src/auth.rs:138:/// Stored AES-256-GCM encrypted in `rl_session` cookie.
src/auth.rs:141:	/// List of session data for all accounts.
src/auth.rs:142:	pub sessions: Vec<SessionData>,
src/auth.rs:147:		Self { sessions: Vec::new() }
src/auth.rs:150:	pub fn add(&mut self, session: SessionData) {
src/auth.rs:151:		// Remove any existing session for the same username
src/auth.rs:152:		self.sessions.retain(|s| s.username != session.username);
src/auth.rs:153:		self.sessions.push(session);
src/auth.rs:157:		self.sessions.retain(|s| s.username != username);
src/auth.rs:161:		self.sessions.iter().find(|s| s.username == username)
src/auth.rs:164:	pub fn active_session(&self) -> Option<&SessionData> {
src/auth.rs:165:		self.sessions.first()
src/auth.rs:169:		self.sessions.iter().map(|s| s.username.clone()).collect()
src/auth.rs:187:	/// A logged-in Reddit user with a real OAuth access token.
src/auth.rs:189:	/// A raw bearer token provided via `REDLIB_RAW_TOKEN` env var.
src/auth.rs:197:	/// 1. `REDLIB_RAW_TOKEN` — direct bearer token (highest priority)
src/auth.rs:198:	/// 2. `REDLIB_BROWSER_TOKEN` — browser-exported `token_v2` JWT, decoded to bearer
src/auth.rs:199:	/// 3. `rl_session` cookie — encrypted OAuth session vault
src/auth.rs:202:		// Priority 1: raw bearer token from config
src/auth.rs:203:		if let Some(token) = CONFIG.raw_token.clone().filter(|s| !s.is_empty()) {
src/auth.rs:204:			return AuthContext::RawBearer(token);
src/auth.rs:207:		// Priority 2: browser-exported token (token_v2 JWT or raw bearer)
src/auth.rs:208:		if let Some(raw) = CONFIG.browser_token.clone().filter(|s| !s.is_empty()) {
src/auth.rs:209:			let bearer = decode_browser_token(&raw).unwrap_or(raw);
src/auth.rs:213:		// Priority 3: encrypted session vault cookie
src/auth.rs:214:		if let Some(cookie) = req.cookie(session_cookie_name()) {
src/auth.rs:215:			if let Some(vault) = decrypt_vault(cookie.value()) {
src/auth.rs:216:				// Find active session: first by rl_active cookie, then first in vault
src/auth.rs:217:				let active_username = req.cookie(active_session_cookie_name()).map(|c| c.value().to_string());
src/auth.rs:222:					if let Some(session) = vault.get(&username) {
src/auth.rs:223:						if session.expires_at > now - 30 {
src/auth.rs:224:							return AuthContext::UserSession(session.clone());
src/auth.rs:229:				// Fall back to first valid session
src/auth.rs:230:				for session in &vault.sessions {
src/auth.rs:231:					if session.expires_at > now - 30 {
src/auth.rs:232:						return AuthContext::UserSession(session.clone());
src/auth.rs:242:	/// Return all usernames in the session vault (for account switching UI).
src/auth.rs:244:		if let Some(cookie) = req.cookie(session_cookie_name()) {
src/auth.rs:245:			if let Some(vault) = decrypt_vault(cookie.value()) {
src/auth.rs:252:	/// Return the active username from the cookie (for display).
src/auth.rs:254:		if let Some(cookie) = req.cookie(session_cookie_name()) {
src/auth.rs:255:			if let Some(vault) = decrypt_vault(cookie.value()) {
src/auth.rs:256:				// Check active cookie first
src/auth.rs:257:				if let Some(active) = req.cookie(active_session_cookie_name()) {
src/auth.rs:263:				// Default to first session
src/auth.rs:264:				return vault.active_session().map(|s| s.username.clone());
src/auth.rs:270:	/// Return the bearer token for authenticated Reddit API calls, if any.
src/auth.rs:271:	pub fn bearer_token(&self) -> Option<&str> {
src/auth.rs:273:			AuthContext::UserSession(s) => Some(&s.access_token),
src/auth.rs:287:	/// Return the CSRF token for form embedding. Empty string when anonymous.
src/auth.rs:288:	pub fn csrf_token(&self) -> String {
src/auth.rs:290:			AuthContext::UserSession(s) => s.csrf_token.clone(),
src/auth.rs:295:	/// Whether there is an active authenticated session (user or raw token).
src/auth.rs:300:	/// Return a reference to the session data when the context is a user session.
src/auth.rs:301:	/// Used by the client layer to refresh the access token on 401 and return updated session to set cookie.
src/auth.rs:302:	pub fn session_data(&self) -> Option<&SessionData> {
src/auth.rs:315:/// random key (sessions will not survive restarts — a warning is printed once).
src/auth.rs:316:fn session_key() -> [u8; 32] {
src/auth.rs:317:	match CONFIG.session_secret.as_deref().filter(|s| !s.is_empty()) {
src/auth.rs:318:		Some(secret) => {
src/auth.rs:319:			let hk = Hkdf::<Sha256>::new(None, secret.as_bytes());
src/auth.rs:321:			hk.expand(b"redlib-session-v1", &mut key).expect("HKDF expand failed");
src/auth.rs:328:/// Attempt to decode a browser-exported Reddit `token_v2` JWT and extract
src/auth.rs:329:/// the bearer token from the payload.
src/auth.rs:331:/// Reddit's `token_v2` cookie is a JWT whose payload contains an `access_token`
src/auth.rs:332:/// field (or `token` in some client builds). If the input is not a valid JWT,
src/auth.rs:333:/// it is returned as-is so callers can treat it as a raw bearer token.
src/auth.rs:334:pub fn decode_browser_token(raw: &str) -> Option<String> {
src/auth.rs:345:	for field in &["access_token", "token", "accessToken"] {
src/auth.rs:353:/// Encrypt `SessionData` to a base64 string suitable for a cookie value.
src/auth.rs:356:pub fn encrypt_session(data: &SessionData) -> Option<String> {
src/auth.rs:357:	let key_bytes = session_key();
src/auth.rs:373:/// Decrypt and deserialize a base64 session cookie value.
src/auth.rs:375:pub fn decrypt_session(encoded: &str) -> Option<SessionData> {
src/auth.rs:381:	let key_bytes = session_key();
src/auth.rs:388:/// Encrypt `SessionVault` to a base64 string suitable for a cookie value.
src/auth.rs:390:	let key_bytes = session_key();
src/auth.rs:406:/// Decrypt and deserialize a base64 session vault cookie value.
src/auth.rs:413:	let key_bytes = session_key();
src/auth.rs:422:/// Add a new session to the vault and return a response with updated cookies.
src/auth.rs:423:/// Sets the new session as active.
src/auth.rs:424:pub fn add_session_to_vault(session: SessionData) -> Result<Response<Body>, String> {
src/auth.rs:426:	vault.add(session.clone());
src/auth.rs:428:	let encrypted = encrypt_vault(&vault).ok_or("Failed to encrypt session vault")?;
src/auth.rs:429:	let mut response = redirect(AUTH_LANDING_PATH);
src/auth.rs:430:	response.insert_cookie(
src/auth.rs:431:		Cookie::build((session_cookie_name(), encrypted))
src/auth.rs:432:			.path("/")
src/auth.rs:434:			.secure(secure_cookies())
src/auth.rs:439:	// Set active session to the new username
src/auth.rs:440:	response.insert_cookie(
src/auth.rs:441:		Cookie::build((active_session_cookie_name(), session.username.clone()))
src/auth.rs:442:			.path("/")
src/auth.rs:444:			.secure(secure_cookies())
src/auth.rs:453:pub fn switch_active_session(username: &str, vault_cookie: Option<&str>) -> Result<Response<Body>, String> {
src/auth.rs:455:	if let Some(cookie_val) = vault_cookie {
src/auth.rs:456:		let vault = decrypt_vault(cookie_val).ok_or("No sessions found")?;
src/auth.rs:462:	let mut response = redirect(AUTH_LANDING_PATH);
src/auth.rs:463:	response.insert_cookie(
src/auth.rs:464:		Cookie::build((active_session_cookie_name(), username.to_string()))
src/auth.rs:465:			.path("/")
src/auth.rs:467:			.secure(secure_cookies())
src/auth.rs:475:/// Remove a session from the vault.
src/auth.rs:476:pub fn remove_session_from_vault(username: &str, vault_cookie: Option<&str>) -> Result<Response<Body>, String> {
src/auth.rs:477:	let mut vault = vault_cookie.and_then(|c| decrypt_vault(c)).unwrap_or_default();
src/auth.rs:479:	// If removing active session, switch to another if available
src/auth.rs:481:	let was_only = vault.sessions.len() == 1;
src/auth.rs:484:	let encrypted = encrypt_vault(&vault).ok_or("Failed to encrypt session vault")?;
src/auth.rs:485:	let mut response = redirect(AUTH_LANDING_PATH);
src/auth.rs:486:	response.insert_cookie(
src/auth.rs:487:		Cookie::build((session_cookie_name(), encrypted))
src/auth.rs:488:			.path("/")
src/auth.rs:490:			.secure(secure_cookies())
src/auth.rs:496:	// If we removed the active session and there are others, switch to first
src/auth.rs:498:		if let Some(first) = vault.active_session() {
src/auth.rs:499:			response.insert_cookie(
src/auth.rs:500:				Cookie::build((active_session_cookie_name(), first.username.clone()))
src/auth.rs:501:					.path("/")
src/auth.rs:503:					.secure(secure_cookies())
src/auth.rs:509:	} else if was_only || vault.sessions.is_empty() {
src/auth.rs:510:		// No sessions left - clear active cookie
src/auth.rs:511:		response.remove_cookie(active_session_cookie_name().to_string());
src/auth.rs:516:/// Returns `true` when the `Secure` cookie attribute should be set.
src/auth.rs:520:pub fn secure_cookies() -> bool {
src/auth.rs:522:		.secure_cookies
src/auth.rs:544:/// Reserialize a mutated `SessionData` back into the session cookie on a response.
src/auth.rs:545:pub fn update_session_cookie(response: &mut Response<Body>, data: &SessionData) {
src/auth.rs:546:	if let Some(encrypted) = encrypt_session(data) {
src/auth.rs:547:		response.insert_cookie(
src/auth.rs:548:			Cookie::build((session_cookie_name(), encrypted))
src/auth.rs:549:				.path("/")
src/auth.rs:551:				.secure(secure_cookies())
src/auth.rs:561:/// Validate the CSRF token submitted in a POST form body against the session.
src/auth.rs:563:pub fn validate_csrf_token(auth: &AuthContext, submitted: &str) -> Result<(), String> {
src/auth.rs:564:	if let AuthContext::UserSession(session) = auth {
src/auth.rs:565:		if submitted != session.csrf_token {
src/auth.rs:566:			return Err("CSRF token mismatch — request rejected".to_string());
src/auth.rs:579:		return Ok(redirect(AUTH_LANDING_PATH));
src/auth.rs:587:	let local_profiles = token_import::discover_local_profiles()
src/auth.rs:606:async fn complete_browser_import_login(bearer_token: String, user_agent: Option<String>) -> Result<Response<Body>, String> {
src/auth.rs:607:	let username = fetch_username(&bearer_token).await.unwrap_or_else(|_| "unknown".to_string());
src/auth.rs:608:	let reddit_subs = client::fetch_subscribed_subreddits_with_bearer(&bearer_token).await.unwrap_or_default();
src/auth.rs:614:	let session = SessionData {
src/auth.rs:615:		access_token: bearer_token,
src/auth.rs:616:		refresh_token: String::new(),
src/auth.rs:619:		csrf_token: Uuid::new_v4().to_string(),
src/auth.rs:622:	let mut response = add_session_to_vault(session)?;
src/auth.rs:625:		response.insert_cookie(
src/auth.rs:626:			Cookie::build((subscriptions_cookie_name(), reddit_subs.join("+")))
src/auth.rs:627:				.path("/")
src/auth.rs:629:				.secure(secure_cookies())
src/auth.rs:639:/// `POST /login/reddit` — generate a CSRF state token, then redirect to Reddit's
src/auth.rs:648:				"Reddit OAuth is not configured. Set REDLIB_OAUTH_CLIENT_ID and REDLIB_OAUTH_CLIENT_SECRET in redlibe-secrets. In your Reddit app (reddit.com/prefs/apps) set redirect URI to https://redlibe.home/auth/callback.",
src/auth.rs:652:	let redirect_uri = CONFIG.oauth_redirect_uri.clone().ok_or("REDLIB_OAUTH_REDIRECT_URI is not configured")?;
src/auth.rs:654:	// CSRF state token — stored in a short-lived cookie, compared on callback
src/auth.rs:658:		"https://www.reddit.com/api/v1/authorize?client_id={client_id}&response_type=code&state={state}&redirect_uri={redirect_uri}&duration=permanent&scope={scopes}",
src/auth.rs:661:		redirect_uri = percent_encoding::utf8_percent_encode(&redirect_uri, percent_encoding::NON_ALPHANUMERIC),
src/auth.rs:665:	let mut response = redirect(&authorize_url);
src/auth.rs:666:	response.insert_cookie(
src/auth.rs:667:		Cookie::build((csrf_cookie_name(), state))
src/auth.rs:668:			.path("/")
src/auth.rs:670:			.secure(secure_cookies())
src/auth.rs:672:			// Expire the CSRF cookie after 10 minutes — enough time to complete login
src/auth.rs:679:/// `POST /login/ssh-import` — extract a Reddit bearer token from a Firefox or
src/auth.rs:683:/// Reads cookies.sqlite on the remote, decodes the token_v2 JWT, detects the
src/auth.rs:684:/// browser version/arch for a matching User-Agent, then creates a session.
src/auth.rs:704:	// Validate ssh_host and ssh_user to prevent injection (used as CLI args, not shell strings)
src/auth.rs:713:	let ssh_password = form.get("ssh_password").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
src/auth.rs:715:	if !has_pasted_key && ssh_password.is_none() && !has_config_key {
src/auth.rs:716:		return render_login_error(&prefs, "Provide either an SSH private key or an SSH password (or both).");
src/auth.rs:719:	// Use pasted private key from form if provided; otherwise we'll use password or config key
src/auth.rs:720:	let (key_path_opt, _temp_guard) = if let Some(pasted) = form.get("ssh_private_key").map(|s| s.trim()) {
src/auth.rs:737:			let path = temp_dir.join(format!("redlib_ssh_{}.key", Uuid::new_v4()));
src/auth.rs:738:			if std::fs::write(&path, normalized).is_err() {
src/auth.rs:744:				if std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).is_err() {
src/auth.rs:745:					let _ = std::fs::remove_file(&path);
src/auth.rs:749:			let path_str = path.to_string_lossy().into_owned();
src/auth.rs:750:			(Some(path_str), Some(TempKeyFile(path)))
src/auth.rs:756:	// If no pasted key, use config key path only when password not provided (so key-only from server config)
src/auth.rs:757:	let key_path_opt = if key_path_opt.is_some() {
src/auth.rs:758:		key_path_opt
src/auth.rs:759:	} else if ssh_password.is_none() {
src/auth.rs:761:		Some(shellexpand::tilde(&k).into_owned())
src/auth.rs:766:	// Run the extraction: key+passphrase when both provided, else key-only, else password-only
src/auth.rs:767:	let result = match (key_path_opt.as_ref(), ssh_password.as_ref()) {
src/auth.rs:770:			ssh_extract_token_key_passphrase(&ssh_host, &ssh_user, kp, pass, browser).await
src/auth.rs:774:			ssh_extract_token(&ssh_host, &ssh_user, kp, browser).await
src/auth.rs:777:			log::info!("SSH import: using password auth");
src/auth.rs:778:			ssh_extract_token_with_password(&ssh_host, &ssh_user, pass, browser).await
src/auth.rs:781:			return render_login_error(&prefs, "Provide either an SSH private key or an SSH password (or both).");
src/auth.rs:791:					If the key is passphrase-protected, enter the passphrase in the password field.",
src/auth.rs:798:		Ok((bearer_token, user_agent)) => complete_browser_import_login(bearer_token, Some(user_agent)).await,
src/auth.rs:802:/// `POST /login/local-import` — import a browser token from a local browser profile.
src/auth.rs:807:/// - `profile_path` (optional manual override path)
src/auth.rs:818:	let profile_path = form.get("profile_path").map(|s| s.as_str());
src/auth.rs:820:	match token_import::import_local(browser, profile_id, profile_path) {
src/auth.rs:821:		Ok(imported) => complete_browser_import_login(imported.bearer_token, imported.user_agent).await,
src/auth.rs:835:/// Returns `(find_dirs, version_bin)` for the remote shell script based on
src/auth.rs:845:		// auto: try all known paths for both browsers
src/auth.rs:854:/// Extract a Reddit bearer token and build a matching Firefox User-Agent by
src/auth.rs:855:/// SSHing to the remote machine and querying its browser cookies.sqlite via sqlite3.
src/auth.rs:857:/// Returns `(bearer_token, user_agent_string)`.
src/auth.rs:858:async fn ssh_extract_token(host: &str, user: &str, key_path: &str, browser: &str) -> Result<(String, String), String> {
src/auth.rs:862:DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
src/auth.rs:863:[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
src/auth.rs:865:TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
src/auth.rs:866:[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
src/auth.rs:883:				key_path,
src/auth.rs:925:	let mut token_raw = String::new();
src/auth.rs:931:			token_raw = v.to_string();
src/auth.rs:941:	if token_raw.is_empty() {
src/auth.rs:942:		return Err("No token found in SSH output".to_string());
src/auth.rs:945:	// Decode the JWT payload to extract the bearer token
src/auth.rs:946:	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
src/auth.rs:947:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:948:		token_raw
src/auth.rs:968:/// Same as ssh_extract_token but uses sshpass to supply the key passphrase (for encrypted private keys).
src/auth.rs:969:/// Runs: sshpass -p passphrase ssh -i key_path -o BatchMode=no ...
src/auth.rs:970:async fn ssh_extract_token_key_passphrase(host: &str, user: &str, key_path: &str, passphrase: &str, browser: &str) -> Result<(String, String), String> {
src/auth.rs:974:DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
src/auth.rs:975:[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
src/auth.rs:977:TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
src/auth.rs:978:[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
src/auth.rs:999:				key_path,
src/auth.rs:1039:	let mut token_raw = String::new();
src/auth.rs:1044:			token_raw = v.to_string();
src/auth.rs:1053:	if token_raw.is_empty() {
src/auth.rs:1054:		return Err("No token found in SSH output".to_string());
src/auth.rs:1057:	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
src/auth.rs:1058:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:1059:		token_raw
src/auth.rs:1076:/// Same as ssh_extract_token but authenticates with password via sshpass.
src/auth.rs:1077:async fn ssh_extract_token_with_password(host: &str, user: &str, password: &str, browser: &str) -> Result<(String, String), String> {
src/auth.rs:1081:DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
src/auth.rs:1082:[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
src/auth.rs:1084:TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
src/auth.rs:1085:[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
src/auth.rs:1102:				password,
src/auth.rs:1134:			"SSH extraction (password) failed exit={code} stderr_len={} stderr=\"{}\" stdout_preview=\"{}\"",
src/auth.rs:1143:	let mut token_raw = String::new();
src/auth.rs:1148:			token_raw = v.to_string();
src/auth.rs:1157:	if token_raw.is_empty() {
src/auth.rs:1158:		return Err("No token found in SSH output".to_string());
src/auth.rs:1161:	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
src/auth.rs:1162:		log::warn!("token_v2 could not be decoded as JWT; using raw value");
src/auth.rs:1163:		token_raw
src/auth.rs:1180:/// `GET /auth/callback` — handle Reddit's OAuth redirect, exchange the code
src/auth.rs:1181:/// for tokens, and set the encrypted session cookie.
src/auth.rs:1184:	let client_secret = CONFIG.oauth_client_secret.clone().ok_or("REDLIB_OAUTH_CLIENT_SECRET is not configured")?;
src/auth.rs:1185:	let redirect_uri = CONFIG.oauth_redirect_uri.clone().ok_or("REDLIB_OAUTH_REDIRECT_URI is not configured")?;
src/auth.rs:1193:	let csrf_cookie = req.cookie(csrf_cookie_name()).ok_or("Missing CSRF cookie — possible CSRF attack or cookie expired")?;
src/auth.rs:1194:	if state != csrf_cookie.value() {
src/auth.rs:1205:	// Exchange authorization code for access + refresh tokens
src/auth.rs:1206:	let tokens = exchange_code(&client_id, &client_secret, code, &redirect_uri).await?;
src/auth.rs:1209:	let username = fetch_username(&tokens.access_token).await.unwrap_or_else(|_| "unknown".to_string());
src/auth.rs:1210:	// Populate Reddit subscriptions for Feeds nav (fetch before moving tokens into session)
src/auth.rs:1211:	let reddit_subs: Vec<String> = client::fetch_subscribed_subreddits_with_bearer(&tokens.access_token).await.unwrap_or_default();
src/auth.rs:1213:	let expires_at = OffsetDateTime::now_utc().unix_timestamp() + tokens.expires_in as i64;
src/auth.rs:1215:	let session = SessionData {
src/auth.rs:1216:		access_token: tokens.access_token,
src/auth.rs:1217:		refresh_token: tokens.refresh_token,
src/auth.rs:1220:		csrf_token: Uuid::new_v4().to_string(),
src/auth.rs:1223:	let mut response = add_session_to_vault(session)?;
src/auth.rs:1226:		response.insert_cookie(
src/auth.rs:1227:			Cookie::build((subscriptions_cookie_name(), reddit_subs.join("+")))
src/auth.rs:1228:				.path("/")
src/auth.rs:1230:				.secure(secure_cookies())
src/auth.rs:1236:	// Clear the CSRF cookie — it's served its purpose
src/auth.rs:1237:	response.remove_cookie(csrf_cookie_name().to_string());
src/auth.rs:1241:/// `POST /logout` — validate CSRF token, remove active session from vault.
src/auth.rs:1249:	// Get session cookie before consuming request (clone to own the value)
src/auth.rs:1250:	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());
src/auth.rs:1252:	// Read and parse POST body for CSRF token (with size limit)
src/auth.rs:1259:	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
src/auth.rs:1260:	validate_csrf_token(&auth, submitted_csrf)?;
src/auth.rs:1262:	// Remove active session from vault
src/auth.rs:1264:		return remove_session_from_vault(&username, vault_cookie.as_deref());
src/auth.rs:1267:	let mut response = redirect(AUTH_LANDING_PATH);
src/auth.rs:1268:	response.remove_cookie(session_cookie_name().to_string());
src/auth.rs:1269:	response.remove_cookie(active_session_cookie_name().to_string());
src/auth.rs:1270:	response.remove_cookie(subscriptions_cookie_name().to_string());
src/auth.rs:1277:	// Get session cookie before consuming request (clone to own the value)
src/auth.rs:1278:	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());
src/auth.rs:1291:	switch_active_session(username, vault_cookie.as_deref())
src/auth.rs:1297:	// Get session cookie before consuming request (clone to own the value)
src/auth.rs:1298:	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());
src/auth.rs:1311:	remove_session_from_vault(username, vault_cookie.as_deref())
src/auth.rs:1318:	access_token: String,
src/auth.rs:1319:	refresh_token: String,
src/auth.rs:1323:/// Exchange an authorization code for a Reddit access + refresh token pair.
src/auth.rs:1324:async fn exchange_code(client_id: &str, client_secret: &str, code: &str, redirect_uri: &str) -> Result<TokenResponse, String> {
src/auth.rs:1325:	let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
src/auth.rs:1327:		"grant_type=authorization_code&code={}&redirect_uri={}",
src/auth.rs:1329:		percent_encoding::utf8_percent_encode(redirect_uri, percent_encoding::NON_ALPHANUMERIC),
src/auth.rs:1334:		.uri("https://www.reddit.com/api/v1/access_token")
src/auth.rs:1349:		log::error!("OAuth token exchange failed: HTTP {status} — {}", String::from_utf8_lossy(&bytes));
src/auth.rs:1350:		return Err("Authentication failed — could not obtain access token from Reddit".to_string());
src/auth.rs:1355:		log::error!("OAuth token exchange: failed to parse Reddit token response");
src/auth.rs:1361:async fn fetch_username(access_token: &str) -> Result<String, String> {
src/auth.rs:1364:		.uri("https://oauth.reddit.com/api/v1/me")
src/auth.rs:1365:		.header("Authorization", format!("Bearer {access_token}"))
src/auth.rs:1381:/// Refresh an expired access token using the stored refresh token.
src/auth.rs:1382:/// Returns `(new_access_token, new_expires_at_unix_timestamp)`.
src/auth.rs:1383:pub async fn refresh_access_token(refresh_token: &str) -> Result<(String, i64), String> {
src/auth.rs:1385:	let client_secret = CONFIG.oauth_client_secret.clone().ok_or("REDLIB_OAUTH_CLIENT_SECRET not configured")?;
src/auth.rs:1387:	let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
src/auth.rs:1389:		"grant_type=refresh_token&refresh_token={}",
src/auth.rs:1390:		percent_encoding::utf8_percent_encode(refresh_token, percent_encoding::NON_ALPHANUMERIC),
src/auth.rs:1395:		.uri("https://www.reddit.com/api/v1/access_token")
src/auth.rs:1407:	let new_token = json["access_token"].as_str().ok_or("Missing access_token in refresh response")?.to_string();
src/auth.rs:1411:	Ok((new_token, expires_at))
src/auth.rs:1415:/// from `/subreddits/mine/subscriber` and refresh the subscriptions cookie.
src/auth.rs:1426:	let mut response = crate::utils::redirect(&back);
src/auth.rs:1429:		response.insert_cookie(
src/auth.rs:1430:			Cookie::build((subscriptions_cookie_name(), subs.join("+")))
src/auth.rs:1431:				.path("/")
src/auth.rs:1433:				.secure(secure_cookies())
src/subreddit.rs:5:use crate::auth::{secure_cookies, subscriptions_cookie_name, AuthContext};
src/subreddit.rs:10:	get_filter_keywords, get_filters, get_read_ids, info, nsfw_landing, param, redirect, rewrite_urls, setting, template, val, Post, Preferences, Subreddit,
src/subreddit.rs:13:use cookie::Cookie;
src/subreddit.rs:32:#[template(path = "subreddit.html")]
src/subreddit.rs:40:	redirect_url: String,
src/subreddit.rs:52:#[template(path = "wiki.html")]
src/subreddit.rs:62:#[template(path = "wall.html")]
src/subreddit.rs:118:	let path_and_query = req.uri().path_and_query()?.as_str().to_string();
src/subreddit.rs:119:	if path_and_query.contains("after=") || path_and_query.contains("before=") {
src/subreddit.rs:123:	path_and_query.hash(&mut hasher);
src/subreddit.rs:192:	// Build Reddit API path
src/subreddit.rs:193:	let root = req.uri().path() == "/";
src/subreddit.rs:198:	let post_sort = req.cookie("post_sort").map_or_else(|| "hot".to_string(), |c| c.value().to_string());
src/subreddit.rs:233:		return Ok(redirect(&["/user/", &sub_name[2..]].concat()));
src/subreddit.rs:242:		if req.uri().path().starts_with("/r/") {
src/subreddit.rs:271:	let path = format!("/r/{}/{sort}.json?{}{params}", sub_name.replace('+', "%2B"), req.uri().query().unwrap_or_default());
src/subreddit.rs:272:	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));
src/subreddit.rs:273:	let redirect_url = url[1..].replace('?', "%3F").replace('&', "%26").replace('+', "%2B");
src/subreddit.rs:281:			sort: (sort, param(&path, "t").unwrap_or_default()),
src/subreddit.rs:282:			ends: (param(&path, "after").unwrap_or_default(), String::new()),
src/subreddit.rs:285:			redirect_url,
src/subreddit.rs:297:		match Post::fetch(&path, quarantined).await {
src/subreddit.rs:317:					sort: (sort, param(&path, "t").unwrap_or_default()),
src/subreddit.rs:318:					ends: (param(&path, "after").unwrap_or_default(), after),
src/subreddit.rs:321:					redirect_url,
src/subreddit.rs:362:	let mut response = redirect(&redir);
src/subreddit.rs:363:	response.insert_cookie(
src/subreddit.rs:365:			.path("/")
src/subreddit.rs:367:			.expires(cookie::Expiration::Session)
src/subreddit.rs:378:/// Chunk read-id strings into cookie-sized comma-separated strings (max READ_IDS_COOKIE_CHUNK bytes per chunk).
src/subreddit.rs:399:/// POST /mark-read: body ids=t3_xxx,t3_yyy — merge with existing read_ids and set cookies.
src/subreddit.rs:403:	while req.cookie(&format!("read_ids{old_numbered_count}")).is_some() {
src/subreddit.rs:430:		response.insert_cookie(
src/subreddit.rs:432:				.path("/")
src/subreddit.rs:438:	// Remove any old read_idsN cookies beyond what we wrote (we write read_ids, read_ids1, ... read_ids(num_chunks-1))
src/subreddit.rs:440:		response.remove_cookie(format!("read_ids{n}"));
src/subreddit.rs:445:// Join items in chunks of 4000 bytes in length for cookies
src/subreddit.rs:454:		// Use 4000 bytes to leave us some headroom because the name and options of the cookie count towards the 4096 byte cap
src/subreddit.rs:456:			// If last item add a seperator on the end of the list so it's interpreted properly in tanden with the next cookie
src/subreddit.rs:480:// Sub, filter, unfilter, or unsub by setting subscription cookie using response "Set-Cookie" header
src/subreddit.rs:483:	let action: Vec<String> = req.uri().path().split('/').map(String::from).collect();
src/subreddit.rs:499:	// When logged in, track subscribe/unsubscribe so we can sync to Reddit and refresh reddit_subscriptions cookie
src/subreddit.rs:532:			let path: String = format!("/r/{part}/about.json?raw_json=1");
src/subreddit.rs:533:			display = json(path, true).await;
src/subreddit.rs:568:	// check for redirect parameter if unsubscribing/unfiltering from outside sidebar
src/subreddit.rs:569:	let path = if let Some(redirect_path) = param(&format!("?{query}"), "redirect") {
src/subreddit.rs:570:		format!("/{redirect_path}")
src/subreddit.rs:575:	let mut response = redirect(&path);
src/subreddit.rs:577:	// If sub_list is empty remove all subscriptions cookies, otherwise update them and remove old ones
src/subreddit.rs:579:		// Remove subscriptions cookie
src/subreddit.rs:580:		response.remove_cookie("subscriptions".to_string());
src/subreddit.rs:582:		// Start with first numbered subscriptions cookie
src/subreddit.rs:585:		// While whatever subscriptionsNUMBER cookie we're looking at has a value
src/subreddit.rs:586:		while req.cookie(&format!("subscriptions{subscriptions_number}")).is_some() {
src/subreddit.rs:587:			// Remove that subscriptions cookie
src/subreddit.rs:588:			response.remove_cookie(format!("subscriptions{subscriptions_number}"));
src/subreddit.rs:590:			// Increment subscriptions cookie number
src/subreddit.rs:594:		// Start at 0 to keep track of what number we need to start deleting old subscription cookies from
src/subreddit.rs:597:		// Starting at 0 so we handle the subscription cookie without a number first
src/subreddit.rs:599:			let subscriptions_cookie = if subscriptions_number == 0 {
src/subreddit.rs:605:			response.insert_cookie(
src/subreddit.rs:606:				Cookie::build((subscriptions_cookie, list))
src/subreddit.rs:607:					.path("/")
src/subreddit.rs:616:		// While whatever subscriptionsNUMBER cookie we're looking at has a value
src/subreddit.rs:617:		while req.cookie(&format!("subscriptions{subscriptions_number_to_delete_from}")).is_some() {
src/subreddit.rs:618:			// Remove that subscriptions cookie
src/subreddit.rs:619:			response.remove_cookie(format!("subscriptions{subscriptions_number_to_delete_from}"));
src/subreddit.rs:621:			// Increment subscriptions cookie number
src/subreddit.rs:626:	// If filters is empty remove all filters cookies, otherwise update them and remove old ones
src/subreddit.rs:628:		// Remove filters cookie
src/subreddit.rs:629:		response.remove_cookie("filters".to_string());
src/subreddit.rs:631:		// Start with first numbered filters cookie
src/subreddit.rs:634:		// While whatever filtersNUMBER cookie we're looking at has a value
src/subreddit.rs:635:		while req.cookie(&format!("filters{filters_number}")).is_some() {
src/subreddit.rs:636:			// Remove that filters cookie
src/subreddit.rs:637:			response.remove_cookie(format!("filters{filters_number}"));
src/subreddit.rs:639:			// Increment filters cookie number
src/subreddit.rs:643:		// Start at 0 to keep track of what number we need to start deleting old filters cookies from
src/subreddit.rs:647:			let filters_cookie = if filters_number == 0 {
src/subreddit.rs:653:			response.insert_cookie(
src/subreddit.rs:654:				Cookie::build((filters_cookie, list))
src/subreddit.rs:655:					.path("/")
src/subreddit.rs:664:		// While whatever filtersNUMBER cookie we're looking at has a value
src/subreddit.rs:665:		while req.cookie(&format!("filters{filters_number_to_delete_from}")).is_some() {
src/subreddit.rs:666:			// Remove that filters cookie
src/subreddit.rs:667:			response.remove_cookie(format!("filters{filters_number_to_delete_from}"));
src/subreddit.rs:669:			// Increment filters cookie number
src/subreddit.rs:674:	// When logged in, sync subscribe/unsubscribe to Reddit and refresh reddit_subscriptions cookie for Feeds nav
src/subreddit.rs:683:				response.remove_cookie(subscriptions_cookie_name().to_string());
src/subreddit.rs:685:				response.insert_cookie(
src/subreddit.rs:686:					Cookie::build((subscriptions_cookie_name(), subs.join("+")))
src/subreddit.rs:687:						.path("/")
src/subreddit.rs:689:						.secure(secure_cookies())
src/subreddit.rs:709:	let path: String = format!("/r/{sub}/wiki/{page}.json?raw_json=1");
src/subreddit.rs:712:	match json(path, quarantined).await {
src/subreddit.rs:740:	let path: String = format!("/r/{sub}/about.json?raw_json=1");
src/subreddit.rs:744:	match json(path, quarantined).await {
src/subreddit.rs:781:// 	let path: String = format!("/r/{}/about/moderators.json?raw_json=1", sub);
src/subreddit.rs:784:// 	json(path, quarantined).await.map(|response| {
src/subreddit.rs:805:	let path: String = format!("/r/{sub}/about.json?raw_json=1");
src/subreddit.rs:808:	let res = json(path, quarantined).await?;
src/subreddit.rs:846:	let post_sort = req.cookie("post_sort").map_or_else(|| "hot".to_string(), |c| c.value().to_string());
src/subreddit.rs:849:	// Get path
src/subreddit.rs:850:	let path = format!("/r/{sub}/{sort}.json?{}", req.uri().query().unwrap_or_default());
src/subreddit.rs:856:	let (posts, _) = Post::fetch(&path, false).await?;
docs/FEATURE_GAP_BACKLOG.md:8:- **Redlib (upstream)** — Read-only, signed-out browsing; no real Reddit account session.
docs/FEATURE_GAP_BACKLOG.md:19:| Login / authenticated sessions | ✅ | ❌ | **Done** (OAuth + SSH import) | Must-have | OAuth 2.0 + SSH session import from Firefox/LibreWolf. |
docs/FEATURE_GAP_BACKLOG.md:20:| Multiple accounts + switching | ✅ (Hydra explicit) | ❌ | **Gap** | Nice-to-have | Single session per browser; would need multi-session + UI switcher. |
docs/FEATURE_GAP_BACKLOG.md:21:| Real subscriptions from Reddit | ✅ | ❌ (cookie list only) | **Done** | Must-have | Subscribe/unsubscribe API; subscriptions stored in cookies. |
docs/FEATURE_GAP_BACKLOG.md:26:- [x] **Pull Reddit subscriptions into Feeds/nav** — Subscribe/unsubscribe API works; subscriptions stored in cookies. (Done)
docs/FEATURE_GAP_BACKLOG.md:28:- [ ] **Multiple accounts** — Store multiple sessions, account switcher in nav/settings. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md:70:| Multireddits / custom feeds | ✅ | ❌ (manual /r/a+b+c) | **Done** | Must-have | Internal custom feeds: named, cookie-stored, Feeds menu + /feed/:name. Multis via /r/sub1+sub2+sub3. |
docs/FEATURE_GAP_BACKLOG.md:71:| Favorites (subs or posts) | ✅ | ❌ | **Partial** | Nice-to-have | “Saved” is Reddit-backed (done). Favorites as a distinct list (e.g. starred subs) could be cookie or API. |
docs/FEATURE_GAP_BACKLOG.md:73:| Subreddit filtering / muting | ✅ | ❌ | **Partial** | Nice-to-have | Redlib has filters (cookie); could align with “muted subs” or keyword filters. |
docs/FEATURE_GAP_BACKLOG.md:74:| Keyword filters | ✅ (Sync) | ❌ | **Gap** | Nice-to-have | Filter out posts by keyword; cookie or account-backed. |
docs/FEATURE_GAP_BACKLOG.md:79:- [ ] **Hide read / filter seen** — Persist “read” (cookie or API), filter listing. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md:82:- [ ] **Favorites (e.g. starred subs)** — Distinct from “subscriptions”; optional cookie or Reddit-backed. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md:103:| Download media | ✅ | ❌ | **Gap** | Nice-to-have | Download image/video from post (e.g. /img/... or proxy + download link). |
docs/FEATURE_GAP_BACKLOG.md:118:| Custom themes / edit themes | ✅ (Hydra) | ❌ | **Gap** | Nice-to-have | User-editable theme (e.g. CSS or color set) stored in cookie or account. |
docs/FEATURE_GAP_BACKLOG.md:123:- [ ] **Custom theme editor** — Simple color/font overrides, stored in cookie. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md:134:| **Feeds** | Custom feeds (internal), multireddits, filters (cookie) | Hide read, keyword filters, content-type filters |
docs/FEATURE_GAP_BACKLOG.md:178:- [Redlib (GitHub)](https://github.com/redlib-org/redlib) — “Private front-end for Reddit”; signed-out focus.
docs/FEATURE_GAP_BACKLOG.md:179:- [Hydra (App Store)](https://apps.apple.com/ca/app/hydra-read-upvote-comment/id6478089063) — “Read, upvote, comment.”
docs/FEATURE_GAP_BACKLOG.md:180:- [Sync for Reddit (AndroidGuys review)](https://androidguys.com/reviews/app-reviews/sync-for-reddit-the-gilded-way-of-browsing-reddit-review/).
docs/FEATURE_GAP_BACKLOG.md:181:- [Redlib — self-hosted Reddit (Akash Rajpurohit)](https://akashrajpurohit.com/blog/redlib-selfhosted-reddit-browsing-without-the-bloat/).
docs/FEATURE_GAP_BACKLOG.md:182:- [Sync guide (r/redditsync)](https://www.reddit.com/r/redditsync/comments/i41abv/a_comprehensive_guide_to_sync_for_reddit/).
docs/FEATURE_GAP_BACKLOG.md:183:- [Sync keyword filters (r/redditsync)](https://www.reddit.com/r/redditsync/comments/ypyivt/can_you_block_posts_which_include_particular/).
src/smart_feed/csrf.rs:3:use cookie::Cookie;
src/smart_feed/csrf.rs:8:fn rand_token() -> String {
src/smart_feed/csrf.rs:17:pub fn ensure_csrf_cookie(req: &Request<Body>, res: &mut Response<Body>) -> String {
src/smart_feed/csrf.rs:19:	if let Some(c) = req.cookie(CSRF_COOKIE) {
src/smart_feed/csrf.rs:22:	let token = rand_token();
src/smart_feed/csrf.rs:23:	let cookie = Cookie::build((CSRF_COOKIE, token.clone()))
src/smart_feed/csrf.rs:24:		.path("/")
src/smart_feed/csrf.rs:26:		.same_site(cookie::SameSite::Lax)
src/smart_feed/csrf.rs:27:		.max_age(cookie::time::Duration::days(90))
src/smart_feed/csrf.rs:29:	res.insert_cookie(cookie);
src/smart_feed/csrf.rs:30:	token
src/smart_feed/csrf.rs:33:pub fn verify_csrf(req: &Request<Body>, form_token: &str) -> Result<(), String> {
src/smart_feed/csrf.rs:34:	let Some(c) = req.cookie(CSRF_COOKIE) else {
src/smart_feed/csrf.rs:35:		return Err("Missing CSRF cookie".to_string());
src/smart_feed/csrf.rs:37:	if c.value() != form_token {
src/smart_feed/csrf.rs:38:		return Err("CSRF token mismatch".to_string());
src/smart_feed/stats_view.rs:1:use super::session::{ensure_sid, local_state_enabled};
src/smart_feed/stats_view.rs:8:#[template(path = "stats.html")]
src/smart_feed/cluster.rs:24:	for token in text.split_whitespace() {
src/smart_feed/cluster.rs:25:		let h = fxhash64(token.as_bytes());
src/config.rs:3:use std::path::PathBuf;
src/config.rs:150:	/// Reddit OAuth app client secret.
src/config.rs:152:	pub(crate) oauth_client_secret: Option<String>,
src/config.rs:154:	/// OAuth redirect URI registered with the Reddit app.
src/config.rs:156:	pub(crate) oauth_redirect_uri: Option<String>,
src/config.rs:158:	/// 32+ byte secret used to encrypt session cookies (AES-256-GCM key material).
src/config.rs:160:	pub(crate) session_secret: Option<String>,
src/config.rs:162:	/// Raw Reddit bearer token. When set, all API calls use this token directly,
src/config.rs:163:	/// bypassing the anonymous spoofed-token flow entirely.
src/config.rs:165:	pub(crate) raw_token: Option<String>,
src/config.rs:167:	/// Browser-exported Reddit token. Accepts the raw `token_v2` cookie value
src/config.rs:168:	/// from a Firefox/LibreWolf session — the JWT payload is decoded to extract
src/config.rs:169:	/// the bearer token. Falls back to treating the value as a raw bearer token
src/config.rs:172:	pub(crate) browser_token: Option<String>,
src/config.rs:174:	/// Set to `on` to add the `Secure` attribute to all session and CSRF cookies.
src/config.rs:178:	pub(crate) secure_cookies: Option<String>,
src/config.rs:187:	pub(crate) db_path: Option<String>,
src/config.rs:189:	// --- SSH browser-token import (redlib-extended) ---
src/config.rs:190:	/// SSH hostname (or alias) of the machine running the browser whose session
src/config.rs:227:			for path in per_user_config_candidates() {
src/config.rs:228:				let new_file = read_to_string(&path);
src/config.rs:273:			oauth_client_secret: parse("REDLIB_OAUTH_CLIENT_SECRET"),
src/config.rs:274:			oauth_redirect_uri: parse("REDLIB_OAUTH_REDIRECT_URI"),
src/config.rs:275:			session_secret: parse("REDLIB_SESSION_SECRET"),
src/config.rs:276:			raw_token: parse("REDLIB_RAW_TOKEN"),
src/config.rs:277:			browser_token: parse("REDLIB_BROWSER_TOKEN"),
src/config.rs:278:			secure_cookies: parse("REDLIB_SECURE_COOKIES"),
src/config.rs:280:			db_path: parse("REDLIB_DB_PATH"),
src/config.rs:352:		"REDLIB_OAUTH_CLIENT_SECRET" => config.oauth_client_secret.clone(),
src/config.rs:353:		"REDLIB_OAUTH_REDIRECT_URI" => config.oauth_redirect_uri.clone(),
src/config.rs:354:		"REDLIB_SESSION_SECRET" => config.session_secret.clone(),
src/config.rs:355:		"REDLIB_RAW_TOKEN" => config.raw_token.clone(),
src/config.rs:356:		"REDLIB_BROWSER_TOKEN" => config.browser_token.clone(),
src/config.rs:357:		"REDLIB_SECURE_COOKIES" => config.secure_cookies.clone(),
src/config.rs:359:		"REDLIB_DB_PATH" => config.db_path.clone(),
src/config.rs:442:	let config_to_write = r#"REDLIB_PUSHSHIFT_FRONTEND = "https://api.pushshift.io""#;
src/config.rs:445:	assert_eq!(get_setting("REDLIB_PUSHSHIFT_FRONTEND"), Some("https://api.pushshift.io".into()));
src/token_import/chromium.rs:3:use std::path::{Path, PathBuf};
src/token_import/chromium.rs:38:		let path = entry.path();
src/token_import/chromium.rs:39:		if !path.is_dir() {
src/token_import/chromium.rs:42:		let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
src/token_import/chromium.rs:46:		let cookies = path.join("Cookies");
src/token_import/chromium.rs:47:		if !cookies.is_file() {
src/token_import/chromium.rs:56:			path,
src/token_import/chromium.rs:62:pub fn import_token(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<ImportedBrowserSession, String> {
src/token_import/chromium.rs:63:	let profile = resolve_profile(browser, profile_id, profile_path)?;
src/token_import/chromium.rs:65:	let db_path = profile.join("Cookies");
src/token_import/chromium.rs:66:	if !db_path.is_file() {
src/token_import/chromium.rs:69:	let tmp = copy_to_temp(&db_path)?;
src/token_import/chromium.rs:71:	let conn = Connection::open(&tmp).map_err(|e| format!("Failed to open Chromium cookie DB: {e}"))?;
src/token_import/chromium.rs:73:	let row = read_chromium_cookie(&conn)?;
src/token_import/chromium.rs:74:	let token = if !row.value.is_empty() {
src/token_import/chromium.rs:75:		log::info!("Chromium local import: using plaintext cookie value");
src/token_import/chromium.rs:78:		log::info!("Chromium local import: encrypted cookie detected ({} bytes), attempting decrypt", row.encrypted_value.len());
src/token_import/chromium.rs:79:		decrypt_chromium_cookie(browser, &profile, &row)?
src/token_import/chromium.rs:81:		return Err("No reddit token_v2 cookie found in selected Chromium profile".to_string());
src/token_import/chromium.rs:85:		bearer_token: token,
src/token_import/chromium.rs:104:fn copy_to_temp(path: &Path) -> Result<PathBuf, String> {
src/token_import/chromium.rs:105:	let tmp = env::temp_dir().join(format!("redlib-chromium-cookies-{}.sqlite", Uuid::new_v4()));
src/token_import/chromium.rs:106:	fs::copy(path, &tmp).map_err(|e| format!("Failed to copy cookie DB: {e}"))?;
src/token_import/chromium.rs:110:fn read_chromium_cookie(conn: &Connection) -> Result<ChromiumCookieRow, String> {
src/token_import/chromium.rs:114:			 FROM cookies
src/token_import/chromium.rs:115:			 WHERE name = 'token_v2'
src/token_import/chromium.rs:120:		.map_err(|e| format!("Failed to prepare Chromium cookie query: {e}"))?;
src/token_import/chromium.rs:130:		.map_err(|_| "No reddit token_v2 cookie found in selected Chromium profile".to_string())
src/token_import/chromium.rs:133:fn decrypt_chromium_cookie(browser: &str, profile_dir: &Path, row: &ChromiumCookieRow) -> Result<String, String> {
src/token_import/chromium.rs:136:		return Err("Encrypted cookie value was empty".to_string());
src/token_import/chromium.rs:139:	// Newer Chromium cookie format: version prefix + AES-GCM payload.
src/token_import/chromium.rs:142:			"Chromium local import: cookie prefix={} (modern/legacy tagged format)",
src/token_import/chromium.rs:147:			if let Ok(plain) = decrypt_cookie_gcm(enc, &master_key, &row.host_key) {
src/token_import/chromium.rs:155:		if let Some(pass) = try_get_legacy_password(browser) {
src/token_import/chromium.rs:157:			if let Ok(plain) = decrypt_cookie_legacy_cbc(enc, &pass, legacy_pbkdf2_iterations(), &row.host_key) {
src/token_import/chromium.rs:165:			"Chromium token is encrypted and could not be decrypted. Try Firefox/LibreWolf local import, or provide a profile from an unlocked browser session on the same machine."
src/token_import/chromium.rs:171:	if let Some(pass) = try_get_legacy_password(browser) {
src/token_import/chromium.rs:173:		if let Ok(plain) = decrypt_cookie_legacy_cbc(enc, &pass, legacy_pbkdf2_iterations(), &row.host_key) {
src/token_import/chromium.rs:180:	Err("Unsupported Chromium cookie encryption format".to_string())
src/token_import/chromium.rs:183:fn decrypt_cookie_gcm(enc: &[u8], key: &[u8], host_key: &str) -> Result<String, String> {
src/token_import/chromium.rs:188:		return Err("Encrypted Chromium cookie payload is too short".to_string());
src/token_import/chromium.rs:194:	decode_cookie_plaintext(&plain, host_key)
src/token_import/chromium.rs:197:fn decrypt_cookie_legacy_cbc(enc: &[u8], password: &str, iterations: u32, host_key: &str) -> Result<String, String> {
src/token_import/chromium.rs:198:	let decrypted = decrypt_legacy_cbc_bytes(enc, password, iterations)?;
src/token_import/chromium.rs:199:	decode_cookie_plaintext(&decrypted, host_key)
src/token_import/chromium.rs:202:fn decrypt_legacy_cbc_bytes(enc: &[u8], password: &str, iterations: u32) -> Result<Vec<u8>, String> {
src/token_import/chromium.rs:205:	pbkdf2_hmac::<Sha1>(password.as_bytes(), b"saltysalt", iterations, &mut key);
src/token_import/chromium.rs:214:fn decode_cookie_plaintext(bytes: &[u8], host_key: &str) -> Result<String, String> {
src/token_import/chromium.rs:217:		if is_plausible_token(&trimmed) {
src/token_import/chromium.rs:226:			if is_plausible_token(&trimmed) {
src/token_import/chromium.rs:232:	Err(format!("Decrypted cookie did not look like a token for host {}", host_key))
src/token_import/chromium.rs:235:fn is_plausible_token(s: &str) -> bool {
src/token_import/chromium.rs:270:		log::info!("Chromium local import: attempting macOS Local State key unwrap via Safe Storage password");
src/token_import/chromium.rs:271:		return decrypt_local_state_key_with_safe_storage_password(browser, raw, 1003);
src/token_import/chromium.rs:286:			log::info!("Chromium local import: attempting Linux Local State key unwrap via Secret Service/legacy password");
src/token_import/chromium.rs:287:			return decrypt_local_state_key_with_safe_storage_password(browser, raw, 1);
src/token_import/chromium.rs:300:	let output = Command::new("powershell")
src/token_import/chromium.rs:312:fn try_get_legacy_password(browser: &str) -> Option<String> {
src/token_import/chromium.rs:317:			let output = Command::new("security").args(["find-generic-password", "-w", "-s", service]).output().ok()?;
src/token_import/chromium.rs:333:			for app in linux_secret_tool_application_candidates(browser) {
src/token_import/chromium.rs:334:				log::info!("Chromium local import: trying secret-tool lookup for application={}", app);
src/token_import/chromium.rs:335:				let output = Command::new("secret-tool").args(["lookup", "application", app]).output().ok()?;
src/token_import/chromium.rs:339:						log::info!("Chromium local import: secret-tool lookup succeeded for application={}", app);
src/token_import/chromium.rs:345:			log::warn!("Chromium local import: secret-tool lookup unavailable/failed; falling back to legacy 'peanuts' password");
src/token_import/chromium.rs:355:fn decrypt_local_state_key_with_safe_storage_password(browser: &str, raw: &[u8], iterations: u32) -> Result<Vec<u8>, String> {
src/token_import/chromium.rs:356:	let password = try_get_legacy_password(browser).ok_or_else(|| "No Safe Storage password available for Local State key unwrap".to_string())?;
src/token_import/chromium.rs:357:	let decrypted = decrypt_legacy_cbc_bytes(raw, &password, iterations)?;
src/token_import/chromium.rs:359:		log::info!("Chromium local import: Local State key unwrap via Safe Storage password succeeded");
src/token_import/chromium.rs:391:fn linux_secret_tool_application_candidates(browser: &str) -> &'static [&'static str] {
src/token_import/chromium.rs:398:fn resolve_profile(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<PathBuf, String> {
src/token_import/chromium.rs:399:	if let Some(path) = profile_path.map(str::trim).filter(|s| !s.is_empty()) {
src/token_import/chromium.rs:400:		return Ok(PathBuf::from(path));
src/token_import/chromium.rs:406:				return Ok(p.path);
src/token_import/chromium.rs:415:		.map(|p| p.path)
src/smart_feed/view.rs:8:use super::session::{ensure_sid, local_state_enabled};
src/smart_feed/view.rs:39:#[template(path = "feed.html")]
src/smart_feed/view.rs:200:	// Build fetch path from rule sources
src/smart_feed/view.rs:208:	let mut path = format!("/r/{}/{sort}.json?limit={limit}&raw_json=1", subs.replace('+', "%2B"), sort = preset_obj.upstream_sort);
src/smart_feed/view.rs:210:		path.push_str(&format!("&t={t}"));
src/smart_feed/view.rs:214:			path.push_str(&format!("&after={}", after_tok));
src/smart_feed/view.rs:218:	let (posts, after_cursor) = Post::fetch(&path, false).await?;
src/smart_feed/view.rs:387:	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
src/smart_feed/cluster_view.rs:4:use super::session::ensure_sid;
src/smart_feed/cluster_view.rs:22:#[template(path = "cluster.html")]
src/smart_feed/cluster_view.rs:68:	// Build fetch path
src/smart_feed/cluster_view.rs:73:	let mut path = format!("/r/{}/{sort}.json?limit=200&raw_json=1", subs.replace('+', "%2B"), sort = preset_obj.upstream_sort);
src/smart_feed/cluster_view.rs:75:		path.push_str(&format!("&t={t}"));
src/smart_feed/cluster_view.rs:78:	let (posts, _) = Post::fetch(&path, false).await?;
src/smart_feed/session.rs:3:use cookie::Cookie;
src/smart_feed/session.rs:14:	if let Some(c) = req.cookie("rl_sid") {
src/smart_feed/session.rs:18:	let cookie = Cookie::build(("rl_sid", sid.clone()))
src/smart_feed/session.rs:19:		.path("/")
src/smart_feed/session.rs:21:		.same_site(cookie::SameSite::Lax)
src/smart_feed/session.rs:22:		.max_age(cookie::time::Duration::days(365))
src/smart_feed/session.rs:24:	res.insert_cookie(cookie);
src/smart_feed/mod.rs:12:mod session;
src/smart_feed/saved_view.rs:2:use super::session::{ensure_sid, local_state_enabled};
src/smart_feed/saved_view.rs:21:#[template(path = "saved.html")]
src/smart_feed/saved_view.rs:39:	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
scripts/bench_openwebui.py:28:def make_headers(api_key):
scripts/bench_openwebui.py:30:        "Authorization": f"Bearer {api_key}",
scripts/bench_openwebui.py:35:def ollama_generate(base_url, api_key, model, prompt, options, keep_alive, timeout):
scripts/bench_openwebui.py:46:        headers=make_headers(api_key),
scripts/bench_openwebui.py:54:def chat_completion(base_url, api_key, model, prompt, timeout, stream):
scripts/bench_openwebui.py:63:        r = requests.post(url, headers=make_headers(api_key), json=payload, timeout=timeout)
scripts/bench_openwebui.py:74:        headers=make_headers(api_key),
scripts/bench_openwebui.py:101:            args.api_key,
scripts/bench_openwebui.py:120:            args.api_key,
scripts/bench_openwebui.py:180:            args.api_key,
scripts/bench_openwebui.py:193:            args.api_key,
scripts/bench_openwebui.py:227:    p.add_argument("--base-url", default=os.getenv("OWUI_BASE_URL", "http://openwebui.home"))
scripts/bench_openwebui.py:228:    p.add_argument("--api-key", default=os.getenv("OWUI_API_KEY"))
scripts/bench_openwebui.py:244:    if not args.api_key:
scripts/bench_openwebui.py:245:        p.error("Missing API key. Set --api-key or OWUI_API_KEY.")
src/smart_feed/mutes_ui.rs:2:use super::session::{ensure_sid, local_state_enabled, require_user_key};
src/smart_feed/mutes_ui.rs:10:#[template(path = "mutes.html")]
src/smart_feed/mutes_ui.rs:24:	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
src/smart_feed/mutes_ui.rs:73:	Ok(crate::utils::redirect(form.get("back").map(|s| s.as_str()).unwrap_or("/mutes")))
src/smart_feed/mutes_ui.rs:90:		None => return info(req, "No session.").await,
src/smart_feed/mutes_ui.rs:106:		.insert("content-disposition", hyper::header::HeaderValue::from_static("attachment; filename=\"mutes.json\""));
src/smart_feed/mutes_ui.rs:129:	Ok(crate::utils::redirect("/mutes"))
docs/desktop-distribution.md:16:- create `session_secret` on first run and reuse it
docs/desktop-distribution.md:30:  - plaintext cookie support
docs/desktop-distribution.md:31:  - encrypted-cookie decryption attempts:
docs/desktop-distribution.md:34:    - legacy AES-CBC + macOS Keychain password lookup (`security`)
docs/desktop-distribution.md:35:    - Windows DPAPI decrypt path for `Local State` key via PowerShell
docs/desktop-distribution.md:40:- Firefox-family local import is currently the most reliable path.
docs/desktop-distribution.md:69:- present a desktop app shell
docs/desktop-distribution.md:75:- current production desktop path remains `redlib-desktop`
docs/desktop-distribution.md:86:- still requires repository secrets and platform certificates (not configured by code alone)
docs/desktop-distribution.md:87:- see `docs/ci-signing-secrets.md` for recommended secret names and setup checklists
src/smart_feed/state.rs:2:use super::session::{ensure_sid, local_state_enabled, require_user_key};
src/smart_feed/state.rs:3:use crate::utils::{error, redirect};
src/smart_feed/state.rs:8:fn redirect_back(req: &Request<Body>) -> Response<Body> {
src/smart_feed/state.rs:14:	crate::utils::redirect(back)
src/smart_feed/state.rs:38:	Ok(redirect_back(&req))
src/smart_feed/state.rs:54:	Ok(redirect_back(&req))
src/smart_feed/state.rs:70:	Ok(redirect_back(&req))
src/smart_feed/state.rs:86:	Ok(redirect_back(&req))
src/smart_feed/state.rs:108:	Ok(redirect_back(&req))
src/smart_feed/state.rs:126:	Ok(redirect_back(&req))
src/smart_feed/state.rs:144:	Ok(redirect_back(&req))
src/smart_feed/state.rs:160:	Ok(redirect_back(&req))
src/smart_feed/state.rs:176:	Ok(redirect_back(&req))
src/smart_feed/state.rs:188:	Ok(redirect_back(&req))
src/smart_feed/state.rs:214:/// Marks the post read then redirects to the post URL.
src/smart_feed/state.rs:215:/// URL must be a relative path (starts with /) to prevent open redirect.
src/smart_feed/state.rs:222:	// Only allow relative paths
src/smart_feed/state.rs:225:	// Mark read if local state enabled and user has a session
src/smart_feed/state.rs:235:	Ok(redirect(&dest))
src/state/sqlite.rs:7:use tokio::task::spawn_blocking;
src/state/sqlite.rs:76:		let db_path = get_setting("REDLIB_DB_PATH").unwrap_or_else(|| "redlib.sqlite".into());
src/state/sqlite.rs:77:		let mgr = SqliteConnectionManager::file(db_path);
src/state/sqlite.rs:83:				.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
src/state/sqlite.rs:98:		spawn_blocking(move || {
src/state/sqlite.rs:126:		spawn_blocking(move || {
src/state/sqlite.rs:157:		spawn_blocking(move || {
src/state/sqlite.rs:160:				.execute(
src/state/sqlite.rs:174:		spawn_blocking(move || {
src/state/sqlite.rs:177:				.execute("DELETE FROM mute_rule WHERE id = ?1 AND user_key = ?2", params![mute_id, user_key])
src/state/sqlite.rs:191:		spawn_blocking(move || {
src/state/sqlite.rs:220:		spawn_blocking(move || {
src/state/sqlite.rs:249:		spawn_blocking(move || {
src/state/sqlite.rs:252:			tx.execute(
src/state/sqlite.rs:253:				"INSERT OR IGNORE INTO user_session(user_key, created_at, last_seen_at) VALUES(?1, ?2, ?2)",
src/state/sqlite.rs:266:					stmt.execute(params![user_key, id, now]).map_err(|e| e.to_string())?;
src/state/sqlite.rs:281:		spawn_blocking(move || {
src/state/sqlite.rs:284:				.execute(
src/state/sqlite.rs:301:		spawn_blocking(move || {
src/state/sqlite.rs:325:		spawn_blocking(move || {
src/state/sqlite.rs:340:		spawn_blocking(move || {
src/state/sqlite.rs:343:				.execute("UPDATE post_state SET is_read = 1 WHERE user_key = ?1 AND is_read = 0", params![user_key])
src/state/sqlite.rs:356:		spawn_blocking(move || {
src/state/sqlite.rs:359:				.execute(
src/state/sqlite.rs:377:		spawn_blocking(move || {
src/state/sqlite.rs:381:					.execute(
src/state/sqlite.rs:390:					.execute("UPDATE post_state SET saved_at = NULL WHERE user_key = ?1 AND post_id = ?2", params![user_key, post_id])
src/state/sqlite.rs:407:		spawn_blocking(move || {
src/state/sqlite.rs:423:						.execute(params![e.post_id, e.title, e.community, e.domain, e.permalink, e.score, e.comments, e.created_utc, now])
src/state/sqlite.rs:437:		spawn_blocking(move || {
src/state/sqlite.rs:479:		spawn_blocking(move || {
src/state/sqlite.rs:498:		spawn_blocking(move || {
src/state/sqlite.rs:501:				.execute(
src/state/sqlite.rs:519:		spawn_blocking(move || {
src/state/sqlite.rs:550:		spawn_blocking(move || {
src/state/sqlite.rs:581:		spawn_blocking(move || {
src/state/sqlite.rs:584:			tx.execute(
src/state/sqlite.rs:585:				"INSERT OR IGNORE INTO user_session(user_key, created_at, last_seen_at) VALUES(?1, ?2, ?2)",
src/state/sqlite.rs:589:			tx.execute(
src/state/sqlite.rs:608:		spawn_blocking(move || {
src/state/sqlite.rs:644:				tx.execute("UPDATE channel SET sort_order = ?1 WHERE user_key = ?2 AND slug = ?3", params![i as i64, user_key, s])
src/state/sqlite.rs:657:		spawn_blocking(move || {
src/state/sqlite.rs:660:				.execute("DELETE FROM channel WHERE user_key = ?1 AND slug = ?2", params![user_key, slug])
src/state/sqlite.rs:673:		spawn_blocking(move || {
src/state/sqlite.rs:753:		spawn_blocking(move || {
src/state/sqlite.rs:777:	// V1: base tables (user_session, post_state, mute_rule)
src/state/sqlite.rs:780:			.execute_batch(
src/state/sqlite.rs:783:            CREATE TABLE IF NOT EXISTS user_session (
src/state/sqlite.rs:796:                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
src/state/sqlite.rs:805:                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
src/state/sqlite.rs:818:			.execute_batch(
src/state/sqlite.rs:829:                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
src/state/sqlite.rs:840:		.execute_batch(
src/state/sqlite.rs:859:            FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
src/state/sqlite.rs:871:			.execute_batch(
src/state/sqlite.rs:885:			.execute_batch(
src/state/sqlite.rs:903:		.execute("DELETE FROM post_state WHERE last_seen_at < ?1 AND saved_at IS NULL", params![cutoff])
src/state/sqlite.rs:907:		.execute(
src/main.rs:14:use redlib::client::{canonical_path, proxy, rate_limit_check, upstream_diagnostics_snapshot, upstream_metrics_snapshot_json, upstream_prometheus_metrics, CLIENT};
src/main.rs:16:use redlib::utils::{error, redirect, ThemeAssets};
src/main.rs:210:			Arg::new("redirect-https")
src/main.rs:212:				.long("redirect-https")
src/main.rs:255:			message += "\nhttps://github.com/redlib-org/redlib/issues/new?assignees=sigaloid&labels=bug&title=%F0%9F%90%9B+Bug+Report%3A+Rate+limit+mismatch";
src/main.rs:284:	// in OAUTH case, optionally retrieve the token at startup to avoid paying
src/main.rs:285:	// the penalty at first request. Keep this lazy by default so SSH-session-only
src/main.rs:375:	app.at("/commits.atom").get(|_| async move { proxy_commit_info().await }.boxed());
src/main.rs:376:	app.at("/instances.json").get(|_| async move { proxy_instances().await }.boxed());
src/main.rs:379:	app.at("/vid/:id/:size").get(|r| proxy(r, "https://v.redd.it/{id}/DASH_{size}").boxed());
src/main.rs:380:	app.at("/hls/:id/*path").get(|r| proxy(r, "https://v.redd.it/{id}/{path}").boxed());
src/main.rs:381:	app.at("/img/*path").get(|r| proxy(r, "https://i.redd.it/{path}").boxed());
src/main.rs:382:	app.at("/thumb/:point/:id").get(|r| proxy(r, "https://{point}.thumbs.redditmedia.com/{id}").boxed());
src/main.rs:383:	app.at("/emoji/:id/:name").get(|r| proxy(r, "https://emoji.redditmedia.com/{id}/{name}").boxed());
src/main.rs:385:		.at("/emote/:subreddit_id/:filename")
src/main.rs:386:		.get(|r| proxy(r, "https://reddit-econ-prod-assets-permanent.s3.amazonaws.com/asset-manager/{subreddit_id}/{filename}").boxed());
src/main.rs:389:		.get(|r| proxy(r, "https://{loc}view.redd.it/award_images/{fullname}/{id}").boxed());
src/main.rs:390:	app.at("/preview/:loc/:id").get(|r| proxy(r, "https://{loc}view.redd.it/{id}").boxed());
src/main.rs:391:	app.at("/style/*path").get(|r| proxy(r, "https://styles.redditmedia.com/{path}").boxed());
src/main.rs:392:	app.at("/static/*path").get(|r| proxy(r, "https://www.redditstatic.com/{path}").boxed());
src/main.rs:397:		.get(|r| async move { Ok(redirect(&format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());
src/main.rs:424:		.get(|_| async { Ok(redirect("/login")) }.boxed())
src/main.rs:428:		.get(|_| async { Ok(redirect("/login")) }.boxed())
src/main.rs:432:		.get(|_| async { Ok(redirect("/login")) }.boxed())
src/main.rs:455:	app.at("/feed/:name").get(|r| feeds::redirect_to_feed(r).boxed());
src/main.rs:509:		.get(|r| async move { Ok(redirect(&format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());
src/main.rs:539:		.get(|r| async move { Ok(redirect(&format!("/r/{}/wiki", r.param("sub").unwrap_or_default()))) }.boxed());
src/main.rs:542:		.get(|r| async move { Ok(redirect(&format!("/r/{}/wiki/{}", r.param("sub").unwrap_or_default(), r.param("wiki").unwrap_or_default()))) }.boxed());
src/main.rs:554:	app.at("/w").get(|_| async { Ok(redirect("/wiki")) }.boxed());
src/main.rs:557:		.get(|r| async move { Ok(redirect(&format!("/wiki/{}", r.param("page").unwrap_or_default()))) }.boxed());
src/main.rs:581:				Some(id) if (8..12).contains(&id.len()) => match canonical_path(format!("/r/{sub}/s/{id}"), 3).await {
src/main.rs:582:					Ok(Some(path)) => Ok(redirect(&path)),
src/main.rs:600:				Some(id) if (5..8).contains(&id.len()) => match canonical_path(format!("/comments/{id}"), 3).await {
src/main.rs:601:					Ok(path_opt) => match path_opt {
src/main.rs:602:						Some(path) => Ok(redirect(&path)),
src/main.rs:627:pub async fn proxy_commit_info() -> Result<Response<Body>, String> {
src/main.rs:639:	let uri = Uri::from_str("https://github.com/redlib-org/redlib/commits/main.atom").expect("Invalid URI");
src/main.rs:646:pub async fn proxy_instances() -> Result<Response<Body>, String> {
src/main.rs:658:	let uri = Uri::from_str("https://raw.githubusercontent.com/redlib-org/redlib-instances/refs/heads/main/instances.json").expect("Invalid URI");
src/smart_feed/channels_ui.rs:3:use super::session::{local_state_enabled, require_user_key};
src/smart_feed/channels_ui.rs:6:use crate::utils::{error, info, redirect, Preferences};
src/smart_feed/channels_ui.rs:14:#[template(path = "channels.html")]
src/smart_feed/channels_ui.rs:24:#[template(path = "channel_edit.html")]
src/smart_feed/channels_ui.rs:167:	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
src/smart_feed/channels_ui.rs:203:			Ok(redirect(&format!("/channels/{slug}")))
src/smart_feed/channels_ui.rs:214:			let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
src/smart_feed/channels_ui.rs:258:	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
src/smart_feed/channels_ui.rs:347:	Ok(redirect(&format!("/reader/{slug}")))
src/smart_feed/channels_ui.rs:372:	Ok(redirect("/channels"))
src/smart_feed/channels_ui.rs:393:	Ok(redirect("/channels"))
src/client.rs:21:use crate::auth::{refresh_access_token, AuthContext, SessionData};
src/client.rs:23:use crate::oauth::{force_refresh_token, token_daemon, Oauth, OauthBackendImpl};
src/client.rs:27:const REDDIT_URL_BASE: &str = "https://oauth.reddit.com";
src/client.rs:30:const REDDIT_SHORT_URL_BASE: &str = "https://redd.it";
src/client.rs:33:const ALTERNATIVE_REDDIT_URL_BASE: &str = "https://www.reddit.com";
src/client.rs:43:				// https://github.com/redlib-org/redlib/issues/446#issuecomment-3609306592
src/client.rs:71:	tokio::spawn(token_daemon());
src/client.rs:105:fn log_upstream_event(path: &str, event: &str, status: Option<u16>, attempt: u8, detail: &str) {
src/client.rs:107:		"upstream_event event={event} status={} attempt={} path={} detail={}",
src/client.rs:110:		path,
src/client.rs:120:fn on_upstream_failure(path: &str, category: &str, status: Option<u16>, attempt: u8, detail: &str) {
src/client.rs:137:	log_upstream_event(path, category, status, attempt, detail);
src/client.rs:143:fn upstream_circuit_message(path: &str) -> Option<String> {
src/client.rs:150:			 Please retry shortly or log in so requests can use your account token. | {path}"
src/client.rs:276:/// Gets the canonical path for a resource on Reddit. This is accomplished by
src/client.rs:277:/// making a `HEAD` request to Reddit at the path given in `path`.
src/client.rs:279:/// This function returns `Ok(Some(path))`, where `path`'s value is identical
src/client.rs:280:/// to that of the value of the argument `path`, if Reddit responds to our
src/client.rs:284:/// the `String` will contain the path as reported in `Location`. The return
src/client.rs:290:pub async fn canonical_path(path: String, tries: i8) -> Result<Option<String>, String> {
src/client.rs:297:		// for url base and host in URL_PAIRS, try reddit_short_head(path.clone(), true, url_base, url_base_host) and if it succeeds, set res. else, res = None
src/client.rs:300:			res = reddit_short_head(path.clone(), true, url_base, url_base_host).await.ok();
src/client.rs:315:		// If Reddit responds with a 2xx, then the path is already canonical.
src/client.rs:316:		200..=299 => Ok(Some(path)),
src/client.rs:318:		// If Reddit responds with a 301, then the path is redirected.
src/client.rs:325:				// We need to strip the .json suffix from the original path.
src/client.rs:332:				// endpoints seem to return full paths, instead of relative paths.
src/client.rs:333:				// So we need to strip the .json suffix from the original path, and
src/client.rs:335:				// Otherwise, it will literally redirect to Reddit.com.
src/client.rs:339:				canonical_path(uri, tries - 1).await
src/client.rs:351:		// Special condition rate limiting - https://github.com/redlib-org/redlib/issues/229
src/client.rs:363:pub async fn proxy(req: Request<Body>, format: &str) -> Result<Response<Body>, String> {
src/client.rs:419:/// Makes a GET request to Reddit at `path`. By default, this will honor HTTP
src/client.rs:420:/// 3xx codes Reddit returns and will automatically redirect.
src/client.rs:421:fn reddit_get(path: String, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
src/client.rs:422:	request(&Method::GET, path, true, quarantine, REDDIT_URL_BASE, REDDIT_URL_BASE_HOST)
src/client.rs:425:/// Makes a HEAD request to Reddit at `path, using the short URL base. This will not follow redirects.
src/client.rs:426:fn reddit_short_head(path: String, quarantine: bool, base_path: &'static str, host: &'static str) -> Boxed<Result<Response<Body>, String>> {
src/client.rs:427:	request(&Method::HEAD, path, false, quarantine, base_path, host)
src/client.rs:430:// /// Makes a HEAD request to Reddit at `path`. This will not follow redirects.
src/client.rs:431:// fn reddit_head(path: String, quarantine: bool) -> Boxed<Result<Response<Body>, String>> {
src/client.rs:432:// 	request(&Method::HEAD, path, false, quarantine, false)
src/client.rs:436:/// Makes a request to Reddit. If `redirect` is `true`, `request_with_redirect`
src/client.rs:439:fn request(method: &'static Method, path: String, redirect: bool, quarantine: bool, base_path: &'static str, host: &'static str) -> Boxed<Result<Response<Body>, String>> {
src/client.rs:440:	// Build Reddit URL from path.
src/client.rs:441:	let url = format!("{base_path}{path}");
src/client.rs:468:	// shuffle headers: https://github.com/redlib-org/redlib/issues/324
src/client.rs:493:					// redirect based on caller params.
src/client.rs:494:					if response.status().is_redirection() {
src/client.rs:495:						if !redirect {
src/client.rs:511:									//     https://www.reddit.com) that may be
src/client.rs:513:									//     path (and query parameters) as
src/client.rs:516:									//     2. Percent-encode the path.
src/client.rs:517:									let new_path = percent_encode(val.as_bytes(), CONTROLS)
src/client.rs:522:									format!("{new_path}{}raw_json=1", if new_path.contains('?') { "&" } else { "?" })
src/client.rs:528:							base_path,
src/client.rs:582:					dbg_msg!("{method} {REDDIT_URL_BASE}{path}: {}", e);
src/client.rs:595:pub async fn json(path: String, quarantine: bool) -> Result<Value, String> {
src/client.rs:596:	let coalesce_key = format!("{}|{}", if quarantine { "q" } else { "n" }, path);
src/client.rs:614:			Err(_) => json_uncached(path, quarantine).await,
src/client.rs:618:	let result = json_uncached(path.clone(), quarantine).await;
src/client.rs:632:async fn json_uncached(path: String, quarantine: bool) -> Result<Value, String> {
src/client.rs:634:	if let Some(msg) = upstream_circuit_message(&path) {
src/client.rs:640:		match json_once(path.clone(), quarantine, attempt).await {
src/client.rs:655:	Err(last_err.unwrap_or_else(|| format!("Unknown Reddit JSON fetch error | {path}")))
src/client.rs:658:async fn json_once(path: String, quarantine: bool, attempt: u8) -> Result<Value, String> {
src/client.rs:660:	let err = |msg: &str, e: String, path: String| -> Result<Value, String> {
src/client.rs:662:		Err(format!("{msg}: {e} | {path}"))
src/client.rs:669:		warn!("Rate limit {current_rate_limit} is low. Spawning force_refresh_token()");
src/client.rs:670:		tokio::spawn(force_refresh_token());
src/client.rs:675:	match reddit_get(path.clone(), quarantine).await {
src/client.rs:679:				on_upstream_failure(&path, "http_429", Some(429), attempt, "rate limited");
src/client.rs:708:						// Rate limited, so spawn a force_refresh_token()
src/client.rs:709:						tokio::spawn(force_refresh_token());
src/client.rs:736:								// OAuth token has expired; http status 401
src/client.rs:738:									error!("Forcing a token refresh");
src/client.rs:739:									let () = force_refresh_token().await;
src/client.rs:740:									return Err("OAuth token has expired. Please refresh the page!".to_string());
src/client.rs:760:								Err(format!("Reddit error {} \"{}\": {} | {path}", json["error"], json["reason"], json["message"]))
src/client.rs:768:								on_upstream_failure(&path, "parse", Some(status.as_u16()), attempt, "server error + invalid json");
src/client.rs:776:									on_upstream_failure(&path, "html_403", Some(403), attempt, "html instead of json");
src/client.rs:778:									on_upstream_failure(&path, "parse", Some(status.as_u16()), attempt, "html instead of json");
src/client.rs:783:										See https://github.com/redlib-org/redlib/issues/446"
src/client.rs:786:								on_upstream_failure(&path, "parse", Some(status.as_u16()), attempt, "json parse error");
src/client.rs:787:								format!("{e} | {path}")
src/client.rs:789:							err("Failed to parse page JSON data", hint, path)
src/client.rs:794:					on_upstream_failure(&path, "transport", Some(status.as_u16()), attempt, "body receive failed");
src/client.rs:795:					err("Failed receiving body from Reddit", e.to_string(), path)
src/client.rs:800:			on_upstream_failure(&path, "transport", None, attempt, &e);
src/client.rs:801:			err("Couldn't send request to Reddit", e, path)
src/client.rs:806:/// Make an authenticated GET request to a Reddit API path and parse the JSON response.
src/client.rs:808:/// - `UserSession` / `RawBearer`: uses the caller's bearer token, bypassing the
src/client.rs:813:/// are user-specific and must not be shared across sessions.
src/client.rs:815:/// On 401 Unauthorized with a `UserSession` that has a non-empty `refresh_token`,
src/client.rs:816:/// automatically refreshes the access token, retries the request once, and returns
src/client.rs:817:/// the new session in the second element so the caller can call `update_session_cookie`.
src/client.rs:818:pub async fn authed_json(path: String, quarantine: bool, auth: &AuthContext) -> Result<(Value, Option<SessionData>), String> {
src/client.rs:819:	let bearer = match auth.bearer_token() {
src/client.rs:822:			let json = json(path, quarantine).await?;
src/client.rs:827:	let (json, _updated) = authed_json_with_bearer(path.clone(), quarantine, &bearer).await?;
src/client.rs:829:		// Try refresh for user sessions with a refresh token
src/client.rs:830:		if let Some(s) = auth.session_data() {
src/client.rs:831:			if !s.refresh_token.is_empty() {
src/client.rs:832:				let (new_token, expires_at) = refresh_access_token(&s.refresh_token).await?;
src/client.rs:833:				let new_session = SessionData {
src/client.rs:834:					access_token: new_token,
src/client.rs:835:					refresh_token: s.refresh_token.clone(),
src/client.rs:838:					csrf_token: s.csrf_token.clone(),
src/client.rs:840:				let (retry_json, _) = authed_json_with_bearer(path, quarantine, &new_session.access_token).await?;
src/client.rs:844:				return Ok((retry_json, Some(new_session)));
src/client.rs:847:		return Err("OAuth token is unauthorized — session may have expired".to_string());
src/client.rs:855:/// Inner helper: one GET with a given bearer token. Returns raw JSON (may contain error fields).
src/client.rs:856:async fn authed_json_with_bearer(path: String, quarantine: bool, bearer: &str) -> Result<(Value, Option<SessionData>), String> {
src/client.rs:857:	let url = format!("{REDDIT_URL_BASE}{path}");
src/client.rs:911:/// Uses the given bearer token (e.g. from OAuth callback). Used to populate Feeds nav when logged in.
src/client.rs:913:	let path = "/subreddits/mine/subscriber.json?limit=100&raw_json=1".to_string();
src/client.rs:914:	let (json, _) = authed_json_with_bearer(path, false, bearer).await?;
src/client.rs:925:	let path = "/subreddits/mine/subscriber.json?limit=100&raw_json=1".to_string();
src/client.rs:926:	let (json, _) = authed_json(path, false, auth).await?;
src/client.rs:935:/// On 401 Unauthorized with a `UserSession` that has a non-empty `refresh_token`,
src/client.rs:936:/// refreshes the access token, retries once, and returns the new session so the
src/client.rs:937:/// caller can call `update_session_cookie`.
src/client.rs:938:pub async fn authed_post(path: String, body_str: String, auth: &AuthContext) -> Result<(Value, Option<SessionData>), String> {
src/client.rs:939:	let bearer = auth.bearer_token().ok_or("Authenticated POST requires a logged-in session")?;
src/client.rs:941:	let (value, updated) = authed_post_with_bearer(path.clone(), body_str.clone(), bearer).await?;
src/client.rs:943:		if let Some(s) = auth.session_data() {
src/client.rs:944:			if !s.refresh_token.is_empty() {
src/client.rs:945:				let (new_token, expires_at) = refresh_access_token(&s.refresh_token).await?;
src/client.rs:946:				let new_session = SessionData {
src/client.rs:947:					access_token: new_token,
src/client.rs:948:					refresh_token: s.refresh_token.clone(),
src/client.rs:951:					csrf_token: s.csrf_token.clone(),
src/client.rs:953:				let (retry_value, _) = authed_post_with_bearer(path, body_str, &new_session.access_token).await?;
src/client.rs:957:				return Ok((retry_value, Some(new_session)));
src/client.rs:960:		return Err("OAuth token is unauthorized — session may have expired".to_string());
src/client.rs:968:async fn authed_post_with_bearer(path: String, body_str: String, bearer: &str) -> Result<(Value, Option<SessionData>), String> {
src/client.rs:969:	let url = format!("{REDDIT_URL_BASE}{path}");
src/client.rs:1017:	force_refresh_token().await;
src/client.rs:1062:	assert_eq!(canonical_path(share_link, 3).await, Ok(Some(canonical_link)));
docs/FEATURE_GAP_BACKLOG.md.bak:8:- **Redlib (upstream)** — Read-only, signed-out browsing; no real Reddit account session.
docs/FEATURE_GAP_BACKLOG.md.bak:19:| Login / authenticated sessions | ✅ | ❌ | **Done** (OAuth + SSH import) | Must-have | OAuth 2.0 + SSH session import from Firefox/LibreWolf. |
docs/FEATURE_GAP_BACKLOG.md.bak:20:| Multiple accounts + switching | ✅ (Hydra explicit) | ❌ | **Gap** | Nice-to-have | Single session per browser; would need multi-session + UI switcher. |
docs/FEATURE_GAP_BACKLOG.md.bak:21:| Real subscriptions from Reddit | ✅ | ❌ (cookie list only) | **Partial** | Must-have | Subscribe/unsubscribe API exists; “front page” can use instance cookie list. Pulling Reddit’s own sub list into nav/settings would align with “real” account. |
docs/FEATURE_GAP_BACKLOG.md.bak:26:- [ ] **Pull Reddit subscriptions into Feeds/nav** — Use Reddit API (e.g. `/subreddits/mine`) to show account subs alongside or instead of cookie-only list. (Must-have)
docs/FEATURE_GAP_BACKLOG.md.bak:28:- [ ] **Multiple accounts** — Store multiple sessions, account switcher in nav/settings. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md.bak:70:| Multireddits / custom feeds | ✅ | ❌ (manual /r/a+b+c) | **Done** | Must-have | Internal custom feeds: named, cookie-stored, Feeds menu + /feed/:name. Multis via /r/sub1+sub2+sub3. |
docs/FEATURE_GAP_BACKLOG.md.bak:71:| Favorites (subs or posts) | ✅ | ❌ | **Partial** | Nice-to-have | “Saved” is Reddit-backed (done). Favorites as a distinct list (e.g. starred subs) could be cookie or API. |
docs/FEATURE_GAP_BACKLOG.md.bak:73:| Subreddit filtering / muting | ✅ | ❌ | **Partial** | Nice-to-have | Redlib has filters (cookie); could align with “muted subs” or keyword filters. |
docs/FEATURE_GAP_BACKLOG.md.bak:74:| Keyword filters | ✅ (Sync) | ❌ | **Gap** | Nice-to-have | Filter out posts by keyword; cookie or account-backed. |
docs/FEATURE_GAP_BACKLOG.md.bak:79:- [ ] **Hide read / filter seen** — Persist “read” (cookie or API), filter listing. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md.bak:82:- [ ] **Favorites (e.g. starred subs)** — Distinct from “subscriptions”; optional cookie or Reddit-backed. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md.bak:103:| Download media | ✅ | ❌ | **Gap** | Nice-to-have | Download image/video from post (e.g. /img/... or proxy + download link). |
docs/FEATURE_GAP_BACKLOG.md.bak:118:| Custom themes / edit themes | ✅ (Hydra) | ❌ | **Gap** | Nice-to-have | User-editable theme (e.g. CSS or color set) stored in cookie or account. |
docs/FEATURE_GAP_BACKLOG.md.bak:123:- [ ] **Custom theme editor** — Simple color/font overrides, stored in cookie. (Nice-to-have)
docs/FEATURE_GAP_BACKLOG.md.bak:134:| **Feeds** | Custom feeds (internal), multireddits, filters (cookie) | Hide read, keyword filters, content-type filters |
docs/FEATURE_GAP_BACKLOG.md.bak:178:- [Redlib (GitHub)](https://github.com/redlib-org/redlib) — “Private front-end for Reddit”; signed-out focus.
docs/FEATURE_GAP_BACKLOG.md.bak:179:- [Hydra (App Store)](https://apps.apple.com/ca/app/hydra-read-upvote-comment/id6478089063) — “Read, upvote, comment.”
docs/FEATURE_GAP_BACKLOG.md.bak:180:- [Sync for Reddit (AndroidGuys review)](https://androidguys.com/reviews/app-reviews/sync-for-reddit-the-gilded-way-of-browsing-reddit-review/).
docs/FEATURE_GAP_BACKLOG.md.bak:181:- [Redlib — self-hosted Reddit (Akash Rajpurohit)](https://akashrajpurohit.com/blog/redlib-selfhosted-reddit-browsing-without-the-bloat/).
docs/FEATURE_GAP_BACKLOG.md.bak:182:- [Sync guide (r/redditsync)](https://www.reddit.com/r/redditsync/comments/i41abv/a_comprehensive_guide_to_sync_for_reddit/).
docs/FEATURE_GAP_BACKLOG.md.bak:183:- [Sync keyword filters (r/redditsync)](https://www.reddit.com/r/redditsync/comments/ypyivt/can_you_block_posts_which_include_particular/).
docs/ci-signing-secrets.md:3:This document defines the repository secrets/variables expected by `.github/workflows/tauri-bundle.yml`.
docs/ci-signing-secrets.md:13:## GitHub repository secrets (recommended names)
docs/ci-signing-secrets.md:26:  - App-specific password for the Apple ID notarization account.
docs/ci-signing-secrets.md:39:If you use an external signing service (e.g. Azure Trusted Signing / EV token), replace the workflow step with that provider’s action and add provider-specific secrets instead.
docs/ci-signing-secrets.md:51:  - Set to `1` to indicate the workflow should attempt signing/notarization-specific paths in future extensions.
docs/ci-signing-secrets.md:67:```powershell
docs/ci-signing-secrets.md:76:4. Create an app-specific password for the Apple ID.
docs/ci-signing-secrets.md:90:- Restrict repository admin access (secrets are high impact).
docs/ci-signing-secrets.md:91:- Rotate secrets after contractor/vendor access changes.
docs/ci-signing-secrets.md:96:- `tauri-bundle.yml` builds bundles without requiring secrets.
docs/ci-signing-secrets.md:98:- Missing secrets should not break local Rust builds or the non-Tauri release workflow.
src/utils.rs:12:use cookie::Cookie;
src/utils.rs:21:use serde_json_path::{JsonPath, JsonPathExt};
src/utils.rs:263:			let permalink_base = url_path_basename(data["permalink"].as_str().unwrap_or_default());
src/utils.rs:264:			let media_url_base = url_path_basename(url_val.as_str().unwrap_or_default());
src/utils.rs:276:				// Note: in the data["is_reddit_media_domain"] path above
src/utils.rs:477:	pub async fn fetch(path: &str, quarantine: bool) -> Result<(Vec<Self>, String), String> {
src/utils.rs:478:		let res = json(path.to_string(), quarantine).await?;
src/utils.rs:482:	/// Fetch listing with auth (for saved/upvoted/hidden). Returns (posts, after) and optional session update.
src/utils.rs:483:	pub async fn fetch_authed(path: &str, quarantine: bool, auth: &AuthContext) -> Result<((Vec<Self>, String), Option<crate::auth::SessionData>), String> {
src/utils.rs:484:		let (res, session_updated) = authed_json(path.to_string(), quarantine, auth).await?;
src/utils.rs:486:		Ok(((posts, after), session_updated))
src/utils.rs:491:#[template(path = "comment.html")]
src/utils.rs:570:#[template(path = "error.html")]
src/utils.rs:578:#[template(path = "info.html")]
src/utils.rs:589:#[template(path = "nsfwlanding.html")]
src/utils.rs:651:	/// URL path for this feed (e.g. /feed/My%20Tech). Set when parsing from cookie; not stored in JSON.
src/utils.rs:666:	/// Whether the current request is authenticated (user login or raw token).
src/utils.rs:676:	/// Per-session CSRF token for embedding in HTML forms.
src/utils.rs:680:	pub csrf_token: String,
src/utils.rs:725:	/// Custom feeds JSON (from cookie); revisioned as string. Use custom_feeds_parsed() for template.
src/utils.rs:784:	/// Build preferences from cookies (and auth context for login state).
src/utils.rs:798:		let csrf_token = auth.csrf_token();
src/utils.rs:800:		// When logged in, prefer Reddit account subscriptions (set at login); otherwise use cookie list
src/utils.rs:803:				.cookie("reddit_subscriptions")
src/utils.rs:814:			csrf_token,
src/utils.rs:891:pub(crate) fn parse_custom_feeds_cookie(req: &Request<Body>) -> Vec<CustomFeed> {
src/utils.rs:903:/// Gets a `HashSet` of filters from the cookie in the given `Request`.
src/utils.rs:908:/// Gets a `HashSet` of read post fullnames (e.g. t3_abc123) from the cookie in the given `Request`.
src/utils.rs:928:/// Gets keyword filter set from cookie (comma-separated, case-insensitive match).
src/utils.rs:937:/// Gets flair filter set from cookie (comma-separated).
src/utils.rs:946:/// Gets domain filter set from cookie (comma-separated, case-insensitive match).
src/utils.rs:955:/// Recent search queries (newline-separated in cookie, max 10).
src/utils.rs:993:/// New cookie value after adding a saved search (query and optional label). Deduplicates by query.
src/utils.rs:994:pub fn saved_searches_cookie_value_after_save(req: &Request<Body>, label: &str, query: &str) -> String {
src/utils.rs:1007:/// New cookie value after removing a saved search by query.
src/utils.rs:1008:pub fn saved_searches_cookie_value_after_unsave(req: &Request<Body>, query: &str) -> String {
src/utils.rs:1009:	saved_searches_cookie_value_after_unsave_raw(&setting(req, "saved_searches"), query)
src/utils.rs:1012:/// Like saved_searches_cookie_value_after_save but takes current cookie string (e.g. from headers after consuming request).
src/utils.rs:1013:pub fn saved_searches_cookie_value_after_save_raw(current: &str, label: &str, query: &str) -> String {
src/utils.rs:1038:/// Like saved_searches_cookie_value_after_unsave but takes current cookie string.
src/utils.rs:1039:pub fn saved_searches_cookie_value_after_unsave_raw(current: &str, query: &str) -> String {
src/utils.rs:1071:/// Build cookie value for recent searches after prepending a new query (caller sets cookie on response).
src/utils.rs:1072:pub fn recent_searches_cookie_value(req: &Request<Body>, new_query: &str) -> String {
src/utils.rs:1091:/// Gets the set of user-collapsed comment fullnames (t1_xxx) from cookie.
src/utils.rs:1175:			"<div class=\"md\"><p>[removed] — <a href=\"https://{}{permalink}\">view removed post</a></p></div>",
src/utils.rs:1258:		// Only populated when the request is made with a real user access token.
src/utils.rs:1272:pub fn param(path: &str, value: &str) -> Option<String> {
src/utils.rs:1274:		Url::parse(format!("https://libredd.it/{path}").as_str())
src/utils.rs:1286:	// Parse a cookie value from request
src/utils.rs:1288:	// If this was called with "subscriptions" and the "subscriptions" cookie has a value
src/utils.rs:1289:	if name == "subscriptions" && req.cookie("subscriptions").is_some() {
src/utils.rs:1293:		// Default subscriptions cookie
src/utils.rs:1294:		if req.cookie("subscriptions").is_some() {
src/utils.rs:1295:			subscriptions.push_str(req.cookie("subscriptions").unwrap().value());
src/utils.rs:1298:		// Start with first numbered subscription cookie
src/utils.rs:1301:		// While whatever subscriptionsNUMBER cookie we're looking at has a value
src/utils.rs:1302:		while req.cookie(&format!("subscriptions{subscriptions_number}")).is_some() {
src/utils.rs:1303:			// Push whatever subscriptionsNUMBER cookie we're looking at into the subscriptions string
src/utils.rs:1304:			subscriptions.push_str(req.cookie(&format!("subscriptions{subscriptions_number}")).unwrap().value());
src/utils.rs:1306:			// Increment subscription cookie number
src/utils.rs:1310:		// Return the subscriptions cookies as one large string
src/utils.rs:1313:	// If this was called with "filters" and the "filters" cookie has a value
src/utils.rs:1314:	else if name == "filters" && req.cookie("filters").is_some() {
src/utils.rs:1318:		// Default filters cookie
src/utils.rs:1319:		if req.cookie("filters").is_some() {
src/utils.rs:1320:			filters.push_str(req.cookie("filters").unwrap().value());
src/utils.rs:1323:		// Start with first numbered filters cookie
src/utils.rs:1326:		// While whatever filtersNUMBER cookie we're looking at has a value
src/utils.rs:1327:		while req.cookie(&format!("filters{filters_number}")).is_some() {
src/utils.rs:1328:			// Push whatever filtersNUMBER cookie we're looking at into the filters string
src/utils.rs:1329:			filters.push_str(req.cookie(&format!("filters{filters_number}")).unwrap().value());
src/utils.rs:1331:			// Increment filters cookie number
src/utils.rs:1335:		// Return the filters cookies as one large string
src/utils.rs:1338:	// If this was called with "read_ids" and the "read_ids" cookie has a value
src/utils.rs:1339:	else if name == "read_ids" && req.cookie("read_ids").is_some() {
src/utils.rs:1341:		read_ids.push_str(req.cookie("read_ids").unwrap().value());
src/utils.rs:1343:		while req.cookie(&format!("read_ids{read_ids_number}")).is_some() {
src/utils.rs:1345:			read_ids.push_str(req.cookie(&format!("read_ids{read_ids_number}")).unwrap().value());
src/utils.rs:1353:			.cookie(name)
src/utils.rs:1355:				// If there is no cookie for this setting, try receiving a default from the config
src/utils.rs:1377:/// Detect and redirect in the event of a random subreddit
src/utils.rs:1380:		Ok(redirect(&format!(
src/utils.rs:1387:		Err("No redirect needed".to_string())
src/utils.rs:1406:/// Direct urls to proxy if proxy is enabled
src/utils.rs:1467:	// ref: https://stackoverflow.com/a/4902622
src/utils.rs:1549:// These links all follow a pattern of "https://reddit-econ-prod-assets-permanent.s3.amazonaws.com/asset-manager/SUBREDDIT_ID/RANDOM_FILENAME.png"
src/utils.rs:1550:static REDDIT_EMOTE_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https://reddit-econ-prod-assets-permanent.s3.amazonaws.com/asset-manager/(.*)"#).unwrap());
src/utils.rs:1556:	/* Create the paths we'll use to look for our data inside the json.
src/utils.rs:1558:	let link_path = JsonPath::parse("$[*].s.u").expect("valid JSON Path");
src/utils.rs:1559:	let id_path = JsonPath::parse("$[*].id").expect("valid JSON Path");
src/utils.rs:1560:	let size_path = JsonPath::parse("$[*].s.y").expect("valid JSON Path");
src/utils.rs:1562:	// Extract all of the results from those json paths
src/utils.rs:1563:	let link_nodes = media_metadata.json_path(&link_path);
src/utils.rs:1564:	let id_nodes = media_metadata.json_path(&id_path);
src/utils.rs:1609:				let size = media_metadata.json_path(&size_path).first().unwrap().to_string();
src/utils.rs:1743:pub fn redirect(path: &str) -> Response<Body> {
src/utils.rs:1744:	// HTML-escape the path for safe embedding in the response body.
src/utils.rs:1745:	// The Location header uses the raw path; only the body text is escaped.
src/utils.rs:1746:	let escaped = path.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
src/utils.rs:1750:		.header("Location", path)
src/utils.rs:1831:/// Determines if a request should redirect to a NSFW landing gate.
src/utils.rs:1866:/// Returns the last (non-empty) segment of a path string
src/utils.rs:1867:pub fn url_path_basename(path: &str) -> String {
src/utils.rs:1868:	let url_result = Url::parse(format!("https://libredd.it/{path}").as_str());
src/utils.rs:1872:			url.path_segments_mut().unwrap().pop_if_empty();
src/utils.rs:1874:			url.path_segments().unwrap().next_back().unwrap().to_string()
src/utils.rs:1876:		Err(_) => path.to_string(),
src/utils.rs:1911:				"<a href=\"https://new.reddit.com/r/linux%5C_gaming/comments/x/just%5C_a%5C_test%5C/\">https://new.reddit.com/r/linux\\_gaming/comments/x/just\\_a\\_test/</a>"
src/utils.rs:1913:			"<a href=\"/r/linux_gaming/comments/x/just_a_test/\">https://new.reddit.com/r/linux_gaming/comments/x/just_a_test/</a>"
src/utils.rs:1917:				"e.g. &lt;a href=\"https://www.reddit.com/r/linux%5C_gaming/comments/ql9j15/anyone%5C_else%5C_confused%5C_with%5C_linus%5C_linux%5C_issues/\"&gt;https://www.reddit.com/r/linux\\_gaming/comments/ql9j15/anyone\\_else\\_confused\\_with\\_linus\\_linux\\_issues/&lt;/a&gt;"
src/utils.rs:1919:			"e.g. &lt;a href=\"/r/linux_gaming/comments/ql9j15/anyone_else_confused_with_linus_linux_issues/\"&gt;https://www.reddit.com/r/linux_gaming/comments/ql9j15/anyone_else_confused_with_linus_linux_issues/&lt;/a&gt;"
src/utils.rs:1933:		assert_eq!(format_url("https://a.thumbs.redditmedia.com/XYZ.jpg"), "/thumb/a/XYZ.jpg");
src/utils.rs:1934:		assert_eq!(format_url("https://emoji.redditmedia.com/a/b"), "/emoji/a/b");
src/utils.rs:1937:			format_url("https://external-preview.redd.it/foo.jpg?auto=webp&s=bar"),
src/utils.rs:1941:		assert_eq!(format_url("https://i.redd.it/foobar.jpg"), "/img/foobar.jpg");
src/utils.rs:1943:			format_url("https://preview.redd.it/qwerty.jpg?auto=webp&s=asdf"),
src/utils.rs:1946:		assert_eq!(format_url("https://v.redd.it/foo/DASH_360.mp4?source=fallback"), "/vid/foo/360.mp4");
src/utils.rs:1948:			format_url("https://v.redd.it/foo/HLSPlaylist.m3u8?a=bar&v=1&f=sd"),
src/utils.rs:1951:		assert_eq!(format_url("https://www.redditstatic.com/gold/awards/icon/icon.png"), "/static/gold/awards/icon/icon.png");
src/utils.rs:1953:			format_url("https://www.redditstatic.com/marketplace-assets/v1/core/emotes/snoomoji_emotes/free_emotes_pack/shrug.gif"),
src/utils.rs:1998:	let input = r#"<div class="md"><p>How can you have such hard feelings towards a license? <img src="https://www.redditstatic.com/marketplace-assets/v1/core/emotes/snoomoji_emotes/free_emotes_pack/shrug.gif" width="20" height="20" style="vertical-align:middle"> Let people use what license they want, and BSD is one of the least restrictive ones AFAIK.</p>"#;
src/utils.rs:2032:		r#"<p><a href="https://preview.redd.it/6awags382xo31.png?width=2560&amp;format=png&amp;auto=webp&amp;s=9c563aed4f07a91bdd249b5a3cea43a79710dcfc">caption 1</a></p>"#;
src/utils.rs:2038:fn test_url_path_basename() {
src/utils.rs:2040:	assert_eq!(url_path_basename("/first/last"), "last");
src/utils.rs:2042:	assert_eq!(url_path_basename("/first/last/"), "last");
src/utils.rs:2044:	assert_eq!(url_path_basename("/first/last/?some=query"), "last");
src/utils.rs:2045:	// file path
src/utils.rs:2046:	assert_eq!(url_path_basename("/cdn/image.jpg"), "image.jpg");
src/utils.rs:2047:	// when a full url is passed instead of just a path
src/utils.rs:2048:	assert_eq!(url_path_basename("https://doma.in/first/last"), "last");
src/utils.rs:2049:	// empty path
src/utils.rs:2050:	assert_eq!(url_path_basename("/"), "");
src/utils.rs:2055:	let json_input = serde_json::from_str(r#"{"emote|t5_31hpy|2028":{"e":"Image","id":"emote|t5_31hpy|2028","m":"image/png","s":{"u":"https://reddit-econ-prod-assets-permanent.s3.amazonaws.com/asset-manager/t5_31hpy/PW6WsOaLcd.png","x":60,"y":60},"status":"valid","t":"sticker"}}"#).expect("Valid JSON");
src/utils.rs:2063:	let input = r#"<div class="md"><p>Hi, I&#39;ve bought this very same monitor and found no calibration whatsoever. I have an ICC profile that has been set up since I&#39;ve installed its driver from the LG website and it works ok. I also used <a href="http://www.lagom.nl/lcd-test/">http://www.lagom.nl/lcd-test/</a> to calibrate it. After some good tinkering I&#39;ve found the following settings + the color profile from the driver gets me past all the tests perfectly:
src/utils.rs:2071:- Response Time Middle (personal preference, <a href="https://www.blurbusters.com/">https://www.blurbusters.com/</a> show horrible overdrive with it on high)
src/utils.rs:2078:	let output = r#"<div class="md"><p>Hi, I&#39;ve bought this very same monitor and found no calibration whatsoever. I have an ICC profile that has been set up since I&#39;ve installed its driver from the LG website and it works ok. I also used <a href="http://www.lagom.nl/lcd-test/">http://www.lagom.nl/lcd-test/</a> to calibrate it. After some good tinkering I&#39;ve found the following settings + the color profile from the driver gets me past all the tests perfectly:
src/utils.rs:2079:<ul><li>Brightness 50 (still have to settle on this one, it&#39;s personal preference, it controls the backlight, not the colors)</li><li>Contrast 70 (which for me was the default one)</li><li>Picture mode Custom</li><li>Super resolution + Off (it looks horrible anyway)</li><li>Sharpness 50 (default one I think)</li><li>Black level High (low messes up gray colors)</li><li>DFC Off</li><li>Response Time Middle (personal preference, <a href="https://www.blurbusters.com/">https://www.blurbusters.com/</a> show horrible overdrive with it on high)</li><li>Freesync doesn&#39;t matter</li><li>Black stabilizer 50</li><li>Gamma setting on 0</li><li>Color Temp Medium</li></ul>

## Public mutable ownership surfaces
src/server.rs:763:			let brotli_params = BrotliEncoderParams::default();
src/duplicates.rs:27:	/// params contains the relevant request parameters.
src/api.rs:73:	// Forward only whitelisted query params to prevent injection
src/subreddit.rs:262:	let mut params = String::from("&raw_json=1");
