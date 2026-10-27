# Cadenza — plan

**Cadenza** is a desktop app that turns a one-line idea into a full song with vocals,
entirely on your own machine. Type "a Hindi lullaby for my 3-year-old son Babu who loves
painting", press Create, and a few minutes later a finished track lands in your library.

> *Cadenza* — the moment in a concerto where the soloist improvises freely. Here, the
> soloist is the model.

## Goals

1. **Prompt to full song** — a description is enough. Lyrics, title and style are written
   by a local language model; YuE2 composes the score and sings it.
2. **Runs locally** — no cloud call in the generation path. Models are downloaded once from
   Hugging Face and then everything works offline.
3. **Every model size** — small machines pick the light quantisations, big machines pick
   the full-precision ones. Nothing is hard-coded to one size.
4. **A real desktop app** — native window, Rust backend, no Python and no Node.js at runtime.

## What the UI looks like (modelled on StepAudio 3 Music Studio)

```
┌──────────┬──────────────────────────────┬───────────────────────────────┐
│ Cadenza  │  Song | Song Cover           │  THIS SESSION                 │
│          │  Description (required)      │  ┌──────────────────────────┐ │
│ Home     │  [ ...................... ]  │  │ ▣ Babu's Painting Song ▶ │ │
│ Create   │  Lyrics (optional) ☐ Instr.  │  └──────────────────────────┘ │
│ Library  │  [ [Verse] ... ]  ✦ Write    │  generating… composing score  │
│ Models   │  Song title (optional)       │                               │
│ Settings │  ▸ Advanced                  │                               │
│          │  [        Create →        ]  │                               │
├──────────┴──────────────────────────────┴───────────────────────────────┤
│ ◉ Babu's Painting Song   ⏮ ▶ ⏭  0:42 ▁▃▅▇▅▃▁▃▅▇█▇▅▃ 2:55  ♡ ⤓ ⟳ 🔊       │
└─────────────────────────────────────────────────────────────────────────┘
```

- **Sidebar**: Home, Create, Library, Models, Settings, engine status light.
- **Create → Song**: description (500 chars), lyrics (optional, "✦ Write lyrics" fills them
  from the description), Instrumental toggle, title (optional, auto-named), Advanced
  (duration, language, score mode, steps, seeds, output format, variations).
- **Create → Song Cover**: pick an audio file; SheetSage2 transcribes its melody to a score,
  YuE2 sings that melody in a new style with the lyrics you give.
- **This session**: live job cards with stage and elapsed time, then the finished tracks.
- **Library**: every track ever made, search, favourites, play, save as, re-render variation,
  delete, view lyrics / style / score.
- **Player bar**: cover art, prev / play / next, seek on a real waveform, loop, volume,
  favourite, download.
- **Models**: download manager with every size, progress, resume, SHA-256 check, delete,
  recommended badge based on this machine's RAM.
- **Settings**: language model provider (built-in llama.cpp / Ollama / any OpenAI-compatible
  server), engine options (keep loaded, context size), output format, folders.

## Architecture

```
┌────────────────────────── Cadenza.app (Tauri 2) ──────────────────────────┐
│  ui/ (HTML + CSS + JS, no build step)                                      │
│        │ invoke() / events                                                 │
│  src-tauri/ (Rust)                                                         │
│   ├─ catalog.rs    every downloadable file, sizes, hashes, recommendations │
│   ├─ downloads.rs  resumable HF downloads, SHA-256, tar.gz unpack          │
│   ├─ engine.rs     yue-server supervisor + HTTP client + multipart parser  │
│   ├─ llm.rs        llama-server sidecar / Ollama / OpenAI-compatible chat  │
│   ├─ songwriter.rs prompt → {title, style, lyrics} JSON                    │
│   ├─ jobs.rs       generation pipeline, stages, cancellation, events       │
│   ├─ library.rs    tracks on disk + index.json                             │
│   └─ settings.rs   persisted preferences                                   │
└───────┬───────────────────────────────────────────┬────────────────────────┘
        │ HTTP 127.0.0.1:18087                      │ HTTP 127.0.0.1:18088
  yue-server (yue2.cpp, Metal/CUDA/Vulkan)    llama-server (llama.cpp) or Ollama
  YuE2 backbone + VAE + companion LoRA        Gemma 4 (E4B … 12B Q8) or any model
```

### Generation pipeline

1. **Write** — if lyrics are empty (and not instrumental), the language model gets the
   description and returns JSON `{title, style, lyrics}`. Style is a descriptive caption
   (genre, instruments, voice, mood, tempo) — the format YuE2 was trained on. Lyrics carry
   `[Verse]`, `[Chorus]`, `[Bridge]`, `[Outro]` tags. When lyrics are given, the model only
   writes title and style. Instrumental songs get section tags without words.
