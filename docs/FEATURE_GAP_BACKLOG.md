# Feature gap backlog: redlib-extended vs full Reddit clients

This document maps the **feature surface of full Reddit clients** (e.g. Sync, Hydra) onto **Redlib** and **redlib-extended**, and turns it into a prioritized backlog. The goal is to bring redlib-extended in line with “full Reddit client” capabilities where feasible, while keeping its privacy-first, self-hosted stance.

**Conventions**

- **Sync/Hydra** — What full mobile/desktop Reddit clients typically offer.
- **Redlib (upstream)** — Read-only, signed-out browsing; no real Reddit account session.
- **redlib-extended** — This fork: adds auth, write actions, and extended features on top of Redlib.
- Status: **Done** | **Partial** | **Planned** | **Gap** (not yet planned).
- Priority: **Must-have** (daily use) | **Nice-to-have** | **Hard/expensive** (API/scope/effort).

---

## 1) Reddit account support (identity + personalization)

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Login / authenticated sessions | ✅ | ❌ | **Done** (OAuth + SSH import) | Must-have | OAuth 2.0 + SSH session import from Firefox/LibreWolf. |
| Multiple accounts + switching | ✅ (Hydra explicit) | ❌ | **Gap** | Nice-to-have | Single session per browser; would need multi-session + UI switcher. |
| Real subscriptions from Reddit | ✅ | ❌ (cookie list only) | **Partial** | Must-have | Subscribe/unsubscribe API exists; “front page” can use instance cookie list. Pulling Reddit’s own sub list into nav/settings would align with “real” account. |
| Account-backed state: saved, upvoted, hidden | ✅ | ❌ | **Partial** | Must-have | Save/unsave **Done**. Upvoted/hidden **Gap** (no dedicated “saved” / “upvoted” / “hidden” listing pages yet). |

**Backlog (1)**

- [ ] **Pull Reddit subscriptions into Feeds/nav** — Use Reddit API (e.g. `/subreddits/mine`) to show account subs alongside or instead of cookie-only list. (Must-have)
- [ ] **Saved / Upvoted / Hidden listing pages** — Dedicated routes and UI for `/user/me/saved`, upvoted, hidden (history scope). (Must-have)
- [ ] **Multiple accounts** — Store multiple sessions, account switcher in nav/settings. (Nice-to-have)

---

## 2) Full interaction model (write actions)

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Upvote / downvote | ✅ | ❌ | **Done** | Must-have | POST /vote, UI on posts/comments. |
| Comment / reply | ✅ | ❌ | **Done** | Must-have | POST /comment, “Reply” and “Add a comment” in UI. |
| Create posts (text/link/media) | ✅ | ❌ | **Planned** | Must-have | README “Planned”: post submission. |
| Edit/delete your content | ✅ | ❌ | **Gap** | Must-have | Edit comment/post, delete comment/post via Reddit API. |

**Backlog (2)**

- [ ] **Post submission** — Text, link, image posts; submit to subreddit (submit scope). (Must-have)
- [ ] **Edit comment/post** — Reddit API for edit; UI to edit own comments/posts. (Must-have)
- [ ] **Delete comment/post** — Reddit API for delete; UI for own content. (Must-have)

---

## 3) Inbox, messages, and notifications

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Inbox (messages + notifications) | ✅ | ❌ | **Gap** | Must-have | No inbox/PM/notification UI. |
| Private messages | ✅ | ❌ | **Gap** | Must-have | Requires `privatemessages` scope + inbox/compose UI. |
| Mark read / “mark all read” | ✅ (Hydra) | ❌ | **Gap** | Nice-to-have | After inbox exists. |
| Push notifications | ✅ | ❌ | **Gap** | Nice-to-have | Server would need to poll or use Reddit push; out of scope for “web front-end” unless we add a small notifier. |

**Backlog (3)**

