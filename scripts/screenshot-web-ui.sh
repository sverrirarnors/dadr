#!/usr/bin/env bash
# Regenerate the web UI screenshot on the docs landing page,
# docs-src/assets/dadrock-review-{light,dark}.webp.
#
# Usage:
#   ./scripts/screenshot-web-ui.sh
#
# It builds dadrock, seeds the candidate-review demo (the one `make demo`
# opens) in a throwaway directory, serves it with `dadrock preview` on a free
# port, and captures /inbox with headless Chromium in both color schemes: a
# 1200x780 viewport at 2x, tall enough to show the whole open record. Run
# `make build` afterwards to copy the images into docs/.
#
# Needs Bun (so the web UI is embedded in the binary), Chromium or Chrome
# ($CHROME overrides the lookup), and cwebp or ImageMagick.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

out_dir="$root/docs-src/assets"
width=1200
height=780

chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for candidate in chromium chromium-browser google-chrome google-chrome-stable \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      chrome="$candidate"
      break
    fi
  done
fi
if [ -z "$chrome" ]; then
  echo "Chromium or Chrome not found; set CHROME to its path" >&2
  exit 1
fi

if command -v cwebp >/dev/null 2>&1; then
  to_webp() { cwebp -quiet -q 85 "$1" -o "$2"; }
elif command -v magick >/dev/null 2>&1; then
  to_webp() { magick "$1" -quality 85 "$2"; }
else
  echo "cwebp or ImageMagick (magick) is required to write WebP" >&2
  exit 1
fi

cargo build --quiet -p dadrock --bin dadrock
dadrock="$root/target/debug/dadrock"

work="$(mktemp -d)"
server=""
cleanup() {
  if [ -n "$server" ]; then kill "$server" 2>/dev/null || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

"$dadrock" demo --example candidate-review "$work/demo" >/dev/null
"$dadrock" preview --port 0 "$work/demo" >/dev/null 2>"$work/preview.log" &
server=$!

url=""
for _ in $(seq 1 100); do
  url="$(sed -n 's/^dadrock-preview: listening on //p' "$work/preview.log")"
  [ -n "$url" ] && break
  kill -0 "$server" 2>/dev/null || break
  sleep 0.1
done
if [ -z "$url" ]; then
  echo "dadrock preview did not start:" >&2
  cat "$work/preview.log" >&2
  exit 1
fi

# Without Bun the binary embeds no web UI and every route is a notice page.
if ! curl -fsS "$url/" | grep -q '/_app/'; then
  echo "dadrock was built without the web UI; install Bun and rerun" >&2
  exit 1
fi

# preferredColorScheme: 0 is dark, 1 is light. The page follows the system
# scheme because the throwaway profile has no stored dadr.theme.
for scheme in light:1 dark:0; do
  name="${scheme%%:*}"
  "$chrome" --headless=new --hide-scrollbars --no-first-run \
    --user-data-dir="$work/profile-$name" \
    --force-device-scale-factor=2 --window-size="$width,$height" \
    --virtual-time-budget=5000 \
    --blink-settings=preferredColorScheme="${scheme#*:}" \
    --screenshot="$work/$name.png" "$url/inbox" >/dev/null 2>&1
  to_webp "$work/$name.png" "$out_dir/dadrock-review-$name.webp"
  echo "wrote docs-src/assets/dadrock-review-$name.webp"
done
