/**
 * V15 P1-20-1 Service Worker - PWA 离线缓存支持
 *
 * 缓存策略：
 * - 静态资源（JS/CSS/字体/图片）：Cache First（首屏关键资源优先）
 * - API 请求：Network First（网络优先，失败回退缓存）
 * - HTML 导航请求：Network First（保证最新版本，离线时回退 index.html）
 *
 * 版本管理：CACHE_VERSION 更新时自动清理旧缓存
 */

const CACHE_VERSION = 'v15-p1-v1';
const STATIC_CACHE = `bx-erp-static-${CACHE_VERSION}`;
const RUNTIME_CACHE = `bx-erp-runtime-${CACHE_VERSION}`;
const OFFLINE_URL = '/index.html';

// 首屏关键资源（构建后由 Vite 注入 hash，这里仅缓存基础路径）
const PRECACHE_URLS = [
  '/',
  '/index.html',
  '/favicon.ico',
  '/manifest.json',
  '/robots.txt',
];

// 需要缓存的静态资源后缀
const STATIC_ASSETS_REGEX = /\.(?:js|css|woff2?|ttf|eot|otf|png|jpg|jpeg|gif|svg|ico)$/i;

// 不缓存的 API 路径（实时性要求高）
const NO_CACHE_API_REGEX = /\/api\/(?:auth|upload|export|stream|webhook)/i;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(STATIC_CACHE)
      .then((cache) => cache.addAll(PRECACHE_URLS))
      .then(() => self.skipWaiting())
      .catch((err) => {
        // 预缓存失败不阻塞安装（部分资源可能不存在）
        console.warn('[SW] precache failed:', err);
        return self.skipWaiting();
      })
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((cacheNames) =>
        Promise.all(
          cacheNames
            .filter((name) => name !== STATIC_CACHE && name !== RUNTIME_CACHE)
            .map((name) => caches.delete(name))
        )
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const { request } = event;

  // 仅处理 GET 请求
  if (request.method !== 'GET') {
    return;
  }

  const url = new URL(request.url);

  // 同源请求才缓存
  if (url.origin !== self.location.origin) {
    return;
  }

  // HTML 导航请求：Network First（保证最新版本，离线回退 index.html）
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((response) => {
          const copy = response.clone();
          caches.open(RUNTIME_CACHE).then((cache) => cache.put(request, copy));
          return response;
        })
        .catch(() =>
          caches.match(request).then((cached) => cached || caches.match(OFFLINE_URL))
        )
    );
    return;
  }

  // API 请求：Network First（实时性要求，失败回退缓存）
  if (url.pathname.startsWith('/api/')) {
    // 不缓存敏感 API（导出/上传/流式/Webhook）
    if (NO_CACHE_API_REGEX.test(url.pathname)) {
      return;
    }
    event.respondWith(
      fetch(request)
        .then((response) => {
          if (response.ok && response.status === 200) {
            const copy = response.clone();
            caches.open(RUNTIME_CACHE).then((cache) => cache.put(request, copy));
          }
          return response;
        })
        .catch(() => caches.match(request))
    );
    return;
  }

  // 静态资源：Cache First（命中直接返回，未命中回退网络并缓存）
  if (STATIC_ASSETS_REGEX.test(url.pathname)) {
    event.respondWith(
      caches.match(request).then((cached) => {
        if (cached) {
          return cached;
        }
        return fetch(request).then((response) => {
          if (response.ok && response.status === 200) {
            const copy = response.clone();
            caches.open(STATIC_CACHE).then((cache) => cache.put(request, copy));
          }
          return response;
        });
      })
    );
    return;
  }
});

// 接收前端消息：手动触发更新
self.addEventListener('message', (event) => {
  if (event.data === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});
