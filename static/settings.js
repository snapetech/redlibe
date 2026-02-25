(function() {
	var search = document.getElementById("settings_search");
	var container = document.getElementById("settings");
	if (!search || !container) return;

	function filter() {
		var q = (search.value || "").trim().toLowerCase();
		container.querySelectorAll("fieldset").forEach(function(fs) {
			var legend = (fs.querySelector("legend") && fs.querySelector("legend").textContent) || "";
			var groups = fs.querySelectorAll(".prefs-group");
			var anyVisible = false;
			groups.forEach(function(grp) {
				var label = (grp.querySelector("label") && grp.querySelector("label").textContent) || "";
				var match = !q || legend.toLowerCase().indexOf(q) >= 0 || label.toLowerCase().indexOf(q) >= 0;
				grp.style.display = match ? "" : "none";
				if (match) anyVisible = true;
			});
			fs.style.display = anyVisible ? "" : "none";
		});
	}

	search.addEventListener("input", filter);
	search.addEventListener("search", filter);
})();

(function() {
	var select = document.getElementById("theme");
	var grid = document.getElementById("theme_grid");
	if (!select || !grid) return;

	var form = select.form;
	var saveBtn = document.getElementById("save");
	var autosaveTimer = null;
	var themeValues = Array.prototype.map.call(select.options || [], function(opt) {
		return opt && opt.value ? opt.value : "";
	}).filter(function(value, index, arr) {
		return value && value !== "system" && arr.indexOf(value) === index;
	});

	function prettyThemeName(value) {
		if (!value) return "";
		return value
			.replace(/([a-z])([A-Z])/g, "$1 $2")
			.replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
			.replace(/[-_]+/g, " ")
			.replace(/\b\w/g, function(c) { return c.toUpperCase(); });
	}

	function applyThemePreview(value) {
		var body = document.body;
		if (!body) return;
		themeValues.forEach(function(themeName) {
			body.classList.remove(themeName);
		});
		if (value && value !== "system") {
			body.classList.add(value);
		}
	}

	function syncThemeSelection() {
		var value = select.value || "system";
		applyThemePreview(value);
		grid.querySelectorAll(".theme_tile").forEach(function(tile) {
			var selected = (tile.getAttribute("data-theme-name") || "") === value;
			tile.classList.toggle("selected", selected);
			tile.setAttribute("aria-pressed", selected ? "true" : "false");
		});
	}

	grid.querySelectorAll(".theme_tile").forEach(function(tile) {
		var label = tile.querySelector(".theme_tile_label");
		if (label) label.textContent = prettyThemeName(label.textContent);
		tile.addEventListener("click", function() {
			var theme = tile.getAttribute("data-theme-name") || "";
			if (!theme) return;
			select.value = theme;
			syncThemeSelection();
			select.dispatchEvent(new Event("change", { bubbles: true }));
			if (saveBtn) {
				saveBtn.value = "Saving theme...";
				saveBtn.disabled = true;
			}
			if (form) {
				if (autosaveTimer) window.clearTimeout(autosaveTimer);
				autosaveTimer = window.setTimeout(function() {
					if (form.requestSubmit) form.requestSubmit();
					else form.submit();
				}, 120);
			}
		});
	});

	select.addEventListener("change", syncThemeSelection);
	syncThemeSelection();
})();

(function() {
	var btn = document.getElementById("clear_offline_cache_btn");
	if (!btn) return;
	btn.addEventListener("click", function() {
		var done = function() {
			btn.textContent = "Cache cleared";
			setTimeout(function() { window.location.reload(); }, 800);
		};
		if (!window.caches) { done(); return; }
		caches.keys().then(function(keys) {
			return Promise.all(keys.map(function(k) { return caches.delete(k); }));
		}).then(function() {
			if (navigator.serviceWorker && navigator.serviceWorker.getRegistrations) {
				return navigator.serviceWorker.getRegistrations().then(function(regs) {
					return Promise.all(regs.map(function(r) { return r.unregister(); }));
				});
			}
		}).then(done).catch(done);
	});
})();
