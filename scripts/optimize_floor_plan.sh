#!/usr/bin/env bash
# Optimize the stored floor-plan SVG (the source is ~2.5 MB, which dominates
# floor-plan load time). Extracts the SVG from the Pi's DB, runs SVGO with a
# conservative config (keeps viewBox and element ids so device positions still
# line up), and uploads the smaller SVG back through the API.
#
# Run this FROM YOUR MAC (needs node/npx locally; talks to the Pi over ssh).
# Requires: ssh access to the Pi host, python3, node/npx.
set -euo pipefail

PI="${PI:-gholam}"
DB="${DB:-/var/lib/home-automation-rs/database/home_automation.db}"
API="${API:-http://localhost:8082/api/floor-plan}"   # resolved on the Pi
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "[1/4] Extracting SVG from $PI:$DB …"
ssh "$PI" "sudo sqlite3 '$DB' \"SELECT writefile('/tmp/floor_plan_src.svg', svg_content) FROM floor_plan;\" >/dev/null && cat /tmp/floor_plan_src.svg && sudo rm -f /tmp/floor_plan_src.svg" > "$WORK/floor.svg"
echo "      original: $(wc -c < "$WORK/floor.svg") bytes"

# NOTE: SVGO only helps *vector* SVGs. If the floor plan is a single embedded
# raster (<image> with a base64 data URI), SVGO barely reduces it (~3%) — the
# real lever there is re-exporting/recompressing the raster (e.g. WebP). gzip +
# ETag caching already make delivery cheap regardless.
echo "[2/4] Optimizing with SVGO (viewBox & ids preserved)…"
# SVGO v4: viewBox is kept by default (removeViewBox is no longer in preset-default).
cat > "$WORK/svgo.config.mjs" <<'CFG'
export default {
  multipass: true,
  plugins: [
    { name: 'preset-default', params: { overrides: {
      cleanupIds: false,         // keep ids that may be referenced
      removeHiddenElems: false,  // keep layers even if hidden
    } } },
  ],
};
CFG
npx --yes svgo --config "$WORK/svgo.config.mjs" -i "$WORK/floor.svg" -o "$WORK/floor.min.svg"
echo "      optimized: $(wc -c < "$WORK/floor.min.svg") bytes"

echo "[3/4] Uploading optimized SVG back through the API…"
python3 -c "import json,sys; print(json.dumps({'svg_content': open(sys.argv[1], encoding='utf-8').read()}))" "$WORK/floor.min.svg" \
  | ssh "$PI" "curl -sS -X POST '$API' -H 'Content-Type: application/json' --data-binary @- -o /dev/null -w 'HTTP %{http_code}\n'"

echo "[4/4] Done. Clients will refetch the smaller SVG (ETag changed)."
