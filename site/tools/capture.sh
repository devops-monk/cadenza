#!/bin/bash
# Screenshots the real Cadenza UI (ui/) running on the mock backend, for the
# website. Needs Google Chrome and python3. Output: site/images/app-*.png
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
chrome="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
work="$(mktemp -d)"
trap 'kill "$server" 2>/dev/null || true; rm -rf "$work"' EXIT

cp -R "$root/ui/." "$work/"
cp "$root/site/tools/mock-tauri.js" "$work/"
# the mock must exist before app.js runs
perl -pi -e 's#<script type="module" src="app.js"></script>#<script src="mock-tauri.js"></script><script type="module" src="app.js"></script>#' "$work/index.html"

port=8765
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$work" >/dev/null 2>&1 &
server=$!
sleep 1

mkdir -p "$root/site/images"
shot() {
    local scene="$1" name="$2" size="${3:-1440,900}"
    "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
        --autoplay-policy=no-user-gesture-required --window-size="$size" \
        --virtual-time-budget=5000 --screenshot="$root/site/images/$name.png" \
        "http://127.0.0.1:$port/index.html#$scene" >/dev/null 2>&1
    echo "site/images/$name.png"
}
shot create app-create
shot create,dark app-create-dark
shot library app-library
shot drawer app-lyrics
shot models app-models
shot home app-home
shot settings app-settings
