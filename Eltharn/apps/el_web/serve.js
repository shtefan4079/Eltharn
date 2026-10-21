#!/usr/bin/env node
const http = require("http");
const fs   = require("fs");
const path = require("path");

const PORT     = parseInt(process.argv[2] ?? "8081", 10);
const DIST_DIR = path.resolve(process.argv[3] ?? "./dist");

const MIME = {
  ".html"  : "text/html; charset=utf-8",
  ".js"    : "application/javascript; charset=utf-8",
  ".mjs"   : "application/javascript; charset=utf-8",
  ".wasm"  : "application/wasm",
  ".css"   : "text/css; charset=utf-8",
  ".zip"   : "application/zip",
  ".png"   : "image/png",
  ".jpg"   : "image/jpeg",
  ".svg"   : "image/svg+xml",
  ".ico"   : "image/x-icon",
  ".json"  : "application/json; charset=utf-8",
  ".txt"   : "text/plain; charset=utf-8",
};

const COI_HEADERS = {
  "Cross-Origin-Opener-Policy"    : "same-origin",
  "Cross-Origin-Embedder-Policy"  : "require-corp",
  "Cross-Origin-Resource-Policy"  : "same-origin",
  "Cache-Control"                  : "no-cache",
};

const server = http.createServer((req, res) => {
  let urlPath = req.url.split("?")[0];                      
  if (urlPath === "/" || urlPath === "") urlPath = "/index.html";

  const filePath = path.join(DIST_DIR, urlPath);

  const relative = path.relative(DIST_DIR, filePath);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    res.writeHead(403, COI_HEADERS);
    res.end("Forbidden");
    return;
  }

  fs.stat(filePath, (err, stat) => {
    if (err || !stat.isFile()) {
      const fallback = path.join(DIST_DIR, "index.html");
      fs.createReadStream(fallback)
        .on("error", () => {
          res.writeHead(404, COI_HEADERS);
          res.end("Not Found");
        })
        .pipe(res.writeHead(200, { "Content-Type": MIME[".html"], ...COI_HEADERS }) || res);
      return;
    }

    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME[ext] ?? "application/octet-stream";

    res.writeHead(200, { "Content-Type": contentType, ...COI_HEADERS });
    fs.createReadStream(filePath).pipe(res);
  });
});

server.listen(PORT, "127.0.0.1", () => {
  console.log(`\nEltharn Gate dev server`);
  console.log(`  http://localhost:${PORT}`);
  console.log(`  Serving: ${DIST_DIR}`);
  console.log(`  COOP+COEP headers: ✓  (crossOriginIsolated will be true)\n`);
});
