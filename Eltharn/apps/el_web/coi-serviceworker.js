"use strict";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (ev) => ev.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (ev) => {
  const req = ev.request;
  if (req.cache === "only-if-cached" && req.mode !== "same-origin") return;

  ev.respondWith(
    fetch(req, { cache: "no-cache" }).then((resp) => {
      if (resp.status === 0) return resp;

      const headers = new Headers(resp.headers);
      headers.set("Cross-Origin-Opener-Policy", "same-origin");
      headers.set("Cross-Origin-Embedder-Policy", "require-corp");
      headers.set("Cross-Origin-Resource-Policy", "cross-origin");
      headers.set("Cache-Control", "no-cache, no-store, must-revalidate");

      const serverCT = resp.headers.get("Content-Type");
      if (serverCT) headers.set("Content-Type", serverCT);

      return new Response(resp.body, {
        status: resp.status,
        statusText: resp.statusText,
        headers,
      });
    })
  );
});