- [ ] **Inbox page** — List messages + comment replies (API: `/message/inbox`, etc.). (Must-have)
- [ ] **Private messages** — Read, reply, compose (API + `privatemessages` scope). (Must-have)
- [ ] **Mark (all) read** — After inbox exists. (Nice-to-have)

---

## 4) Feed organization & “power browsing” controls

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Multireddits / custom feeds | ✅ | ❌ (manual /r/a+b+c) | **Done** | Must-have | Internal custom feeds: named, cookie-stored, Feeds menu + /feed/:name. Multis via /r/sub1+sub2+sub3. |
| Favorites (subs or posts) | ✅ | ❌ | **Partial** | Nice-to-have | “Saved” is Reddit-backed (done). Favorites as a distinct list (e.g. starred subs) could be cookie or API. |
| Hide read / filter seen posts | ✅ | ❌ | **Gap** | Nice-to-have | Client-side or server-side “read” tracking; filter feed. |
| Subreddit filtering / muting | ✅ | ❌ | **Partial** | Nice-to-have | Redlib has filters (cookie); could align with “muted subs” or keyword filters. |
| Keyword filters | ✅ (Sync) | ❌ | **Gap** | Nice-to-have | Filter out posts by keyword; cookie or account-backed. |
| Content-type filtering | ✅ | ❌ | **Gap** | Nice-to-have | e.g. only images/GIFs/video; would need post-type metadata in listing. |

**Backlog (4)**

- [ ] **Hide read / filter seen** — Persist “read” (cookie or API), filter listing. (Nice-to-have)
- [ ] **Keyword filters** — UI to add/remove keyword filters; apply when rendering listing. (Nice-to-have)
- [ ] **Content-type filters** — Filter by post type (link, image, video, etc.). (Nice-to-have)
- [ ] **Favorites (e.g. starred subs)** — Distinct from “subscriptions”; optional cookie or Reddit-backed. (Nice-to-have)

---

## 5) Search depth

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Search (posts, subreddits, users) | ✅ “powerful” | Basic | **Partial** | Must-have | Redlib has search; redlib-extended inherits it. Deeper parity: recent searches, filters, subreddit/user scopes. |

**Backlog (5)**

- [ ] **Search UX parity** — Recent searches, clear scope (posts vs subs vs users), better result presentation. (Nice-to-have)

---

## 6) Media handling & export/share workflows

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Rich media viewers (image/video/gallery) | ✅ | Partial (HLS, etc.) | **Partial** | Nice-to-have | Redlib has playback; “client-grade” viewers and galleries could be improved. |
| Download media | ✅ | ❌ | **Gap** | Nice-to-have | Download image/video from post (e.g. /img/... or proxy + download link). |
| Share / system share | ✅ (Hydra) | ❌ | **Gap** | Nice-to-have | Web Share API or copy-link; no native “share sheet”. |

**Backlog (6)**

- [ ] **Download media** — Links or buttons to download image/video from post. (Nice-to-have)
- [ ] **Share** — Copy link, optional Web Share API. (Nice-to-have)

---

## 7) Deep customization (beyond theme/layout)

| Capability | Sync/Hydra | Redlib | redlib-extended | Priority | Notes |
|------------|------------|--------|-----------------|----------|-------|
| Preset themes + layout toggles | ✅ | ✅ | **Done** (inherited) | — | Redlib already has this. |
| Custom themes / edit themes | ✅ (Hydra) | ❌ | **Gap** | Nice-to-have | User-editable theme (e.g. CSS or color set) stored in cookie or account. |
| Custom fonts / typography | ✅ (Sync) | ❌ | **Gap** | Nice-to-have | Font family/size in settings; more “client-grade” UI tuning. |

**Backlog (7)**

- [ ] **Custom theme editor** — Simple color/font overrides, stored in cookie. (Nice-to-have)

---

## Summary: redlib-extended vs “full client” parity

