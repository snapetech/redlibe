(function() {
	"use strict";

	var LS_READLATER = "redlib_power_readlater_v1";
	var LS_PROFILES = "redlib_profiles_v1";
	var palette;
	var paletteInput;
	var paletteList;

	function safeJsonParse(s, fallback) {
		try { return JSON.parse(s || ""); } catch (_) { return fallback; }
	}

	function loadReadLater() {
		return safeJsonParse(localStorage.getItem(LS_READLATER), []);
	}

	function saveReadLater(items) {
		localStorage.setItem(LS_READLATER, JSON.stringify(items.slice(0, 400)));
	}

	function loadProfiles() {
		return safeJsonParse(localStorage.getItem(LS_PROFILES), []);
	}

	function saveProfiles(items) {
		localStorage.setItem(LS_PROFILES, JSON.stringify(items.slice(0, 50)));
	}

	function getPostMeta(postEl) {
		if (!postEl) return null;
		var permalink = postEl.getAttribute("data-post-permalink") || "";
		var title = postEl.getAttribute("data-post-title") || "";
		var community = postEl.getAttribute("data-post-community") || "";
		var id = postEl.getAttribute("data-post-id") || postEl.id || permalink;
		if (!id || !permalink) return null;
		return {
			id: id,
			permalink: permalink,
			title: title,
			community: community,
			savedAt: Date.now(),
			tags: []
		};
	}

	function upsertReadLater(item) {
		var items = loadReadLater();
		var idx = items.findIndex(function(x) { return x.id === item.id; });
		if (idx >= 0) {
			items[idx] = Object.assign({}, items[idx], item);
		} else {
			items.unshift(item);
		}
		saveReadLater(items);
		return items;
	}

	function removeReadLater(id) {
		var items = loadReadLater().filter(function(x) { return x.id !== id; });
		saveReadLater(items);
		return items;
	}

	function findReadLater(id) {
		return loadReadLater().find(function(x) { return x.id === id; }) || null;
	}

	function syncLocalToolButtons() {
		document.querySelectorAll(".post").forEach(function(postEl) {
			var meta = getPostMeta(postEl);
			if (!meta) return;
			var saved = findReadLater(meta.id);
			var saveBtn = postEl.querySelector(".local_readlater_btn");
			var tagBtn = postEl.querySelector(".local_tag_btn");
			if (saveBtn) {
				saveBtn.textContent = saved ? "Saved" : "Read later";
				saveBtn.classList.toggle("active", !!saved);
				saveBtn.setAttribute("aria-pressed", saved ? "true" : "false");
			}
			if (tagBtn) {
				var tagLabel = saved && saved.tags && saved.tags.length ? ("Tag (" + saved.tags.join(", ") + ")") : "Tag";
				tagBtn.textContent = tagLabel;
			}
		});
	}

	function attachLocalTools() {
		document.addEventListener("click", function(e) {
			var saveBtn = e.target.closest && e.target.closest(".local_readlater_btn");
			if (saveBtn) {
				e.preventDefault();
				var postEl = saveBtn.closest(".post");
				var meta = getPostMeta(postEl);
				if (!meta) return;
				var existing = findReadLater(meta.id);
				if (existing) removeReadLater(meta.id);
				else upsertReadLater(meta);
				syncLocalToolButtons();
				refreshPaletteCommands();
				return;
			}
			var tagBtn = e.target.closest && e.target.closest(".local_tag_btn");
			if (tagBtn) {
				e.preventDefault();
				var p = tagBtn.closest(".post");
				var m = getPostMeta(p);
				if (!m) return;
				var ex = findReadLater(m.id) || m;
				var current = (ex.tags || []).join(", ");
				var input = window.prompt("Tags for this post (comma-separated)", current);
				if (input === null) return;
				ex.tags = input.split(",").map(function(s) { return s.trim(); }).filter(Boolean).slice(0, 12);
				upsertReadLater(ex);
				syncLocalToolButtons();
				refreshPaletteCommands();
			}
		});
		syncLocalToolButtons();
	}

	function ensurePalette() {
		if (palette) return palette;
		palette = document.createElement("div");
		palette.className = "power_palette";
		palette.hidden = true;
		palette.innerHTML =
			'<div class="power_palette_backdrop" data-close="1"></div>' +
			'<div class="power_palette_panel" role="dialog" aria-modal="true" aria-label="Command palette">' +
			'<div class="power_palette_header">' +
			'<input id="power_palette_input" type="search" placeholder="Command palette (Ctrl/Cmd+K)" autocomplete="off">' +
			'</div>' +
			'<div class="power_palette_help">Feed: j/k move, o open, r read, s save, a archive | Comments: j/k, [ collapse, ] expand, Shift+C/E | Ctrl+K palette</div>' +
			'<ul id="power_palette_list" class="power_palette_list" role="listbox"></ul>' +
			'</div>';
		document.body.appendChild(palette);
		paletteInput = palette.querySelector("#power_palette_input");
		paletteList = palette.querySelector("#power_palette_list");
		palette.addEventListener("click", function(e) {
			if (e.target && e.target.getAttribute("data-close") === "1") closePalette();
			var item = e.target.closest(".power_palette_item");
			if (item) {
				var href = item.getAttribute("data-href");
				var action = item.getAttribute("data-action");
				if (action === "focus-search") {
					var s = document.getElementById("search");
					if (s) { s.focus(); s.select && s.select(); }
					closePalette();
				} else if (href) {
					window.location.href = href;
				}
			}
		});
		paletteInput.addEventListener("input", renderPaletteList);
		paletteInput.addEventListener("keydown", function(e) {
			if (e.key === "Escape") { closePalette(); return; }
			if (e.key === "Enter") {
				var first = paletteList.querySelector(".power_palette_item:not([hidden])");
				if (first) first.click();
			}
		});
		return palette;
	}

	var paletteCommands = [];

	function refreshPaletteCommands() {
		var cmds = [
			{ label: "Go: Home", href: "/" },
			{ label: "Go: Popular", href: "/r/popular" },
			{ label: "Go: All", href: "/r/all" },
			{ label: "Go: Settings", href: "/settings" },
			{ label: "Action: Focus Search", action: "focus-search" }
		];
		var loginLink = document.getElementById("login_link");
		if (loginLink) cmds.push({ label: "Go: Login", href: loginLink.getAttribute("href") || "/login" });
		loadReadLater().slice(0, 12).forEach(function(item) {
			var tags = item.tags && item.tags.length ? " [" + item.tags.join(", ") + "]" : "";
			cmds.push({ label: "Read later: " + (item.title || item.permalink) + tags, href: item.permalink });
		});
		paletteCommands = cmds;
		renderPaletteList();
	}

	function renderPaletteList() {
		if (!paletteList) return;
		var q = ((paletteInput && paletteInput.value) || "").trim().toLowerCase();
		paletteList.innerHTML = "";
		paletteCommands.forEach(function(cmd) {
			if (q && cmd.label.toLowerCase().indexOf(q) === -1) return;
			var li = document.createElement("li");
			li.className = "power_palette_item";
			li.tabIndex = 0;
			li.setAttribute("role", "option");
			li.textContent = cmd.label;
			if (cmd.href) li.setAttribute("data-href", cmd.href);
			if (cmd.action) li.setAttribute("data-action", cmd.action);
			li.addEventListener("keydown", function(e) { if (e.key === "Enter") li.click(); });
			paletteList.appendChild(li);
		});
		if (!paletteList.children.length) {
			var empty = document.createElement("li");
			empty.className = "power_palette_empty";
			empty.textContent = "No matching commands";
			paletteList.appendChild(empty);
		}
	}

	function openPalette() {
		ensurePalette();
		refreshPaletteCommands();
		palette.hidden = false;
		document.body.classList.add("power_palette_open");
		setTimeout(function() { if (paletteInput) paletteInput.focus(); }, 0);
	}

	function closePalette() {
		if (!palette) return;
		palette.hidden = true;
		document.body.classList.remove("power_palette_open");
	}

	function attachGlobalKeyboard() {
		document.addEventListener("keydown", function(e) {
			var target = e.target;
			var typing = target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
			if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "K")) {
				e.preventDefault();
				if (palette && !palette.hidden) closePalette(); else openPalette();
				return;
			}
			if (typing) return;
			if (e.key === "?") {
				e.preventDefault();
				openPalette();
			}
		});
	}

	function attachCommentPower() {
		// Persist collapse state (CSP-safe replacement for inline script)
		document.querySelectorAll("details.comment_right[data-comment-id]").forEach(function(details) {
			if (details.getAttribute("data-power-bound") === "1") return;
			details.setAttribute("data-power-bound", "1");
			details.addEventListener("toggle", function() {
				var id = details.getAttribute("data-comment-id");
				if (!id) return;
				var action = details.open ? "expand" : "collapse";
				fetch("/comment-collapse", {
					method: "POST",
					headers: { "Content-Type": "application/x-www-form-urlencoded" },
					body: "id=" + encodeURIComponent(id) + "&action=" + action,
					credentials: "same-origin"
				}).catch(function() {});
			});
		});

		var threads = Array.prototype.slice.call(document.querySelectorAll(".thread[data-top-level-comment]"));
		if (!threads.length) return;

		function visibleThreadIndex() {
			var i = threads.findIndex(function(t) { return t.getBoundingClientRect().top >= -10; });
			return i < 0 ? 0 : i;
		}
		function scrollToThread(index) {
			if (index >= 0 && index < threads.length) {
				threads[index].scrollIntoView({ behavior: "smooth", block: "start" });
				var firstDetails = threads[index].querySelector("details.comment_right");
				if (firstDetails) firstDetails.focus();
			}
		}
		function setThreadCollapsed(index, collapsed) {
			var details = threads[index] && threads[index].querySelector("details.comment_right");
			if (!details) return;
			details.open = !collapsed;
		}
		document.addEventListener("keydown", function(e) {
			var t = e.target;
			if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
			if (e.key === "j") { e.preventDefault(); scrollToThread((visibleThreadIndex() + 1) % threads.length); }
			else if (e.key === "k") { e.preventDefault(); scrollToThread((visibleThreadIndex() + threads.length - 1) % threads.length); }
			else if (e.key === "[") { e.preventDefault(); setThreadCollapsed(visibleThreadIndex(), true); }
			else if (e.key === "]") { e.preventDefault(); setThreadCollapsed(visibleThreadIndex(), false); }
			else if (e.shiftKey && e.key === "C") {
				e.preventDefault();
				document.querySelectorAll("details.comment_right[data-comment-id]").forEach(function(d) { d.open = false; });
			} else if (e.shiftKey && e.key === "E") {
				e.preventDefault();
				document.querySelectorAll("details.comment_right[data-comment-id]").forEach(function(d) { d.open = true; });
			}
		});
	}

	function attachMediaFallbacks() {
		document.querySelectorAll("video.post_media_video").forEach(function(video) {
			if (video.getAttribute("data-fallback-bound") === "1") return;
			video.setAttribute("data-fallback-bound", "1");
			video.addEventListener("error", function() {
				var parent = video.closest(".post");
				if (!parent) return;
				if (parent.querySelector(".video_fallback_hint")) return;
				var wrap = document.createElement("div");
				wrap.className = "video_fallback_hint";
				var permalink = parent.getAttribute("data-post-permalink") || "";
				var direct = video.getAttribute("src") || "";
				var source = video.querySelector('source[type="video/mp4"]');
				if (!direct && source) direct = source.getAttribute("src") || "";
				var alt = video.querySelector('source[type="application/vnd.apple.mpegurl"]');
				var hls = alt ? (alt.getAttribute("src") || "") : "";
				wrap.innerHTML =
					'<strong>Video playback failed here.</strong> ' +
					'<button type="button" class="vf_retry">Retry</button>' +
					(permalink ? ' <a href="' + permalink + '">Open post</a>' : '') +
					(direct ? ' <a rel="nofollow" href="' + direct + '">Open MP4</a>' : '') +
					(hls ? ' <a rel="nofollow" href="' + hls + '">Open HLS</a>' : '');
				var mediaContainer = video.closest(".post_media_content") || video.parentElement;
				if (mediaContainer) mediaContainer.appendChild(wrap);
				var retry = wrap.querySelector(".vf_retry");
				if (retry) {
					retry.addEventListener("click", function() {
						try {
							video.load();
							video.play && video.play().catch(function() {});
						} catch (_) {}
					});
				}
			});
		});
	}

	function attachSettingsProfiles() {
		var note = document.getElementById("settings_note");
		var settings = document.getElementById("settings");
		if (!note || !settings) return;
		var current = document.getElementById("current_prefs_urlencoded");
		var panel = document.createElement("div");
		panel.className = "settings_profiles_panel";
		panel.innerHTML =
			'<h3>Profiles</h3>' +
			'<p>Save local profiles for subscriptions/filters/preferences (work, news, hobbies) without a Reddit account.</p>' +
			'<div class="settings_profiles_actions">' +
			'<input type="text" id="settings_profile_name" placeholder="Profile name (e.g. Work)">' +
			'<button type="button" id="settings_profile_save">Save current profile</button>' +
			'</div>' +
			'<div id="settings_profiles_list" class="settings_profiles_list"></div>';
		note.prepend(panel);

		function renderProfiles() {
			var list = panel.querySelector("#settings_profiles_list");
			var profiles = loadProfiles();
			list.innerHTML = "";
			if (!profiles.length) {
				list.innerHTML = '<p class="settings_profiles_empty">No profiles yet.</p>';
				return;
			}
			profiles.forEach(function(p) {
				var row = document.createElement("div");
				row.className = "settings_profile_row";
				row.innerHTML =
					'<div class="settings_profile_meta"><strong></strong><span></span></div>' +
					'<div class="settings_profile_buttons">' +
					'<button type="button" class="apply">Apply</button>' +
					'<button type="button" class="rename">Rename</button>' +
					'<button type="button" class="delete">Delete</button>' +
					'</div>';
				row.querySelector("strong").textContent = p.name || "Unnamed";
				row.querySelector("span").textContent = new Date(p.updatedAt || p.createdAt || Date.now()).toLocaleString();
				row.querySelector(".apply").addEventListener("click", function() {
					window.location.href = "/settings/restore/?" + (p.encoded || "");
				});
				row.querySelector(".rename").addEventListener("click", function() {
					var next = window.prompt("Rename profile", p.name || "");
					if (!next) return;
					p.name = next.trim();
					p.updatedAt = Date.now();
					saveProfiles(profiles);
					renderProfiles();
					refreshPaletteCommands();
				});
				row.querySelector(".delete").addEventListener("click", function() {
					if (!window.confirm("Delete profile \"" + (p.name || "Unnamed") + "\"?")) return;
					saveProfiles(profiles.filter(function(x) { return x.id !== p.id; }));
					renderProfiles();
					refreshPaletteCommands();
				});
				list.appendChild(row);
			});
		}

		panel.querySelector("#settings_profile_save").addEventListener("click", function() {
			var encoded = current ? (current.value || "") : "";
			if (!encoded) {
				window.alert("Could not read current settings blob on this page.");
				return;
			}
			var input = panel.querySelector("#settings_profile_name");
			var name = (input.value || "").trim() || "Profile " + new Date().toLocaleTimeString();
			var profiles = loadProfiles();
			var existing = profiles.find(function(x) { return x.name === name; });
			if (existing) {
				existing.encoded = encoded;
				existing.updatedAt = Date.now();
			} else {
				profiles.unshift({ id: "p_" + Date.now() + "_" + Math.random().toString(36).slice(2, 8), name: name, encoded: encoded, createdAt: Date.now(), updatedAt: Date.now() });
			}
			saveProfiles(profiles);
			input.value = "";
			renderProfiles();
			refreshPaletteCommands();
		});

		renderProfiles();
	}

	/* ── Sidebar Init ────────────────────────────────────────── */
	function initSidebar() {
		var feeds = document.getElementById("feeds");
		if (!feeds) return;
		var isDesktop = window.innerWidth > 800;
		if (isDesktop) {
			feeds.open = true;
			document.body.classList.add("has-sidebar");
		}
	}

	/* ── Inline Feed Management ──────────────────────────────── */
	function attachInlineFeedManage() {
		// Inline add form
		var addForm = document.getElementById("add_feed_form");
		var addInput = document.getElementById("add_feed_input");
		if (addForm && addInput) {
			addForm.addEventListener("submit", function(e) {
				e.preventDefault();
				var sub = addInput.value.trim().replace(/^\/?r\//, "");
				if (!sub) return;
				var redirect = window.location.pathname.replace(/^\//, "");
				fetch("/r/" + encodeURIComponent(sub) + "/subscribe?redirect=" + encodeURIComponent(redirect), {
					method: "POST",
					credentials: "same-origin",
					redirect: "manual"
				}).then(function() {
					// Inject the new sub row before the add panel
					var addPanel = document.getElementById("add_feed_panel");
					var feedList = document.getElementById("feed_list");
					if (!addPanel || !feedList) return;
					var row = document.createElement("span");
					row.className = "feed_sub_row";
					var link = document.createElement("a");
					link.href = "/r/" + sub;
					link.textContent = sub;
					var unsub = document.createElement("button");
					unsub.className = "feed_unsub_btn";
					unsub.setAttribute("data-sub", sub);
					unsub.title = "Unsubscribe";
					unsub.textContent = "×";
					row.appendChild(link);
					row.appendChild(unsub);
					feedList.insertBefore(row, addPanel);
					addInput.value = "";
					// Close the add panel
					var panel = document.getElementById("add_feed_panel");
					if (panel) panel.open = false;
					attachUnsubBtn(unsub);
				}).catch(function() {});
			});
		}

		// Unsubscribe buttons
		document.querySelectorAll(".feed_unsub_btn").forEach(attachUnsubBtn);
	}

	function attachUnsubBtn(btn) {
		btn.addEventListener("click", function() {
			var sub = btn.getAttribute("data-sub");
			if (!sub) return;
			var redirect = window.location.pathname.replace(/^\//, "");
			fetch("/r/" + encodeURIComponent(sub) + "/unsubscribe?redirect=" + encodeURIComponent(redirect), {
				method: "POST",
				credentials: "same-origin",
				redirect: "manual"
			}).then(function() {
				var row = btn.closest(".feed_sub_row");
				if (row) row.remove();
			}).catch(function() {});
		});
	}

	/* ── Layout Switcher ─────────────────────────────────────── */
	var LS_LAYOUT = "redlib_layout_v1";
	var LAYOUT_CLASSES = ["card", "clean", "compact", "wide"];

	function applyLayout(layout) {
		var body = document.body;
		LAYOUT_CLASSES.forEach(function(cls) { body.classList.remove(cls); });
		if (layout && layout !== "card") body.classList.add(layout);
		// Persist preference
		fetch("/settings/update/?layout=" + encodeURIComponent(layout) + "&redirect=%2F", {
			method: "GET", credentials: "same-origin"
		}).catch(function() {});
		// Update active button state
		document.querySelectorAll(".layout_btn").forEach(function(btn) {
			var isActive = btn.getAttribute("data-layout") === layout ||
				(layout === "card" && btn.getAttribute("data-layout") === "card");
			btn.classList.toggle("active", isActive);
		});
	}

	function attachLayoutSwitcher() {
		// Determine current layout from body classes
		var currentLayout = "card";
		LAYOUT_CLASSES.forEach(function(cls) {
			if (document.body.classList.contains(cls)) currentLayout = cls;
		});
		document.querySelectorAll(".layout_btn").forEach(function(btn) {
			var layout = btn.getAttribute("data-layout");
			if (layout === currentLayout || (currentLayout === "card" && layout === "card")) {
				btn.classList.add("active");
			}
			btn.addEventListener("click", function() { applyLayout(layout); });
		});
	}

	/* ── Post Read-Status Tracking ───────────────────────────── */
	var LS_SEEN = "redlib_seen_v1";
	var LS_READ = "redlib_read_v1";

	function loadSet(key) {
		try { return new Set(JSON.parse(localStorage.getItem(key) || "[]")); } catch(_) { return new Set(); }
	}

	function saveSet(key, set) {
		var arr = Array.from(set);
		if (arr.length > 3000) arr = arr.slice(-3000);
		localStorage.setItem(key, JSON.stringify(arr));
	}

	function attachPostStatus() {
		var posts = document.querySelectorAll(".post[data-post-id]");
		if (!posts.length) return;
		var seen = loadSet(LS_SEEN);
		var read = loadSet(LS_READ);
		var seenChanged = false;

		posts.forEach(function(post) {
			var id = post.getAttribute("data-post-id") || "";
			if (!id) return;

			if (read.has(id)) {
				post.classList.add("post-read");
			} else if (seen.has(id)) {
				post.classList.add("post-seen");
			} else {
				post.classList.add("post-new");
				seen.add(id);
				seenChanged = true;
			}

			// Mark as read when the post title link is clicked
			var titleLink = post.querySelector(".post_title a");
			if (titleLink) {
				titleLink.addEventListener("click", function() {
					read.add(id);
					post.classList.remove("post-new", "post-seen");
					post.classList.add("post-read");
					saveSet(LS_READ, read);
				}, { once: true });
			}
		});

		if (seenChanged) saveSet(LS_SEEN, seen);
	}

	/* ── Feed Filter Toggle ──────────────────────────────────── */
	function attachFeedFilter() {
		var btns = document.querySelectorAll("#feed_filter .filter_btn");
		if (!btns.length) return;

		btns.forEach(function(btn) {
			btn.addEventListener("click", function() {
				var filter = btn.getAttribute("data-filter");
				btns.forEach(function(b) { b.classList.toggle("active", b === btn); });
				var posts = document.querySelectorAll(".post");
				posts.forEach(function(post) {
					var visible = true;
					if (filter === "new") visible = post.classList.contains("post-new");
					else if (filter === "unseen") visible = post.classList.contains("post-new") || post.classList.contains("post-seen");
					post.style.display = visible ? "" : "none";
				});
			});
		});
	}

	/* ── Feed Card Click-to-Open ─────────────────────────────── */
	function attachFeedCardClick() {
		var container = document.getElementById("feed_items");
		if (!container) return;
		container.addEventListener("click", function(e) {
			// Ignore clicks on interactive elements
			if (e.target.closest("a, button, input, textarea, select, label, form")) return;
			var card = e.target.closest(".feed_item[data-feed-open-url]");
			if (!card) return;
			var url = card.getAttribute("data-feed-open-url");
			if (url) window.location.href = url;
		});
		// Also set cursor on cards
		container.querySelectorAll(".feed_item[data-feed-open-url]").forEach(function(el) {
			el.style.cursor = "pointer";
		});
	}

	/* ── Reader Feed Keyboard Nav ────────────────────────────── */
	function attachFeedKeyNav() {
		var container = document.getElementById("feed_items");
		if (!container) return;

		var items = Array.prototype.slice.call(container.querySelectorAll(".feed_item[data-feed-post-id]"));
		if (!items.length) return;

		var currentIdx = -1;

		function getCsrf() {
			return container.getAttribute("data-csrf") || "";
		}

		function focusItem(idx) {
			if (idx < 0 || idx >= items.length) return;
			currentIdx = idx;
			items[idx].scrollIntoView({ behavior: "smooth", block: "nearest" });
			items[idx].focus();
			items.forEach(function(el, i) { el.classList.toggle("feed_item_focused", i === idx); });
		}

		function visibleIdx() {
			// Find the first item whose top is at or below the viewport top
			var scrollY = window.scrollY || window.pageYOffset;
			var vpTop = scrollY + 60; // account for fixed nav
			var best = 0;
			for (var i = 0; i < items.length; i++) {
				if (items[i].getBoundingClientRect().top + scrollY >= vpTop) { best = i; break; }
				best = i;
			}
			return best;
		}

		function postAction(endpoint, postId) {
			var csrf = getCsrf();
			if (!csrf || !postId) return;
			fetch(endpoint, {
				method: "POST",
				headers: { "Content-Type": "application/x-www-form-urlencoded" },
				body: "csrf=" + encodeURIComponent(csrf) + "&post_id=" + encodeURIComponent(postId),
				credentials: "same-origin"
			}).catch(function() {});
		}

		document.addEventListener("keydown", function(e) {
			var t = e.target;
			if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
			// Only activate on feed pages
			if (!container) return;

			if (e.key === "j") {
				e.preventDefault();
				var next = currentIdx < 0 ? visibleIdx() : Math.min(currentIdx + 1, items.length - 1);
				focusItem(next);
			} else if (e.key === "k") {
				e.preventDefault();
				var prev = currentIdx <= 0 ? 0 : currentIdx - 1;
				focusItem(prev);
			} else if (e.key === "o" || e.key === "Enter") {
				if (currentIdx < 0) return;
				var item = items[currentIdx];
				var openUrl = item.getAttribute("data-feed-open-url");
				if (openUrl) { window.location.href = openUrl; }
			} else if (e.key === "r") {
				if (currentIdx < 0) return;
				var item = items[currentIdx];
				var postId = item.getAttribute("data-feed-post-id");
				var isRead = item.classList.contains("feed_item_read");
				var endpoint = isRead ? "/action/mark_unread" : "/action/mark_read";
				postAction(endpoint, postId);
				item.classList.toggle("feed_item_read", !isRead);
				item.classList.toggle("feed_item_unread", isRead);
			} else if (e.key === "s") {
				if (currentIdx < 0) return;
				var item = items[currentIdx];
				var postId = item.getAttribute("data-feed-post-id");
				var isSaved = item.classList.contains("feed_item_saved");
				var endpoint = isSaved ? "/action/unsave" : "/action/save";
				postAction(endpoint, postId);
				item.classList.toggle("feed_item_saved", !isSaved);
			} else if (e.key === "a") {
				if (currentIdx < 0) return;
				var item = items[currentIdx];
				var postId = item.getAttribute("data-feed-post-id");
				postAction("/action/archive", postId);
				// Fade out and remove card
				item.style.transition = "opacity 0.25s";
				item.style.opacity = "0";
				setTimeout(function() { item.remove(); }, 270);
				// Move focus to next item
				items.splice(currentIdx, 1);
				if (items.length > 0) focusItem(Math.min(currentIdx, items.length - 1));
			}
		});
	}

	/* ── Scroll-to-Read (IntersectionObserver) ───────────────── */
	function attachScrollToRead() {
		var container = document.getElementById("feed_items");
		if (!container) return;
		if (!window.IntersectionObserver) return;

		var csrf = container.getAttribute("data-csrf") || "";
		if (!csrf) return; // local state not enabled

		var dwellTimers = {};
		var DWELL_MS = 2000;

		var observer = new IntersectionObserver(function(entries) {
			entries.forEach(function(entry) {
				var el = entry.target;
				var postId = el.getAttribute("data-feed-post-id");
				if (!postId) return;

				if (entry.isIntersecting && entry.intersectionRatio >= 0.5) {
					// Start dwell timer
					if (!dwellTimers[postId]) {
						dwellTimers[postId] = setTimeout(function() {
							delete dwellTimers[postId];
							if (!el.classList.contains("feed_item_read")) {
								fetch("/action/mark_read", {
									method: "POST",
									headers: { "Content-Type": "application/x-www-form-urlencoded" },
									body: "csrf=" + encodeURIComponent(csrf) + "&post_id=" + encodeURIComponent(postId),
									credentials: "same-origin"
								}).catch(function() {});
								el.classList.add("feed_item_read");
								el.classList.remove("feed_item_unread");
							}
						}, DWELL_MS);
					}
				} else {
					// Cancel dwell timer
					if (dwellTimers[postId]) {
						clearTimeout(dwellTimers[postId]);
						delete dwellTimers[postId];
					}
				}
			});
		}, { threshold: 0.5 });

		container.querySelectorAll(".feed_item[data-feed-post-id]").forEach(function(el) {
			observer.observe(el);
		});
	}

	/* ── Reader Nav Badge ─────────────────────────────────────── */
	function attachReaderBadge() {
		var link = document.getElementById("reader_link");
		if (!link) return;
		fetch("/api/reader/unread_count", { credentials: "same-origin" })
			.then(function(r) { return r.json(); })
			.then(function(data) {
				var count = data && data.count ? data.count : 0;
				if (count <= 0) return;
				var badge = document.createElement("span");
				badge.className = "reader_nav_badge";
				badge.textContent = count > 99 ? "99+" : String(count);
				link.appendChild(badge);
			})
			.catch(function() {});
	}

	function init() {
		attachGlobalKeyboard();
		attachLocalTools();
		attachCommentPower();
		attachMediaFallbacks();
		attachSettingsProfiles();
		refreshPaletteCommands();
		initSidebar();
		attachInlineFeedManage();
		attachLayoutSwitcher();
		attachPostStatus();
		attachFeedFilter();
		attachFeedCardClick();
		attachFeedKeyNav();
		attachScrollToRead();
		attachReaderBadge();
	}

	if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
	else init();
})();
