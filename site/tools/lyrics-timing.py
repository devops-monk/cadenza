#!/usr/bin/env python3
# Timed lyrics for the example songs in site/main.js, worked out from the
# score each song keeps in the app's library: the ABC header gives the tempo,
# the % comments the sections and the vocal voice every sung note. Each lyric
# line starts on the note its share of syllables reaches in its section.
# The model often makes the opening shorter than the score plans and keeps the
# tempo after it, so every line is then moved by the offset measured in the
# audio (needs ffmpeg and numpy).
#   Usage: lyrics-timing.py "<library index.json>" <track id prefix or title> [...]
import json
import os
import re
import subprocess
import sys
from fractions import Fraction

NOTE = re.compile(r'"[^"]*"|!\w+!|\[[^\]]*\]|([_^=]*)([A-Ga-gzZx])([,\']*)(\d*)(/*)(\d*)')


def section_key(name):
    name = re.sub(r"[^a-z]", "", name.lower())
    return re.sub(r"\d+$", "", name)


def read_score(abc):
    """[(section, [note start seconds]), ...] and the score's length in seconds."""
    header = dict(re.findall(r"^([A-Z]):\s*(.*)$", abc, re.M))
    unit = Fraction(header.get("L", "1/8"))
    beat, bpm = header.get("Q", "1/4=120").split("=")
    seconds = unit / Fraction(beat) * 60 / Fraction(bpm)
    meter = header.get("M", "4/4")
    bar = Fraction(meter) / unit if "/" in meter else Fraction(16)
    sections, voice, time = [], None, Fraction(0)
    for line in abc.splitlines():
        line = line.strip()
        if line.startswith("%"):
            sections.append([line.lstrip("% ").strip(), []])
        elif line.startswith("V:"):
            voice = line[2:].strip().split()[0]
        elif voice == "Vocal" and sections and line and not re.match(r"^[A-Z]:", line):
            for m in NOTE.finditer(line):
                if not m.group(2):
                    continue
                pitch, num, slashes, den = m.group(2), m.group(4), m.group(5), m.group(6)
                if pitch == "Z":
                    time += bar * int(num or 1)
                    continue
                length = Fraction(int(num) if num else 1)
                if slashes:
                    length /= int(den) if den else 2 ** len(slashes)
                if pitch not in "zx":
                    sections[-1][1].append(float(time * seconds))
                time += length
    return sections, float(time * seconds)


def read_lyrics(text):
    """[(section, [lines]), ...] from lyrics marked [Verse], [Chorus], ..."""
    parts = []
    for line in text.splitlines():
        line = line.strip()
        tag = re.match(r"^\[(.+)\]$", line)
        if tag:
            parts.append([tag.group(1), []])
        elif line and parts:
            parts[-1][1].append(line)
    return [p for p in parts if p[1]]


def syllables(line):
    return max(1, len(re.findall(r"[aeiouyAEIOUY]+", line)) or len(line) // 3)


def timed(track, shift=0.0):
    """([[seconds, line, starts_section], ...], synced, score seconds).

    Synced when every lyric section has a scored section of the same name, in
    order; otherwise the lines are only spread over the notes, for the cut-off
    below, and the page shows them untimed. Lines the song ends before are dropped."""
    sections, length = read_score(track["abc"])
    sung = [s for s in sections if s[1]]
    parts = read_lyrics(track["lyrics"])
    picks, cursor = [], 0
    for name, _ in parts:
        pick = next((i for i in range(cursor, len(sung)) if section_key(sung[i][0]) == section_key(name)), None)
        if pick is None:
            break
        picks.append(pick)
        cursor = pick + 1
    synced = len(picks) == len(parts)
    if synced:
        groups = [(sung[pick][1], lines) for pick, (_, lines) in zip(picks, parts)]
    else:
        groups = [([t for _, notes in sung for t in notes], [l for _, lines in parts for l in lines])]
        firsts = [int(i == 0) for _, lines in parts for i in range(len(lines))]
    out = []
    for notes, lines in groups:
        weights = [syllables(l) for l in lines]
        total, done = sum(weights), 0
        for index, (line, weight) in enumerate(zip(lines, weights)):
            start = notes[min(len(notes) - 1, round(done / total * len(notes)))]
            first = int(index == 0) if synced else firsts[index]
            out.append([round(max(0.0, start - shift), 2), line, first])
            done += weight
    out = [line for line in out if line[0] < track["duration"] - 1]
    return out, synced, length


HOP = 0.05  # seconds per frame of the curves below


def voice_curve(path):
    """Where a voice is likely, frame by frame: centre against sides, 250-4000 Hz."""
    import numpy as np
    rate = 16000
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-ac", "2", "-ar", str(rate), "-f", "f32le", "-"],
                         check=True, capture_output=True).stdout
    x = np.frombuffer(raw, np.float32).reshape(-1, 2)
    mid, side = (x[:, 0] + x[:, 1]) / 2, (x[:, 0] - x[:, 1]) / 2
    size, hop = 1024, int(rate * HOP)
    window = np.hanning(size)
    freqs = np.fft.rfftfreq(size, 1 / rate)
    band = (freqs > 250) & (freqs < 4000)
    curve = []
    for i in range(0, len(mid) - size, hop):
        m = (np.abs(np.fft.rfft(mid[i:i + size] * window))[band] ** 2).sum()
        s = (np.abs(np.fft.rfft(side[i:i + size] * window))[band] ** 2).sum()
        curve.append(np.log(m + 1e-9) - np.log(s + 1e-9))
    curve = np.convolve(np.array(curve), np.ones(6) / 6, "same")
    return (curve - curve.mean()) / (curve.std() + 1e-9)


def best_lag(voice, notes, start, end):
    """Seconds the singing comes before the score, for frames [start, end)."""
    import numpy as np
    lags = range(int(-10 / HOP), int(6 / HOP) + 1)
    scores = [np.mean(voice[start:end] * notes[start + k:end + k]) if start + k >= 0 and end + k <= len(notes) else -9 for k in lags]
    return lags[int(np.argmax(scores))] * HOP


def measured_shift(path, abc):
    """The offset when the whole song and most 45 s windows of it agree, else 0."""
    import numpy as np
    voice = voice_curve(path)
    notes = np.zeros(len(voice))
    for _, starts in read_score(abc)[0]:
        for t in starts:
            notes[int(t / HOP):int((t + 0.5) / HOP)] = 1
    notes = (notes - notes.mean()) / (notes.std() + 1e-9)
    whole = best_lag(voice, notes, int(10 / HOP), len(voice) - int(6 / HOP))
    windows = [best_lag(voice, notes, s, s + int(45 / HOP)) for s in range(int(10 / HOP), len(voice) - int(51 / HOP), int(30 / HOP))]
    agree = sum(abs(w - whole) <= 0.4 for w in windows)
    return (round(whole, 2) if windows and agree * 2 >= len(windows) else 0.0), [round(w, 2) for w in windows]


def main():
    library = json.load(open(sys.argv[1]))
    folder = os.path.dirname(os.path.abspath(sys.argv[1]))
    for key in sys.argv[2:]:
        track = next(t for t in library if t["id"].startswith(key) or t["title"] == key)
        shift, windows = measured_shift(os.path.join(folder, track["file"]), track["abc"])
        lines, synced, length = timed(track, shift)
        print(json.dumps({"id": track["id"][:8], "title": track["title"], "synced": synced, "audio": round(track["duration"], 1),
                          "score": round(length, 1), "shift": shift, "windows": windows, "lines": lines}, ensure_ascii=False))


if __name__ == "__main__":
    main()