| Area | Done / Partial | Planned / Gap |
|------|-----------------|----------------|
| **Account** | Login (OAuth + SSH), save/unsave, subscribe API | Multi-account, Reddit subs in nav, saved/upvoted/hidden pages |
| **Write** | Vote, comment/reply | Post create, edit, delete |
| **Inbox** | — | Inbox, PM, mark read |
| **Feeds** | Custom feeds (internal), multireddits, filters (cookie) | Hide read, keyword filters, content-type filters |
| **Search** | Inherited search | UX parity, recent searches |
| **Media** | Inherited playback | Download, share |
| **Customization** | Themes/layout (inherited) | Theme editor, fonts |

**Suggested implementation order (must-have first)**

1. **Saved / upvoted / hidden listing pages** (account-backed state).
2. **Pull Reddit subscriptions into Feeds** (real account subs).
3. **Post submission** (text/link/image).
4. **Edit/delete own content** (comment + post).
5. **Inbox + private messages** (messages scope + UI).

Then: hide read, keyword filters, inbox “mark all read”, multi-account, media download/share, search UX, custom theme editor.

---

## Parity phases (Sync/Hydra-style roadmap)

Phases below group the "medium + little" parity work into a sane backlog (no swipe/gesture). Phase 1 = "make browsing feel like a client"; Phase 2 = "polish that people immediately notice"; Phase 3 = "extra credit parity".

### Phase 1 — "Make browsing feel like a client"

- [x] **Read-state** — Mark posts as read on scroll; hide read posts; optional clear read / unread only / read indicators. (See Backlog (4): Hide read / filter seen.)
- [x] **Filters v1** — Flair filters / blacklist; keyword (titles, ideally comments); user; domain; subreddit; rule/template-based. (See Backlog (4): Keyword filters, content-type; extend to flair/user/domain/sub.)
- [x] **Custom feeds + favorites** — Local multireddits, favorite subreddits, fast sub list. (See Backlog (4): Custom feeds done; Favorites.)
- [x] **Comment nav + collapsed comment memory** — Next/prev top-level comment, scroll-to-comment; persisted collapsed state per thread; more inline/long-press-style actions. (New backlog.)

### Phase 2 — "Polish that people immediately notice"

- [x] **Gallery / media-only mode + better viewer + reader mode** — Gallery mode (Hydra-style), album handling, media-only browsing; reader mode for articles; low-data / media quality controls. (Extends Backlog (6) and section 6 table.)
- [x] **Settings search + export/import profiles** — Search within settings; manual backup/export and import (e.g. JSON); named profiles. (Extends Backlog (7) / section 7.)
- [x] **Quick subreddit search + better search surfacing** — Quick subreddit search; search within community, flair-scoped search, comment search; saved searches. (Extends Backlog (5).)
- [x] **Font sizing + rendering correctness** — Adjustable font sizing in settings; markdown table support and other rendering correctness. (Extends Backlog (7): Custom fonts/typography; new: tables.)

### Phase 3 — "Extra credit parity"

- [x] **Offline mode + cache controls** — Service Worker + cache policies; data usage controls; optional/toggleable. (New backlog.)
- [x] **Subreddit theme support** — Respect basic subreddit theming cues where feasible. (New backlog.)

---

## References

- [Redlib (GitHub)](https://github.com/redlib-org/redlib) — “Private front-end for Reddit”; signed-out focus.
- [Hydra (App Store)](https://apps.apple.com/ca/app/hydra-read-upvote-comment/id6478089063) — “Read, upvote, comment.”
- [Sync for Reddit (AndroidGuys review)](https://androidguys.com/reviews/app-reviews/sync-for-reddit-the-gilded-way-of-browsing-reddit-review/).
- [Redlib — self-hosted Reddit (Akash Rajpurohit)](https://akashrajpurohit.com/blog/redlib-selfhosted-reddit-browsing-without-the-bloat/).
- [Sync guide (r/redditsync)](https://www.reddit.com/r/redditsync/comments/i41abv/a_comprehensive_guide_to_sync_for_reddit/).
- [Sync keyword filters (r/redditsync)](https://www.reddit.com/r/redditsync/comments/ypyivt/can_you_block_posts_which_include_particular/).
