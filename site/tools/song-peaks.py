#!/usr/bin/env python3
# The waveform of an example song for the SONGS list in site/main.js: 96 bar
# heights from 8 to 100. Needs ffmpeg.   Usage: song-peaks.py site/audio/*.mp3
import array
import math
import subprocess
import sys

BARS = 96

for path in sys.argv[1:]:
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-ac", "1", "-ar", "8000", "-f", "s16le", "-"],
        check=True, capture_output=True,
    ).stdout
    samples = array.array("h", raw[: len(raw) - len(raw) % 2])
    size = len(samples) // BARS
    levels = [math.sqrt(sum(s * s for s in samples[i * size : (i + 1) * size]) / size) for i in range(BARS)]
    top = max(levels) or 1
    print(path, [max(8, round(100 * (level / top) ** 0.8)) for level in levels])