2. **Free memory** — on machines under 32 GB the llama-server sidecar is stopped before
   music starts, so both models never sit in unified memory together.
3. **Compose & sing** — `POST /synth` to yue-server (`cot: full` → the model writes an ABC
   score first, then the semantic codes, then flow-matching paints the audio, VAE decodes).
   Poll `GET /job`, stream engine log lines to the UI as the stage detail.
4. **Save** — `GET /job?result=1` returns multipart/mixed: replay JSON + audio + latents per
   track. Audio and replay JSON (score, seeds, semantic codes) go to the library.
5. **Re-render** — a variation reuses the stored `semantic_tokens` with a new noise seed:
   same composition and singing, new sound, in a fraction of the time.

## Models

### Music engine (YuE2, CC BY-NC 4.0 — non-commercial)

From `Serveurperso/YuE2-GGUF` at a pinned revision, SHA-256 verified.

| Set | Backbone | + VAE + companion | Machine |
|---|---|---|---|
| Light | Q5_K_M 2.62 GB | 3.29 GB | 8–16 GB RAM |
| Balanced | Q6_K 2.94 GB | 3.61 GB | 16 GB RAM (recommended on M1 Pro 16 GB) |
| Quality | Q8_0 3.81 GB | 4.48 GB | 24 GB+ |
| Full | BF16 7.17 GB | 7.84 GB | 32 GB+ |

Optional for covers: SheetSage2 transcriber (Q8 106 MB / F32 229 MB) + MERT-v2 encoder
(Q8 902 MB / F32 2.53 GB).

### Language model (writes lyrics, title, style)

| Option | Size | Machine |
|---|---|---|
| Gemma 4 E4B QAT q4_0 | 5.2 GB | 16 GB (default) |
| Gemma 4 12B QAT q4_0 | 7.0 GB | 24 GB+ |
| Gemma 4 12B Q5_K_M | 8.4 GB | 32 GB+ |
| Gemma 4 12B Q6_K | 9.8 GB | 32 GB+ |
| Gemma 4 12B Q8_0 | 12.7 GB | 48 GB+ |
| Ollama | any model you pulled (`llama3.3:70b`, `qwen3:32b`, …) | yours |
| OpenAI-compatible | LM Studio, vLLM, a remote box, … | yours |

Runtime: llama.cpp `llama-server` (pinned release b11900, macOS arm64), downloaded in-app.

## Engine binary

`yue-server` is built from yue2.cpp (pinned commit `ec43393`) with
`scripts/build-engine.sh`: Metal on macOS, CUDA when `nvcc` is found on Linux.
The binary and its ggml dylibs go to `src-tauri/resources/yue2-cpp/` and are bundled into
the app. A custom `yue-server` path can be set in Settings.

## Milestones

- [x] M1 — Research: StepAudio-3 UI, YuE2 Studio / Ai Music Studio (yue2.cpp + Tauri),
      Glaux, Dynamic Island (Tauri patterns).
- [x] M2 — Plan, name, icon.
- [x] M3 — Rust backend: settings, catalog, downloads, engine supervisor, LLM, jobs, library.
- [x] M4 — UI: Create (Song / Cover), session, Library, Models, Settings, player.
- [x] M5 — Build scripts, README, app icons, `cargo check`/tests.
- [ ] M6 — Later: stems (HT-Demucs), karaoke LRC, MIDI export, LoRA picker, Windows/Linux
      installers, "Vocal to Song" (needs a model that conditions on a vocal take).

## Why YuE2 and not StepAudio 3 Music

StepAudio 3 Music (`stepaudio-3-music-preview`) is the model behind the Hugging Face Space
that inspired this UI, but it is **API-only**: StepFun publishes no weights, and the preview
is free for a limited time only, with a paid version to follow. It can't run locally or
offline, and it can't generate without limits. YuE2 has open weights, a native Metal port,
and a score you can edit, so it is the local engine. A StepAudio cloud backend could be
added later as an optional provider.

## Out of scope / known limits

- **Vocal to Song** (StepAudio tab) needs a model conditioned on a sung recording; YuE2 has
  no such input, so it is not offered. Song Cover is the closest equivalent.
- YuE2 weights are **non-commercial** (CC BY-NC 4.0). Shown in the Models page.
- Generation speed on an M1 Pro is roughly real-time to 3× slower than real-time for the
  song length; a 3-minute song takes several minutes.
