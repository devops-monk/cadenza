#!/usr/bin/env python3
# Draws the C4 and sequence diagrams of site/architecture.html as inline SVG
# and writes each between its <!-- diagram:NAME --> markers. Colours come
# from classes in style.css, so the diagrams follow the light and dark theme.
#   Usage: python3 site/tools/diagrams.py
import html
import pathlib
import re

PAGE = pathlib.Path(__file__).resolve().parent.parent / "architecture.html"


def esc(text):
    return html.escape(text, quote=True)


class Svg:
    def __init__(self, name, width, height, title):
        self.name, self.parts = name, []
        self.head = (
            f'<svg class="d" viewBox="0 0 {width} {height}" role="img" aria-labelledby="{name}-t">'
            f'<title id="{name}-t">{esc(title)}</title>'
            f'<defs><marker id="{name}-a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">'
            f'<path class="d-head" d="M0 0 10 5 0 10z"/></marker></defs>'
        )

    def box(self, x, y, w, h, kind, title, tech="", lines=()):
        self.parts.append(f'<rect class="d-{kind}" x="{x}" y="{y}" width="{w}" height="{h}" rx="14"/>')
        cx, ty = x + w / 2, y + 26
        self.parts.append(f'<text class="d-title d-on-{kind}" x="{cx}" y="{ty}" text-anchor="middle">{esc(title)}</text>')
        if tech:
            ty += 17
            self.parts.append(f'<text class="d-tech d-on-{kind}" x="{cx}" y="{ty}" text-anchor="middle">{esc(tech)}</text>')
        ty += 8
        for line in lines:
            ty += 16
            self.parts.append(f'<text class="d-desc d-on-{kind}" x="{cx}" y="{ty}" text-anchor="middle">{esc(line)}</text>')

    def boundary(self, x, y, w, h, label, lx, ly, anchor="start"):
        self.parts.append(f'<rect class="d-boundary" x="{x}" y="{y}" width="{w}" height="{h}" rx="18"/>')
        self.parts.append(f'<text class="d-blabel" x="{lx}" y="{ly}" text-anchor="{anchor}">{esc(label)}</text>')

    def line(self, points, dashed=False):
        d = "M" + " L".join(f"{x} {y}" for x, y in points)
        cls = "d-line d-ret" if dashed else "d-line"
        self.parts.append(f'<path class="{cls}" d="{d}" marker-end="url(#{self.name}-a)"/>')

    def label(self, x, y, text, anchor="middle", rotate=False, muted=False):
        cls = "d-label d-muted" if muted else "d-label"
        turn = f' transform="rotate(-90 {x} {y})"' if rotate else ""
        self.parts.append(f'<text class="{cls}" x="{x}" y="{y}" text-anchor="{anchor}"{turn}>{esc(text)}</text>')

    def raw(self, markup):
        self.parts.append(markup)

    def render(self):
        return self.head + "".join(self.parts) + "</svg>"


def context():
    s = Svg("c4l1", 1000, 350, "C4 level 1, system context: a song maker uses Cadenza, which downloads models from Hugging Face and can ask your own LLM server for lyrics; the app itself comes from GitHub Releases.")
    s.box(380, 10, 240, 110, "person", "Song maker", "[Person]", ["Describes a song, listens,", "saves and remakes it"])
    s.box(740, 10, 240, 110, "ext", "GitHub Releases", "[External system]", ["Installers for macOS,", "Windows and Linux"])
    s.box(380, 220, 240, 120, "system", "Cadenza", "[Software system]", ["Writes, composes and sings", "songs on the user's computer"])
    s.box(20, 220, 240, 120, "ext", "Your LLM server", "[External system, optional]", ["Ollama, LM Studio or any", "OpenAI-compatible API"])
    s.box(740, 220, 240, 120, "ext", "Hugging Face", "[External system]", ["Hosts the model weights", "at pinned revisions"])
    s.line([(500, 120), (500, 218)]); s.label(512, 174, "Uses", "start")
    s.line([(620, 65), (738, 65)]); s.label(679, 57, "Installs from")
    s.line([(620, 280), (738, 280)]); s.label(679, 272, "Downloads models"); s.label(679, 297, "[HTTPS]", muted=True)
    s.line([(380, 280), (262, 280)]); s.label(321, 272, "Asks for lyrics"); s.label(321, 297, "[HTTP, optional]", muted=True)
    return s.render()


