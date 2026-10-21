#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "$CRATE_DIR/../../.." && pwd)"
CALLER_DIR="$(pwd)"
absolute_path() {
  case "$1" in
    /*) realpath -m -- "$1" ;;
    *) realpath -m -- "$CALLER_DIR/$1" ;;
  esac
}
EFFECTIVE_TARGET_DIR="$(absolute_path "${CARGO_TARGET_DIR:-$WORKSPACE/../.eltharn-build}")"
DIST_DIR="$(absolute_path "${1:-$CRATE_DIR/dist}")"
case "$DIST_DIR" in
  "$WORKSPACE/"*) ;;
  *) echo "Web output must be inside the workspace: $DIST_DIR" >&2; exit 2 ;;
esac
if [[ -L "$DIST_DIR" || ( -e "$DIST_DIR" && ! -f "$DIST_DIR/.eltharn-web-bundle" ) ]]; then
  echo "Refusing to replace an unmarked output directory: $DIST_DIR" >&2
  exit 2
fi

if [[ -z "${ELTHARN_WEB_CONFIG:-}" ]]; then
  echo "Set ELTHARN_WEB_CONFIG to a deployment JSON file or an eltharn --print-backends JSON export" >&2
  exit 2
fi
CONFIG_FILE="$(absolute_path "$ELTHARN_WEB_CONFIG")"
if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "Web deployment configuration does not exist: $CONFIG_FILE" >&2
  exit 2
fi
if ! command -v node >/dev/null 2>&1; then
  echo "Node.js is required to validate the Web deployment configuration" >&2
  exit 2
fi

EXPECTED_WB_VERSION="$(awk '/^name = "wasm-bindgen"$/ { found=1; next } found && /^version = / { gsub(/"/, "", $3); print $3; exit }' "$WORKSPACE/Cargo.lock")"
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen CLI $EXPECTED_WB_VERSION is required" >&2
  exit 2
fi
if [[ "$(wasm-bindgen --version | awk '{print $2}')" != "$EXPECTED_WB_VERSION" ]]; then
  echo "wasm-bindgen CLI must match $EXPECTED_WB_VERSION in Cargo.lock" >&2
  exit 2
fi
mkdir -p -- "$(dirname "$DIST_DIR")"
STAGING_DIR="$(mktemp -d "$(dirname "$DIST_DIR")/.eltharn-web-XXXXXXXX")"
trap 'rm -rf -- "$STAGING_DIR"' EXIT
node - "$CONFIG_FILE" "$STAGING_DIR/eltharn_gate-config.js" <<'JS'
const fs = require("node:fs");
const value = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const config = Array.isArray(value) ? {backend_manifests: value} : value;
if (!config || typeof config !== "object" || Array.isArray(config)) {
  throw new Error("Web deployment configuration must be an object or a backend manifest array");
}
if (!Array.isArray(config.backend_manifests) || config.backend_manifests.length === 0) {
  throw new Error("Provide at least one deployed backend manifest");
}
const versions = new Set();
for (const manifest of config.backend_manifests) {
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
    throw new Error("Each backend manifest must be an object");
  }
  for (const key of ["backend_name", "minecraft_version", "protocol_source", "assets_compatibility"]) {
    if (typeof manifest[key] !== "string" || manifest[key].trim().length === 0) {
      throw new Error(`Backend manifest requires a nonempty ${key}`);
    }
  }
  for (const key of ["ipc_version", "protocol_version"]) {
    if (!Number.isSafeInteger(manifest[key]) || manifest[key] <= 0) {
      throw new Error(`Backend manifest requires a positive integer ${key}`);
    }
  }
  if (manifest.executable == null) manifest.executable = "";
  if (typeof manifest.executable !== "string") {
    throw new Error("Backend manifest executable must be a string when provided");
  }
  if (versions.has(manifest.minecraft_version)) {
    throw new Error(`Duplicate deployed protocol: ${manifest.minecraft_version}`);
  }
  versions.add(manifest.minecraft_version);
}
for (const key of ["default_protocol", "gateway_url"]) {
  if (config[key] !== undefined && typeof config[key] !== "string") {
    throw new Error(`${key} must be a string`);
  }
}
if (config.default_protocol && !versions.has(config.default_protocol)) {
  throw new Error("default_protocol must name a deployed backend");
}
config.default_protocol ||= config.backend_manifests[0].minecraft_version;
fs.writeFileSync(process.argv[3], `window.ELTHARN_GATE_CONFIG = Object.freeze(${JSON.stringify(config, null, 2)});\n`);
JS
cd "$WORKSPACE"
env -u CARGO_ENCODED_RUSTFLAGS \
  RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--max-memory=1073741824 -C link-arg=--import-memory -C link-arg=--export=__heap_base -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base" \
  cargo build --locked -p el_web --target-dir "$EFFECTIVE_TARGET_DIR" --target wasm32-unknown-unknown --release \
  -Z build-std=std,panic_abort
wasm-bindgen "$EFFECTIVE_TARGET_DIR/wasm32-unknown-unknown/release/el_web.wasm" \
  --target web --out-dir "$STAGING_DIR" --out-name eltharn_gate
sed -i "s|import('../../..')|import('../../../eltharn_gate.js')|g" \
  "$STAGING_DIR"/snippets/wasm-bindgen-rayon-*/src/workerHelpers.js
for name in index.html favicon.svg eltharn_gate.css eltharn_gate-boot.js eltharn_gate-main.js extensions.js shader_settings.mjs coi-serviceworker.js; do
  cp -- "$CRATE_DIR/$name" "$STAGING_DIR/$name"
done
env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS \
  cargo build --release --locked -p el_mod_web --target-dir "$EFFECTIVE_TARGET_DIR" --target wasm32-unknown-unknown
mkdir -p -- "$STAGING_DIR/mods"
wasm-bindgen "$EFFECTIVE_TARGET_DIR/wasm32-unknown-unknown/release/el_mod_web.wasm" \
  --target web --out-dir "$STAGING_DIR/mods" --out-name eltharn_mod_web
cp -- "$WORKSPACE"/Eltharn/crates/el_mod_web/browser/*.mjs "$STAGING_DIR/mods/"
touch "$STAGING_DIR/.eltharn-web-bundle"
if [[ -d "$DIST_DIR" ]]; then
  BACKUP_DIR="$(mktemp -d "$(dirname "$DIST_DIR")/.eltharn-web-previous-XXXXXXXX")"
  rmdir -- "$BACKUP_DIR"
  mv -- "$DIST_DIR" "$BACKUP_DIR"
  if ! mv -- "$STAGING_DIR" "$DIST_DIR"; then
    mv -- "$BACKUP_DIR" "$DIST_DIR"
    exit 1
  fi
  rm -rf -- "$BACKUP_DIR"
else
  mv -- "$STAGING_DIR" "$DIST_DIR"
fi
echo "WASM bundle written to $DIST_DIR"
echo "Game resources are not bundled. Configure the deployment resource provider before gameplay."
