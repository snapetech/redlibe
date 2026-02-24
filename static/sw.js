/* Redlib offline cache: caches static assets for use when offline or slow. */
const CACHE_NAME = 'redlib-static-v1';
const STATIC_URLS = [
	'/style.css',
	'/favicon.ico',
	'/manifest.json',
	'/Inter.var.woff2',
	'/logo.png',
	'/touch-icon-iphone.png',
	'/apple-touch-icon.png',
	'/opensearch.xml',
	'/copy.js',
	'/playHLSVideo.js',
	'/hls.min.js',
	'/highlighted.js',
	'/check_update.js'
];

self.addEventListener('install', function (event) {
	event.waitUntil(
		caches.open(CACHE_NAME).then(function (cache) {
			return cache.addAll(STATIC_URLS.map(function (u) { return new Request(u, { cache: 'reload' }); }));
		}).then(function () { return self.skipWaiting(); }).catch(function () {})
	);
});

self.addEventListener('activate', function (event) {
	event.waitUntil(
		caches.keys().then(function (names) {
			return Promise.all(
				names.filter(function (name) { return name !== CACHE_NAME; }).map(function (name) { return caches.delete(name); })
			);
		}).then(function () { return self.clients.claim(); })
	);
});

self.addEventListener('fetch', function (event) {
	if (event.request.method !== 'GET') return;
	var url = new URL(event.request.url);
	if (url.origin !== self.location.origin) return;
	var path = url.pathname;
	var isStatic = STATIC_URLS.some(function (s) { return path === s || (s.indexOf('?') === -1 && path.split('?')[0] === s); });
	if (isStatic) {
		event.respondWith(
			caches.match(event.request).then(function (cached) {
				return cached || fetch(event.request).then(function (res) {
					var clone = res.clone();
					caches.open(CACHE_NAME).then(function (cache) { cache.put(event.request, clone); });
					return res;
				});
			})
		);
	}
});