def containers():
    s = Svg("c4l2", 1110, 700, "C4 level 2, containers: inside Cadenza, the web UI talks to the Rust core over Tauri IPC; the core runs llama-server and yue-server over HTTP and owns the library and models folders.")
    s.box(50, 10, 240, 90, "person", "Song maker", "[Person]")
    s.box(890, 10, 200, 90, "ext", "Hugging Face", "[External system]")
    s.boundary(20, 130, 850, 560, "Cadenza  [Software system]", 38, 678)
    s.box(50, 160, 260, 110, "container", "Web UI", "[Container: HTML, CSS, JS · web view]", ["Create, Library, Models,", "Settings and the player"])
    s.box(470, 160, 260, 110, "container", "Rust core", "[Container: Rust · Tauri 2]", ["Runs jobs, supervises both", "servers, owns the files"])
    s.box(890, 160, 200, 110, "ext", "Your LLM server", "[External, optional]", ["Ollama or any", "OpenAI-compatible API"])
    s.box(50, 360, 260, 110, "container", "llama-server", "[Container: C++ · llama.cpp]", ["Built-in songwriter: Gemma 4", "OpenAI API on :18088"])
    s.box(470, 360, 260, 110, "container", "yue-server", "[Container: C++ · yue2.cpp, GGML]", ["YuE2, VAE and remake models", "HTTP job API on :18087"])
    s.box(50, 550, 260, 110, "store", "Library", "[Container: file system]", ["MP3 files and index.json", "with each song's recipe"])
    s.box(470, 550, 260, 110, "store", "Models", "[Container: file system]", ["GGUF weights and the", "llama.cpp runtime"])
    s.line([(170, 100), (170, 158)]); s.label(182, 134, "Uses", "start")
    s.line([(310, 200), (468, 200)]); s.label(390, 192, "commands")
    s.line([(470, 232), (312, 232)], dashed=True); s.label(390, 225, "events  [Tauri IPC]", muted=True)
    s.line([(470, 248), (390, 248), (390, 415), (312, 415)]); s.label(381, 332, "POST /v1/chat/completions", rotate=True)
    s.line([(470, 262), (430, 262), (430, 605), (312, 605)]); s.label(421, 520, "writes MP3 + index.json", rotate=True)
    s.line([(630, 270), (630, 358)]); s.label(642, 312, "POST /synth, GET /job", "start"); s.label(642, 330, "[HTTP/JSON]", "start", muted=True)
    s.line([(630, 470), (630, 548)]); s.label(642, 514, "loads weights", "start")
    s.line([(730, 262), (815, 262), (815, 605), (732, 605)]); s.label(806, 432, "saves downloads", rotate=True)
    s.line([(700, 160), (700, 55), (888, 55)]); s.label(795, 47, "downloads  [HTTPS]")
    s.line([(730, 215), (888, 215)]); s.label(809, 207, "chat  [HTTP]")
    return s.render()


def components():
    s = Svg("c4l3", 1110, 640, "C4 level 3, components of the Rust core: commands start jobs; jobs use the songwriter, engine client, library and audio reader; the LLM client, engine client and downloads talk to llama-server, yue-server and Hugging Face.")
    s.box(40, 10, 210, 80, "container", "Web UI", "[Container]")
    s.boundary(12, 115, 868, 510, "Rust core  [Container: Rust · Tauri 2]", 862, 138, "end")
    comp = lambda x, y, w, t, tech, line: s.box(x, y, w, 90, "component", t, tech, [line])
    comp(40, 150, 210, "Commands", "[lib.rs]", "Every call the UI can make")
    comp(340, 150, 220, "Jobs", "[jobs.rs]", "Pipeline, stages, queue, cancel")
    comp(650, 150, 210, "Engine client", "[engine.rs]", "Starts yue-server, submits, polls")
    comp(40, 310, 210, "Library", "[library.rs]", "Tracks on disk + index.json")
    comp(340, 310, 220, "Songwriter", "[songwriter.rs]", "Prompt → title, style, lyrics")
    comp(650, 310, 210, "LLM client", "[llm.rs]", "llama-server, Ollama, OpenAI")
    comp(40, 470, 210, "Cover art", "[cover.rs]", "Embeds images in MP3 (ID3)")
    comp(340, 470, 220, "Audio", "[audio.rs]", "Reads recordings for remakes")
    comp(650, 470, 210, "Downloads", "[downloads.rs]", "Resumable, SHA-256, unpack")
    s.box(930, 150, 170, 90, "container", "yue-server", "[Container]")
    s.box(930, 310, 170, 90, "container", "llama-server", "[Container]")
    s.box(930, 470, 170, 90, "ext", "Hugging Face", "[External system]")
    s.line([(145, 90), (145, 148)]); s.label(157, 107, "invoke / events", "start")
    s.line([(250, 195), (338, 195)]); s.label(295, 187, "start, cancel")
    s.line([(560, 195), (648, 195)]); s.label(605, 187, "submit, poll")
    s.line([(450, 240), (450, 308)]); s.label(462, 279, "write lyrics", "start")
    s.line([(560, 355), (648, 355)]); s.label(605, 347, "chat")
    s.line([(145, 240), (145, 308)]); s.label(157, 279, "list, rename, save", "start")
    s.line([(145, 400), (145, 468)]); s.label(157, 439, "embed cover", "start")
    s.line([(340, 214), (285, 214), (285, 355), (252, 355)]); s.label(277, 285, "save track", rotate=True)
    s.line([(340, 228), (305, 228), (305, 515), (338, 515)]); s.label(328, 445, "read recording", rotate=True)
    s.line([(40, 205), (26, 205), (26, 600), (755, 600), (755, 562)]); s.label(400, 592, "start and cancel downloads")
    s.line([(860, 195), (928, 195)]); s.label(905, 187, "HTTP")
    s.line([(860, 355), (928, 355)]); s.label(905, 347, "HTTP")
    s.line([(860, 515), (928, 515)]); s.label(905, 507, "HTTPS")
    return s.render()


