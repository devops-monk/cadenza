#!/bin/bash
# Records the website's hero clip: the real Cadenza UI on the mock backend,
# a song playing while another is being composed. Headless Chrome renders
# each frame at an exact virtual time; ffmpeg joins them into a looping
# video. Needs Google Chrome, python3 and ffmpeg.
# Output: site/images/app-playing.{mp4,webm,jpg}; THEMES="light dark" adds
# app-playing-dark.* too. Each frame is a fresh Chrome replaying the page up
# to its moment, so a theme takes about 20 minutes.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
chrome="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
fps="${FPS:-20}"
seconds="${SECONDS_LONG:-8}"
jobs="${JOBS:-6}"
work="$(mktemp -d)"
port=18767
trap 'kill "$server" 2>/dev/null || true; rm -rf "$work"' EXIT

mkdir -p "$work/www"
cp -R "$root/ui/." "$work/www/"
cp "$root/site/tools/mock-tauri.js" "$work/www/"
perl -pi -e 's#<script type="module" src="app.js"></script>#<script src="mock-tauri.js"></script><script type="module" src="app.js"></script>#' "$work/www/index.html"
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$work/www" >/dev/null 2>&1 &
server=$!
sleep 1

frame() { # theme index
    local theme="$1" i="$2"
    # Boot and staging take about 2.2 s of virtual time; frames start after.
    local budget=$(( 2600 + i * 1000 / fps ))
    local png="$work/$theme/$(printf %04d "$i").png"
    # Now and then a headless Chrome never finishes: give each try 90 s.
    for attempt in 1 2 3; do
        "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
            --user-data-dir="$work/profile-$theme-$i-$attempt" --window-size=1440,900 \
            --virtual-time-budget="$budget" --screenshot="$png" \
            "http://127.0.0.1:$port/index.html#create,$theme,rec" >/dev/null 2>&1 &
        local pid=$!
        for _ in $(seq 90); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done
        kill "$pid" 2>/dev/null || true
        rm -rf "$work/profile-$theme-$i-$attempt"
        [ -s "$png" ] && return 0
    done
    echo "frame $i of $theme failed three times" >&2
    return 1
}
export -f frame
export chrome work port fps

count=$(( fps * seconds ))
for theme in ${THEMES:-light}; do
    mkdir -p "$work/$theme"
    seq 0 $(( count - 1 )) | xargs -P "$jobs" -I{} bash -c "frame $theme {}"
    suffix=$([ "$theme" = dark ] && echo "-dark" || true)
    out="$root/site/images/app-playing$suffix"
    # 1920 wide: sharp on most screens, a fraction of the 2x frames' weight.
    ffmpeg -loglevel error -y -framerate "$fps" -i "$work/$theme/%04d.png" \
        -vf "scale=1920:-2:flags=lanczos,format=yuv420p" -c:v libx264 -preset slow -crf 22 -movflags +faststart -an "$out.mp4"
    ffmpeg -loglevel error -y -framerate "$fps" -i "$work/$theme/%04d.png" \
        -vf "scale=1920:-2:flags=lanczos" -c:v libvpx-vp9 -b:v 0 -crf 36 -row-mt 1 -an "$out.webm"
    ffmpeg -loglevel error -y -i "$work/$theme/0000.png" -vf "scale=1920:-2:flags=lanczos" "$out.jpg"
    ls -la "$out".*
done