def sequence():
    lanes = {"you": 75, "ui": 255, "core": 470, "llama": 690, "yue": 905, "lib": 1075}
    heads = [("you", "You", "person"), ("ui", "Web UI", "container"), ("core", "Rust core", "container"),
             ("llama", "llama-server", "container"), ("yue", "yue-server", "container"), ("lib", "Library", "store")]
    steps = [
        ("msg", "you", "ui", "Create: a description"),
        ("msg", "ui", "core", "invoke create_song(request)"),
        ("ret", "core", "ui", "Job { state: queued }"),
        ("self", "core", "lock the pipeline: one song at a time"),
        ("frame", "opt", "no lyrics given", "ui", "llama"),
        ("msg", "core", "ui", "event job: Writing lyrics"),
        ("msg", "core", "llama", "start llama-server, if needed"),
        ("msg", "core", "llama", "GET /health until ready"),
        ("msg", "core", "llama", "POST /v1/chat/completions"),
        ("ret", "llama", "core", "{ title, style, lyrics } JSON"),
        ("end",),
        ("frame", "opt", "free memory: on by default under 32 GB", "core", "llama"),
        ("msg", "core", "llama", "stop the process"),
        ("end",),
        ("msg", "core", "ui", "event job: Composing & singing"),
        ("msg", "core", "yue", "start yue-server, if needed"),
        ("msg", "core", "yue", "GET /health until ready"),
        ("msg", "core", "yue", "POST /synth { style, lyrics, cot, duration, seeds }"),
        ("ret", "yue", "core", "{ id }"),
        ("frame", "loop", "every 0.7 s until done", "ui", "yue"),
        ("msg", "core", "yue", "GET /job?id"),
        ("ret", "yue", "core", "{ status }"),
        ("msg", "core", "ui", "event engine-log: progress"),
        ("end",),
        ("msg", "core", "yue", "GET /job?id&result=1"),
        ("ret", "yue", "core", "multipart: replay JSON + MP3"),
        ("msg", "core", "lib", "write MP3, update index.json"),
        ("msg", "core", "ui", "event job: done"),
        ("msg", "ui", "core", "invoke track_audio(id)"),
        ("ret", "core", "ui", "MP3 bytes"),
        ("msg", "ui", "you", "plays the song"),
    ]
    y, rows, frames, open_frames = 108, [], [], []
    for step in steps:
        if step[0] == "frame":
            y += 12
            open_frames.append((step, y))
            y += 46
        elif step[0] == "end":
            frame, top = open_frames.pop()
            frames.append((frame, top, y - 6))
            y += 18
        else:
            rows.append((step, y))
            y += 46 if step[0] == "self" else 34
    height = y + 20
    s = Svg("seq", 1160, height, "Sequence diagram: what happens between the web UI, the Rust core, llama-server, yue-server and the library when you press Create, until the song plays.")
    for lane, title, kind in heads:
        x = lanes[lane]
        s.raw(f'<line class="d-life" x1="{x}" y1="64" x2="{x}" y2="{height - 10}"/>')
        s.box(x - 72, 10, 144, 54, kind, title)
    for (frame, kind, cond, a, b), top, bottom in frames:
        x1, x2 = lanes[a] - 60, lanes[b] + 60
        s.raw(f'<rect class="d-frame" x="{x1}" y="{top}" width="{x2 - x1}" height="{bottom - top}" rx="8"/>')
        s.raw(f'<path class="d-tag" d="M{x1} {top + 8}a8 8 0 0 1 8-8h{36 + 7 * len(kind)}v14l-8 8H{x1}z"/>')
        s.raw(f'<text class="d-tagtext" x="{x1 + 10}" y="{top + 16}">{esc(kind)}</text>')
        s.label(x1 + 54 + 7 * len(kind), top + 16, f"[{cond}]", "start", muted=True)
    for step, y in rows:
        if step[0] == "self":
            x = lanes[step[1]]
            s.line([(x, y), (x + 34, y), (x + 34, y + 16), (x + 4, y + 16)])
            s.label(x + 44, y + 12, step[2], "start")
            continue
        kind, a, b, text = step
        xa, xb = lanes[a], lanes[b]
        end = xb - 4 if xb > xa else xb + 4
        s.line([(xa, y), (end, y)], dashed=(kind == "ret"))
        s.label((xa + xb) / 2, y - 7, text, muted=(kind == "ret"))
    return s.render()


def main():
    page = PAGE.read_text()
    for name, draw in [("context", context), ("containers", containers), ("components", components), ("sequence", sequence)]:
        pattern = re.compile(rf"(<!-- diagram:{name} -->).*?(<!-- /diagram:{name} -->)", re.S)
        page, count = pattern.subn(lambda m: m.group(1) + draw() + m.group(2), page)
        if count != 1:
            raise SystemExit(f"marker for {name} not found once in {PAGE}")
    PAGE.write_text(page)
    print("diagrams written to", PAGE)


if __name__ == "__main__":
    main()
