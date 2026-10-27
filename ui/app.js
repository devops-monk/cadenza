// Cadenza UI. No framework, no build step: talks to the Rust side through
// window.__TAURI__ (withGlobalTauri).

const T = window.__TAURI__;
const invoke = (cmd, args) => T.core.invoke(cmd, args);
const listen = (event, fn) => T.event.listen(event, (e) => fn(e.payload));
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

const dialog = {
  save: (options) => (T.dialog ? T.dialog.save(options) : invoke("plugin:dialog|save", { options })),
  open: (options) => (T.dialog ? T.dialog.open(options) : invoke("plugin:dialog|open", { options })),
};
const reveal = (path) => (T.opener ? T.opener.revealItemInDir(path) : invoke("plugin:opener|reveal_item_in_dir", { paths: [path] }));

const state = {
  view: "create",
  tab: "song",
  coverPath: null,
  settings: null,
  system: null,
  catalog: null,
  engine: null,
  jobs: [],
  tracks: [],
  sessionStart: Date.now(),
  filter: "all",
  search: "",
  queue: [],
  current: null,
  drawerTrack: null,
  drawerTab: "lyrics",
};

// ------------------------------------------------------------------ helpers

function esc(text) {
  return String(text ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

function fmtTime(seconds) {
  if (!isFinite(seconds) || seconds < 0) seconds = 0;
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

function fmtBytes(bytes) {
  if (!bytes) return "—";
  const gb = bytes / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(2)} GB` : `${(bytes / 1024 ** 2).toFixed(0)} MB`;
}

function toast(message, bad = false) {
  const el = document.createElement("div");
  el.className = "toast" + (bad ? " bad" : "");
  el.textContent = message;
  $("#toasts").append(el);
  setTimeout(() => el.remove(), bad ? 7000 : 3500);
}

function hash(text) {
  let h = 2166136261;
  for (const ch of String(text)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619);
  return h >>> 0;
}

/** Deterministic cover art: two-hue gradient with soft rings and a wave. */
function artSvg(seed) {
  const h = hash(seed);
  const a = h % 360;
  const b = (a + 40 + ((h >> 9) % 120)) % 360;
  const cx = 20 + ((h >> 3) % 60);
  const cy = 20 + ((h >> 7) % 60);
  const amp = 6 + ((h >> 11) % 10);
  const id = "g" + h.toString(36);
  let wave = "M0 70";
  for (let x = 0; x <= 100; x += 10) wave += ` Q${x + 5} ${70 + (x % 20 ? amp : -amp)} ${x + 10} 70`;
  return `<svg viewBox="0 0 100 100" preserveAspectRatio="xMidYMid slice"><defs>
    <linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${a} 85% 62%)"/><stop offset="1" stop-color="hsl(${b} 70% 32%)"/></linearGradient>
    <radialGradient id="${id}r" cx="${cx}%" cy="${cy}%" r="60%"><stop offset="0" stop-color="#fff" stop-opacity=".55"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient></defs>
    <rect width="100" height="100" fill="url(#${id})"/><rect width="100" height="100" fill="url(#${id}r)"/>
    <circle cx="${cx}" cy="${cy}" r="${18 + (h % 14)}" fill="none" stroke="#fff" stroke-opacity=".25" stroke-width="1.5"/>
    <circle cx="${cx}" cy="${cy}" r="${30 + (h % 18)}" fill="none" stroke="#fff" stroke-opacity=".14" stroke-width="1"/>
    <path d="${wave}" fill="none" stroke="#fff" stroke-opacity=".5" stroke-width="2"/></svg>`;
}

function kindLabel(kind) {
  return { song: "Song", cover: "Cover", variation: "Variation" }[kind] || "Song";
}

// --------------------------------------------------------------- navigation

function show(view) {
  state.view = view;
  $$(".nav button").forEach((b) => b.classList.toggle("active", b.dataset.view === view));
  $$(".view").forEach((v) => v.classList.toggle("active", v.id === `view-${view}`));
  if (view === "models") refreshCatalog();
  if (view === "settings") fillSettings();
  if (view === "library") renderLibrary();
  if (view === "home") renderHome();
}

$$(".nav button").forEach((b) => b.addEventListener("click", () => show(b.dataset.view)));
$$("[data-goto]").forEach((b) => b.addEventListener("click", () => show(b.dataset.goto)));

// ------------------------------------------------------------------- theme

function applyTheme(choice) {
  if (choice === "auto") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", choice);
  $$("[data-theme-choice]").forEach((b) => b.classList.toggle("active", b.dataset.themeChoice === choice));
  try { localStorage.setItem("cadenza-theme", choice); } catch {}
  drawWave();
}
$$("[data-theme-choice]").forEach((b) => b.addEventListener("click", () => applyTheme(b.dataset.themeChoice)));
let savedTheme = "auto";
try { savedTheme = localStorage.getItem("cadenza-theme") || "auto"; } catch {}

// ------------------------------------------------------------------ create

const IDEAS = [
  "compose a hindi song for my 3 years old son. his name is babu singh and he loves music and he goes to nursery and likes painting",
  "an upbeat 80s synth-pop anthem about driving through a neon city at midnight, female vocals",
  "a warm acoustic folk ballad about my grandmother's kitchen on Sunday mornings",
  "lo-fi hip hop study beat with soft piano and vinyl crackle, instrumental",
  "a Bollywood style romantic duet about meeting again in the monsoon rain",
  "epic orchestral rock song for a marathon finish line, male vocals, huge choir",
  "a cheerful birthday song for my best friend Maya who loves cats and coffee",
  "a Spanish reggaeton summer track about dancing on the beach until sunrise",
  "a sad indie piano song about moving away from your hometown",
  "a Punjabi bhangra party song with dhol, for a wedding sangeet",
];

const form = {
  description: $("#description"),
  lyrics: $("#lyrics"),
  title: $("#title"),
  style: $("#style"),
  instrumental: $("#instrumental"),
  language: $("#language"),
  duration: $("#duration"),
  cot: $("#cot"),
  variations: $("#variations"),
  steps: $("#steps"),
  temperature: $("#temperature"),
  lmSeed: $("#lmSeed"),
  seed: $("#seed"),
};

function updateCounters() {
  $("#descCount").textContent = `${form.description.value.length}/500`;
  $("#durationValue").textContent = fmtTime(+form.duration.value);
  $("#stepsValue").textContent = form.steps.value;
  $("#temperatureValue").textContent = (+form.temperature.value).toFixed(2);
  form.lyrics.disabled = form.instrumental.checked;
  $("#writeLyrics").disabled = form.instrumental.checked;
}
["input", "change"].forEach((ev) => Object.values(form).forEach((el) => el.addEventListener(ev, updateCounters)));

$("#shuffleIdea").addEventListener("click", () => {
  const options = IDEAS.filter((idea) => idea !== form.description.value);
  form.description.value = options[Math.floor(Math.random() * options.length)];
  updateCounters();
});

$$(".tabs .tab[data-tab]").forEach((tab) =>
  tab.addEventListener("click", () => {
    state.tab = tab.dataset.tab;
    $$(".tabs .tab[data-tab]").forEach((t) => t.classList.toggle("active", t === tab));
    $(".create").dataset.tab = state.tab;
    const cover = state.tab === "cover";
    $("#tabHint").textContent = cover
      ? "Pick a recording. Its melody is sung again in the style you describe."
      : "Write a prompt, and lyrics if you like. Get a song of your own.";
    $("#descLabel").textContent = cover ? "Style of the cover" : "Description";
    form.description.placeholder = cover ? "jazz funk with a female singer and a horn section" : IDEAS[0];
    $("#descReq").textContent = cover ? "Optional" : "Required";
  })
);

$("#coverPick").addEventListener("click", async () => {
  const path = await dialog.open({ multiple: false, directory: false, filters: [{ name: "Audio", extensions: ["mp3", "wav"] }] });
  if (!path) return;
  state.coverPath = typeof path === "string" ? path : path.path;
  $("#coverName").textContent = state.coverPath.split(/[\\/]/).pop();
  $("#coverPick").classList.add("has");
});

function readRequest() {
  const number = (el) => (el.value.trim() === "" ? null : Number(el.value));
  return {
    description: form.description.value.trim(),
    lyrics: form.instrumental.checked ? "" : form.lyrics.value,
    title: form.title.value.trim(),
    style: form.style.value.trim(),
    instrumental: form.instrumental.checked,
    language: form.language.value,
    duration: Number(form.duration.value),
    cot: form.cot.value,
    steps: Number(form.steps.value),
    variations: Number(form.variations.value),
    temperature: Number(form.temperature.value) === 1 ? null : Number(form.temperature.value),
    lm_seed: number(form.lmSeed),
    seed: number(form.seed),
    cover_audio: state.tab === "cover" ? state.coverPath : null,
    melody_only: $("#melodyOnly").checked,
  };
}

function formError(message) {
  const el = $("#formError");
  el.hidden = !message;
  el.textContent = message || "";
}

async function createSong(request) {
  formError("");
  if (state.tab === "cover" && !request.cover_audio) return formError("Choose a recording to cover.");
  if (state.tab === "cover" && !request.lyrics.trim() && !request.instrumental) return formError("A cover needs lyrics that fit the melody — the original lyrics work best.");
  if (!request.description && !request.lyrics.trim()) return formError("Describe the song you want.");
  try {
    const job = await invoke("create_song", { request });
    upsertJob(job);
    show("create");
  } catch (error) {
    formError(String(error));
  }
}

$("#createBtn").addEventListener("click", () => createSong(readRequest()));
form.description.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) createSong(readRequest());
});

$("#writeLyrics").addEventListener("click", async () => {
  const button = $("#writeLyrics");
  const request = readRequest();
  if (!request.description) return formError("Describe the song first, then let the model write the lyrics.");
  formError("");
  button.disabled = true;
  const label = button.innerHTML;
  button.innerHTML = `<span class="spark">✦</span> Writing…`;
  try {
    const written = await invoke("write_lyrics", { request });
    form.lyrics.value = written.lyrics;
    if (!form.title.value.trim()) form.title.value = written.title;
    if (!form.style.value.trim()) form.style.value = written.style;
    $("#advanced").open = true;
    toast("Lyrics written — edit anything you like, then Create.");
  } catch (error) {
    formError(String(error));
  } finally {
    button.innerHTML = label;
    updateCounters();
  }
});

// -------------------------------------------------------------------- jobs

function upsertJob(job) {
  const index = state.jobs.findIndex((j) => j.id === job.id);
  if (index >= 0) state.jobs[index] = job;
  else state.jobs.unshift(job);
  renderSession();
}

function jobRow(job) {
  const active = job.state === "running" || job.state === "queued";
  const elapsed = active && job.state === "running" ? ` · ${fmtTime((Date.now() - job.started_at) / 1000)}` : "";
  const sub = job.state === "error" ? "Failed" : job.state === "cancelled" ? "Cancelled" : `${job.stage}${job.detail ? " — " + job.detail : ""}${elapsed}`;
  return `<div class="track job ${job.state}" data-job="${job.id}">
    <div class="job-art"><div class="bars"><i></i><i></i><i></i><i></i><i></i></div></div>
    <div class="track-text"><strong>${esc(job.title)}</strong><span>${esc(sub)}</span>
      ${active ? '<div class="progress-line"></div>' : ""}
      ${job.error ? `<div class="job-error">${esc(job.error)}</div>` : ""}</div>
    <div class="track-actions">
      ${active ? `<button class="btn ghost small" data-cancel="${job.id}">Cancel</button>` : `<button class="icon-btn" data-dismiss="${job.id}" title="Dismiss"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18"/></svg></button>`}
    </div></div>`;
}

function trackRow(track) {
  const playing = state.current && state.current.id === track.id;
  const meta = [kindLabel(track.kind), track.duration ? fmtTime(track.duration) : null, new Date(track.created_at).toLocaleDateString()].filter(Boolean).join(" · ");
  return `<div class="track ${playing ? "playing" : ""}" data-track="${track.id}">
    <div class="art">${artSvg(track.id)}</div>
    <div class="track-text" data-open="${track.id}"><strong>${esc(track.title)}</strong><span>${esc(meta)}</span></div>
    <div class="track-actions">
      <button class="icon-btn ${track.favorite ? "on" : ""}" data-fav="${track.id}" title="Favourite"><svg viewBox="0 0 24 24"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 4.5 6.6 4.5c2.1 0 3.6 1.2 4.4 2.6.8-1.4 2.3-2.6 4.4-2.6 3.6 0 5.7 3.9 4.2 7.3C19.5 16.4 12 21 12 21z"/></svg></button>
      <button class="icon-btn" data-save="${track.id}" title="Save as…"><svg viewBox="0 0 24 24"><path d="M12 4v11M7 10l5 5 5-5M5 20h14"/></svg></button>
      <button class="icon-btn" data-delete="${track.id}" title="Delete"><svg viewBox="0 0 24 24"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13"/></svg></button>
      <button class="play" data-play="${track.id}" title="Play"><svg viewBox="0 0 24 24" class="fill"><path d="${playing && !audio.paused ? "M7 5h4v14H7zM13 5h4v14h-4z" : "M8 5v14l11-7z"}"/></svg></button>
    </div></div>`;
}

function sessionTracks() {
  return state.tracks.filter((t) => new Date(t.created_at).getTime() >= state.sessionStart - 1000);
}

function renderSession() {
  const jobs = state.jobs.filter((j) => j.state !== "done");
  const tracks = sessionTracks();
  $("#sessionList").innerHTML = jobs.map(jobRow).join("") + tracks.map(trackRow).join("");
  $("#sessionEmpty").hidden = jobs.length + tracks.length > 0;
}

// Elapsed time on running jobs.
setInterval(() => {
  if (state.jobs.some((j) => j.state === "running")) renderSession();
}, 1000);

// ----------------------------------------------------------------- library

async function loadTracks() {
  state.tracks = await invoke("list_tracks");
  renderSession();
  if (state.view === "library") renderLibrary();
  if (state.view === "home") renderHome();
}

function filteredTracks() {
  const q = state.search.toLowerCase();
  return state.tracks.filter((t) => {
    if (state.filter === "fav" && !t.favorite) return false;
    if (state.filter === "song" && t.kind === "cover") return false;
    if (state.filter === "cover" && t.kind !== "cover") return false;
    if (!q) return true;
    return [t.title, t.lyrics, t.style, t.description].some((field) => (field || "").toLowerCase().includes(q));
  });
}

function renderLibrary() {
  const tracks = filteredTracks();
  $("#libraryList").innerHTML = tracks.map(trackRow).join("");
  $("#libraryEmpty").hidden = tracks.length > 0;
}

$("#librarySearch").addEventListener("input", (e) => {
  state.search = e.target.value;
  renderLibrary();
});
$$("#libraryFilter button").forEach((b) =>
  b.addEventListener("click", () => {
    state.filter = b.dataset.filter;
    $$("#libraryFilter button").forEach((x) => x.classList.toggle("active", x === b));
    renderLibrary();
  })
);
$("#openLibraryDir").addEventListener("click", async () => {
  try { await reveal(await invoke("library_dir")); } catch (e) { toast(String(e), true); }
});

async function saveTrack(id) {
  const name = await invoke("track_file_name", { id });
  const ext = name.split(".").pop();
  const destination = await dialog.save({ defaultPath: name, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!destination) return;
  try {
    await invoke("export_track", { id, destination });
    toast("Saved");
  } catch (e) {
    toast(String(e), true);
  }
}

async function toggleFavorite(id) {
  const track = state.tracks.find((t) => t.id === id);
  if (!track) return;
  const updated = await invoke("update_track", { id, favorite: !track.favorite });
  Object.assign(track, updated);
  refreshTrackViews();
}

async function deleteTrack(id) {
  const track = state.tracks.find((t) => t.id === id);
  if (!track) return;
  const ok = T.dialog ? await T.dialog.ask(`Delete “${track.title}”? This removes the audio file.`, { title: "Delete track", kind: "warning" }) : confirm(`Delete “${track.title}”?`);
  if (!ok) return;
  if (state.current && state.current.id === id) stopPlayback();
  await invoke("delete_track", { id });
  state.tracks = state.tracks.filter((t) => t.id !== id);
  closeDrawer();
  refreshTrackViews();
}

function refreshTrackViews() {
  renderSession();
  if (state.view === "library") renderLibrary();
  if (state.view === "home") renderHome();
  updatePlayerMeta();
}

// One delegated handler for every track list.
document.addEventListener("click", (e) => {
  const target = e.target.closest("[data-play],[data-fav],[data-save],[data-delete],[data-open],[data-cancel],[data-dismiss]");
  if (!target) return;
  const d = target.dataset;
  const list = target.closest(".track-list");
  if (d.play) {
    if (state.current && state.current.id === d.play) togglePlay();
    else playTrack(d.play, queueFor(list));
  } else if (d.fav) toggleFavorite(d.fav);
  else if (d.save) saveTrack(d.save);
  else if (d.delete) deleteTrack(d.delete);
  else if (d.open) openDrawer(d.open);
  else if (d.cancel) invoke("cancel_job", { id: d.cancel });
  else if (d.dismiss) {
    invoke("dismiss_job", { id: d.dismiss });
    state.jobs = state.jobs.filter((j) => j.id !== d.dismiss);
    renderSession();
  }
});

function queueFor(list) {
  if (!list) return state.tracks.map((t) => t.id);
  return $$("[data-track]", list).map((el) => el.dataset.track);
}

// ------------------------------------------------------------------ drawer

function openDrawer(id) {
  const track = state.tracks.find((t) => t.id === id);
  if (!track) return;
  state.drawerTrack = track;
  $("#drawerArt").innerHTML = artSvg(track.id);
  $("#drawerTitle").value = track.title;
  const meta = [kindLabel(track.kind), track.duration ? fmtTime(track.duration) : null, new Date(track.created_at).toLocaleString(), track.backbone?.replace("yue-", "YuE2 ").toUpperCase()].filter(Boolean);
  $("#drawerMeta").textContent = meta.join(" · ");
  $("#drawer").hidden = false;
  renderDrawerTab();
}

function renderDrawerTab() {
  const t = state.drawerTrack;
  if (!t) return;
  $$("[data-dtab]").forEach((b) => b.classList.toggle("active", b.dataset.dtab === state.drawerTab));
  const body = $("#drawerBody");
  const seeds = `\n\nComposition seed: ${t.lm_seed ?? "—"}\nSound seed: ${t.seed ?? "—"}`;
  const content = {
    lyrics: t.instrumental ? "Instrumental" : t.lyrics,
    style: t.style,
    score: t.abc || "No score: this song was made with the score turned off.",
    prompt: (t.description || "—") + seeds,
  }[state.drawerTab];
  body.classList.toggle("mono", state.drawerTab === "score");
  body.textContent = content || "—";
}

function closeDrawer() {
  $("#drawer").hidden = true;
  state.drawerTrack = null;
}

$$("[data-dtab]").forEach((b) => b.addEventListener("click", () => { state.drawerTab = b.dataset.dtab; renderDrawerTab(); }));
$$("#drawer [data-close]").forEach((el) => el.addEventListener("click", closeDrawer));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("#drawer").hidden) closeDrawer();
  if (e.code === "Space" && !/INPUT|TEXTAREA|SELECT/.test(document.activeElement.tagName) && state.current) {
    e.preventDefault();
    togglePlay();
  }
});
$("#drawerTitle").addEventListener("change", async (e) => {
  const t = state.drawerTrack;
  if (!t) return;
  Object.assign(t, await invoke("update_track", { id: t.id, title: e.target.value }));
  refreshTrackViews();
});
$("#drawerPlay").addEventListener("click", () => state.drawerTrack && playTrack(state.drawerTrack.id, state.tracks.map((t) => t.id)));
$("#drawerSave").addEventListener("click", () => state.drawerTrack && saveTrack(state.drawerTrack.id));
$("#drawerDelete").addEventListener("click", () => state.drawerTrack && deleteTrack(state.drawerTrack.id));
$("#drawerVariation").addEventListener("click", async () => {
  const t = state.drawerTrack;
  if (!t) return;
  try {
    const job = await invoke("create_song", { request: { description: t.description, parent_id: t.id, title: t.title, steps: Number(form.steps.value) } });
    upsertJob(job);
    closeDrawer();
    show("create");
    toast("Re-rendering with a new sound seed");
  } catch (e) {
    toast(String(e), true);
  }
});
$("#drawerReuse").addEventListener("click", () => {
  const t = state.drawerTrack;
  if (!t) return;
  form.description.value = t.description || "";
  form.lyrics.value = t.instrumental ? "" : t.lyrics;
  form.style.value = t.style;
  form.instrumental.checked = t.instrumental;
  form.title.value = "";
  $("#advanced").open = true;
  updateCounters();
  closeDrawer();
  show("create");
});

// ------------------------------------------------------------------ player

const audio = $("#audio");
const wave = $("#wave");
const player = { url: null, peaks: [], loop: false, muted: false };
audio.volume = 0.9;

async function playTrack(id, queue) {
  const track = state.tracks.find((t) => t.id === id);
  if (!track) return;
  state.queue = queue && queue.length ? queue : state.tracks.map((t) => t.id);
  state.current = track;
  updatePlayerMeta();
  try {
    const buffer = await invoke("track_audio", { id });
    if (player.url) URL.revokeObjectURL(player.url);
    const type = track.format === "wav" ? "audio/wav" : "audio/mpeg";
    const blob = new Blob([buffer], { type });
    player.url = URL.createObjectURL(blob);
    audio.src = player.url;
    await audio.play();
    computePeaks(buffer.slice ? buffer.slice(0) : buffer);
  } catch (e) {
    toast(`Could not play: ${e}`, true);
  }
}

function stopPlayback() {
  audio.pause();
  audio.removeAttribute("src");
  state.current = null;
  player.peaks = [];
  updatePlayerMeta();
  drawWave();
}

function togglePlay() {
  if (!state.current) {
    const first = sessionTracks()[0] || state.tracks[0];
    if (first) playTrack(first.id);
    return;
  }
  if (audio.paused) audio.play();
  else audio.pause();
}

function step(delta) {
  if (!state.current) return;
  const index = state.queue.indexOf(state.current.id);
  const next = state.queue[index + delta];
  if (next) playTrack(next, state.queue);
  else if (delta < 0) audio.currentTime = 0;
}

async function computePeaks(arrayBuffer) {
  player.peaks = [];
  drawWave();
  try {
    const context = new (window.AudioContext || window.webkitAudioContext)();
    const decoded = await context.decodeAudioData(arrayBuffer);
    const data = decoded.getChannelData(0);
    const count = 220;
    const size = Math.floor(data.length / count);
    const peaks = [];
    for (let i = 0; i < count; i++) {
      let max = 0;
      for (let j = i * size; j < (i + 1) * size; j += 16) max = Math.max(max, Math.abs(data[j]));
      peaks.push(max);
    }
    const top = Math.max(...peaks, 0.01);
    player.peaks = peaks.map((p) => p / top);
    context.close();
  } catch {
    player.peaks = [];
  }
  drawWave();
}

function drawWave() {
  if (!wave) return;
  const ratio = window.devicePixelRatio || 1;
  const width = wave.clientWidth;
  const height = wave.clientHeight;
  if (!width) return;
  wave.width = width * ratio;
  wave.height = height * ratio;
  const ctx = wave.getContext("2d");
  ctx.scale(ratio, ratio);
  ctx.clearRect(0, 0, width, height);
  const styles = getComputedStyle(document.documentElement);
  const base = styles.getPropertyValue("--line").trim() || "#ddd";
  const played = ctx.createLinearGradient(0, 0, width, 0);
  played.addColorStop(0, "#ffb048");
  played.addColorStop(0.45, "#ff4d8d");
  played.addColorStop(1, "#7c5cff");
  const bars = Math.max(20, Math.floor(width / 4));
  const progress = audio.duration ? audio.currentTime / audio.duration : 0;
  for (let i = 0; i < bars; i++) {
    const p = player.peaks.length ? player.peaks[Math.floor((i / bars) * player.peaks.length)] : 0.12 + 0.08 * Math.sin(i * 0.7);
    const h = Math.max(2, p * (height - 4));
    const x = i * 4;
    ctx.fillStyle = i / bars < progress ? played : base;
    ctx.fillRect(x, (height - h) / 2, 2, h);
  }
}

wave.addEventListener("click", (e) => {
  if (!audio.duration) return;
  const rect = wave.getBoundingClientRect();
  audio.currentTime = ((e.clientX - rect.left) / rect.width) * audio.duration;
});
window.addEventListener("resize", drawWave);

function updatePlayerMeta() {
  const t = state.current;
  $("#playerTitle").textContent = t ? t.title : "Nothing playing";
  $("#playerKind").textContent = t ? kindLabel(t.kind) : "—";
  $("#playerArt").innerHTML = t ? artSvg(t.id) : "";
  $("#favBtn").classList.toggle("on", !!(t && t.favorite));
  $("#playIcon").setAttribute("d", audio.paused ? "M8 5v14l11-7z" : "M7 5h4v14H7zM13 5h4v14h-4z");
  $("#player").classList.toggle("playing", !audio.paused);
  $$(".track[data-track]").forEach((row) => {
    const playing = t && row.dataset.track === t.id;
    row.classList.toggle("playing", !!playing);
    const icon = $(".play path", row);
    if (icon) icon.setAttribute("d", playing && !audio.paused ? "M7 5h4v14H7zM13 5h4v14h-4z" : "M8 5v14l11-7z");
  });
}

audio.addEventListener("timeupdate", () => {
  $("#curTime").textContent = fmtTime(audio.currentTime);
  drawWave();
});
audio.addEventListener("loadedmetadata", () => ($("#durTime").textContent = fmtTime(audio.duration)));
audio.addEventListener("play", updatePlayerMeta);
audio.addEventListener("pause", updatePlayerMeta);
audio.addEventListener("ended", () => {
  if (player.loop) {
    audio.currentTime = 0;
    audio.play();
  } else step(1);
});

$("#playBtn").addEventListener("click", togglePlay);
$("#prevBtn").addEventListener("click", () => step(-1));
$("#nextBtn").addEventListener("click", () => step(1));
$("#favBtn").addEventListener("click", () => state.current && toggleFavorite(state.current.id));
$("#dlBtn").addEventListener("click", () => state.current && saveTrack(state.current.id));
$("#lyricsBtn").addEventListener("click", () => state.current && openDrawer(state.current.id));
$("#loopBtn").addEventListener("click", () => {
  player.loop = !player.loop;
  $("#loopBtn").classList.toggle("on", player.loop);
});
$("#volume").addEventListener("input", (e) => {
  audio.volume = Number(e.target.value);
  audio.muted = false;
  $("#volWaves").style.opacity = 1;
});
$("#muteBtn").addEventListener("click", () => {
  audio.muted = !audio.muted;
  $("#volWaves").style.opacity = audio.muted ? 0.15 : 1;
});

// -------------------------------------------------------------------- home

function renderHome() {
  const chips = IDEAS.slice(1, 7).map((idea) => `<button class="chip" data-idea="${esc(idea)}">${esc(idea.length > 46 ? idea.slice(0, 44) + "…" : idea)}</button>`);
  $("#heroChips").innerHTML = chips.join("");
  $("#homeRecent").innerHTML = state.tracks.slice(0, 5).map(trackRow).join("") || `<p class="muted small">Nothing yet.</p>`;
  renderChecklist();
}

$("#heroChips").addEventListener("click", (e) => {
  const chip = e.target.closest("[data-idea]");
  if (chip) $("#heroPrompt").value = chip.dataset.idea;
});
$("#heroGo").addEventListener("click", () => {
  const text = $("#heroPrompt").value.trim();
  if (!text) return $("#heroPrompt").focus();
  form.description.value = text;
  updateCounters();
  show("create");
  createSong(readRequest());
});

function renderChecklist() {
  const c = state.catalog;
  const s = state.settings;
  const e = state.engine;
  if (!c || !s || !e) return;
  const installed = (id) => c.entries.some((x) => x.id === id && x.installed);
  const llmReady = s.llm_provider !== "builtin" || (installed(s.llm_model) && installed("llama-runtime"));
  const items = [
    { done: !!e.executable, text: "Music engine (yue-server) is built", hint: e.executable ? "" : "Run scripts/build-engine.sh", goto: "settings" },
    { done: installed(s.backbone) && installed("yue-vae"), text: "YuE2 music model downloaded", goto: "models" },
    { done: llmReady, text: s.llm_provider === "builtin" ? "Language model downloaded (writes lyrics)" : `Language model: ${s.llm_provider === "ollama" ? "Ollama" : "OpenAI-compatible"}`, goto: s.llm_provider === "builtin" ? "models" : "settings" },
  ];
  const allDone = items.every((i) => i.done);
  $("#setupCard").hidden = allDone;
  $("#checklist").innerHTML = items
    .map((i) => `<li class="${i.done ? "done" : ""}"><span class="tick">${i.done ? "✓" : ""}</span>${esc(i.text)}${i.hint ? ` <span class="muted small">— ${esc(i.hint)}</span>` : ""}<span class="spacer"></span>${i.done ? "" : `<button class="btn ghost small" data-goto-btn="${i.goto}">Set up</button>`}</li>`)
    .join("");
  $$("[data-goto-btn]").forEach((b) => b.addEventListener("click", () => show(b.dataset.gotoBtn)));
}

// ------------------------------------------------------------------ engine

const ENGINE_LABELS = {
  running: "Engine ready",
  starting: "Engine starting…",
  stopped: "Engine idle",
  no_models: "Models needed",
  no_engine: "Engine not built",
};

async function refreshEngine() {
  state.engine = await invoke("engine_status");
  const pill = $("#enginePill");
  pill.dataset.state = state.engine.state;
  $(".label", pill).textContent = ENGINE_LABELS[state.engine.state] || state.engine.state;
  const notice = $("#engineNotice");
  if (state.engine.state === "no_engine") {
    notice.hidden = false;
    notice.textContent = "The yue-server engine is not built yet. In the project folder run:\n  ./scripts/build-engine.sh\nor point Settings → yue-server path at an existing build.";
  } else if (state.engine.message) {
    notice.hidden = false;
    notice.textContent = "The engine stopped:\n" + state.engine.message;
  } else notice.hidden = true;
  renderChecklist();
}
$("#enginePill").addEventListener("click", () => show(state.engine && state.engine.state === "no_models" ? "models" : "settings"));

// ------------------------------------------------------------------ models

const progress = {};

async function refreshCatalog() {
  state.catalog = await invoke("catalog");
  for (const p of state.catalog.progress) if (p.state === "running" || p.state === "verifying") progress[p.id] = p;
  renderModels();
  renderChecklist();
}

function modelRow(entry, { selectable, selected, recommended }) {
  const p = progress[entry.id];
  let side;
  if (entry.downloading || (p && (p.state === "running" || p.state === "verifying"))) {
    const pct = p && p.total ? Math.round((p.downloaded / p.total) * 100) : 0;
    const text = p && p.state === "verifying" ? "Verifying…" : p ? `${pct}% · ${fmtBytes(p.downloaded)} · ${(p.bytes_per_sec / 1024 ** 2).toFixed(1)} MB/s` : "Starting…";
    side = `<div class="bar"><div class="track-bar"><div class="fill-bar" style="width:${pct}%"></div></div><div class="bar-text">${text}</div></div>
      <button class="btn ghost small" data-dl-cancel="${entry.id}">Cancel</button>`;
  } else if (entry.installed) {
    side = `<span class="badge ok">Downloaded</span><button class="icon-btn" data-dl-delete="${entry.id}" title="Delete"><svg viewBox="0 0 24 24"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13"/></svg></button>`;
  } else {
    const resume = entry.partial > 0 ? "Resume" : "Download";
    side = `${p && p.state === "error" ? `<span class="small" style="color:var(--bad)" title="${esc(p.error)}">Failed</span>` : ""}<button class="btn primary small" data-dl-start="${entry.id}">${resume}</button>`;
  }
  const radio = selectable ? `<button class="radio ${selected ? "on" : ""}" data-select="${entry.id}" title="Use this one"></button>` : `<span class="radio fixed"></span>`;
  const meta = [
    entry.bytes ? `<span>${fmtBytes(entry.bytes)}</span>` : "",
    entry.ram_gb ? `<span>· ${entry.ram_gb} GB+ memory</span>` : "",
    recommended ? `<span class="badge rec">Recommended for this Mac</span>` : "",
    selected ? `<span class="badge">In use</span>` : "",
  ].join("");
  return `<div class="model ${selected ? "selected" : ""}">${radio}
    <div><div class="model-name">${esc(entry.label)}</div><div class="model-note">${esc(entry.note)}</div><div class="model-meta">${meta}</div></div>
    <div class="model-side">${side}</div></div>`;
}

function renderModels() {
  const c = state.catalog;
  const s = state.settings;
  if (!c || !s) return;
  const sys = state.system || {};
  const by = (kinds) => c.entries.filter((e) => kinds.includes(e.kind));
  $("#modelsMusic").innerHTML = [
    ...by(["backbone"]).map((e) => modelRow(e, { selectable: true, selected: e.id === s.backbone, recommended: e.id === sys.recommended_backbone })),
    ...by(["vae", "companion"]).map((e) => modelRow(e, { selectable: false })),
  ].join("");
  $("#modelsLlm").innerHTML = [
    ...by(["llm_runtime"]).map((e) => modelRow(e, { selectable: false })),
    ...by(["llm"]).map((e) => modelRow(e, { selectable: true, selected: s.llm_provider === "builtin" && e.id === s.llm_model, recommended: e.id === sys.recommended_llm })),
  ].join("");
  $("#modelsCovers").innerHTML = by(["transcriber", "transcriber_base"]).map((e) => modelRow(e, { selectable: false })).join("");
  const free = c.free_bytes != null ? ` · ${fmtBytes(c.free_bytes)} free` : "";
  $("#modelsMeta").textContent = `${c.models_dir}${free}`;
}

$("#view-models").addEventListener("click", async (e) => {
  const t = e.target.closest("[data-dl-start],[data-dl-cancel],[data-dl-delete],[data-select]");
  if (!t) return;
  const d = t.dataset;
  try {
    if (d.dlStart) {
      const entry = state.catalog.entries.find((x) => x.id === d.dlStart);
      if (state.catalog.free_bytes != null && entry.bytes > state.catalog.free_bytes) {
        toast(`Not enough disk space for ${entry.label} (${fmtBytes(entry.bytes)}).`, true);
        return;
      }
      progress[d.dlStart] = { id: d.dlStart, state: "running", downloaded: entry.partial, total: entry.bytes, bytes_per_sec: 0 };
      await invoke("download_start", { id: d.dlStart });
    } else if (d.dlCancel) {
      await invoke("download_cancel", { id: d.dlCancel });
    } else if (d.dlDelete) {
      const entry = state.catalog.entries.find((x) => x.id === d.dlDelete);
      const ok = T.dialog ? await T.dialog.ask(`Delete ${entry.label}? You can download it again later.`, { title: "Delete model", kind: "warning" }) : confirm("Delete?");
      if (!ok) return;
      await invoke("model_delete", { id: d.dlDelete });
    } else if (d.select) {
      const entry = state.catalog.entries.find((x) => x.id === d.select);
      const next = { ...state.settings };
      if (entry.kind === "backbone") next.backbone = entry.id;
      else {
        next.llm_model = entry.id;
        next.llm_provider = "builtin";
      }
      state.settings = await invoke("save_settings", { settings: next });
      toast(`Using ${entry.label}`);
    }
  } catch (err) {
    toast(String(err), true);
  }
  refreshCatalog();
  refreshEngine();
});

// ---------------------------------------------------------------- settings

function fillSettings() {
  const s = state.settings;
  const c = state.catalog;
  if (!s) return;
  const options = (kind, selected) =>
    (c ? c.entries.filter((e) => e.kind === kind) : []).map((e) => `<option value="${e.id}" ${e.id === selected ? "selected" : ""}>${esc(e.label)}${e.installed ? "" : " (not downloaded)"}</option>`).join("");
  $("#llmModel").innerHTML = options("llm", s.llm_model);
  $("#backbone").innerHTML = options("backbone", s.backbone);
  $("#ollamaUrl").value = s.ollama_url;
  $("#ollamaModel").value = s.ollama_model;
  $("#openaiUrl").value = s.openai_url;
  $("#openaiModel").value = s.openai_model;
  $("#openaiKey").value = s.openai_key;
  $("#unloadLlm").checked = s.unload_llm_before_music;
  $("#keepLoaded").checked = s.keep_loaded;
  $("#useCompanion").checked = s.use_companion;
  $("#maxSeq").value = s.max_seq ?? "";
  $("#outputFormat").value = s.output_format;
  $("#mp3Bitrate").value = String(s.mp3_bitrate);
  $("#enginePath").value = s.engine_path;
  $("#modelsDir").value = s.models_dir;
  setProvider(s.llm_provider);
  refreshStorage();
  const sys = state.system;
  if (sys) $("#systemInfo").textContent = `${sys.cpu} · ${sys.cores} cores · ${sys.ram_gb} GB memory · ${sys.os} (${sys.arch}). The music engine and llama.cpp run on the GPU through Metal.`;
}

let storage = null;

async function refreshStorage() {
  try {
    storage = await invoke("storage_info");
  } catch {
    return;
  }
  if (!storage) return;
  const total = storage.models_bytes + storage.runtime_bytes + storage.library_bytes;
  const pct = (bytes) => (total ? `${(bytes / total) * 100}%` : "0");
  $("#storageBar .seg-models").style.width = pct(storage.models_bytes);
  $("#storageBar .seg-runtime").style.width = pct(storage.runtime_bytes);
  $("#storageBar .seg-songs").style.width = pct(storage.library_bytes);
  $("#storageModels").textContent = storage.models_bytes ? fmtBytes(storage.models_bytes) : "None";
  $("#storageModelsMeta").textContent = storage.models_count ? `${storage.models_count} file${storage.models_count === 1 ? "" : "s"} · ${storage.models_dir}` : storage.models_dir;
  $("#storageRuntime").textContent = storage.runtime_bytes ? fmtBytes(storage.runtime_bytes) : "None";
  $("#storageSongs").textContent = storage.library_bytes ? fmtBytes(storage.library_bytes) : "None";
  $("#storageSongsMeta").textContent = `${storage.library_count} track${storage.library_count === 1 ? "" : "s"}`;
  $("#storageFree").textContent = `Using ${fmtBytes(total)} in all${storage.free_bytes != null ? ` · ${fmtBytes(storage.free_bytes)} free on this disk` : ""}.`;
  $("#removeAllModels").disabled = !(storage.models_bytes + storage.runtime_bytes);
}

$$("[data-reveal]").forEach((b) =>
  b.addEventListener("click", async () => {
    const path = storage && storage[b.dataset.reveal];
    if (!path) return;
    try { await reveal(path); } catch (e) { toast(String(e), true); }
  })
);

$("#removeAllModels").addEventListener("click", async () => {
  if (!storage) return;
  const size = fmtBytes(storage.models_bytes + storage.runtime_bytes);
  const message = `Remove all downloaded models and the llama.cpp runtime (${size})?\n\nYour songs are kept. You can download models again from the Models page.`;
  const ok = T.dialog ? await T.dialog.ask(message, { title: "Remove all models", kind: "warning" }) : confirm(message);
  if (!ok) return;
  try {
    const freed = await invoke("remove_all_models");
    toast(`Removed. ${fmtBytes(freed)} freed.`);
  } catch (e) {
    toast(String(e), true);
  }
  refreshStorage();
  refreshCatalog();
  refreshEngine();
});

let provider = "builtin";
function setProvider(value) {
  provider = value;
  $$("#providerSeg button").forEach((b) => b.classList.toggle("active", b.dataset.provider === value));
  $$(".provider-pane").forEach((p) => p.classList.toggle("active", p.dataset.pane === value));
}
$$("#providerSeg button").forEach((b) => b.addEventListener("click", () => setProvider(b.dataset.provider)));

function readSettings() {
  return {
    ...state.settings,
    llm_provider: provider,
    llm_model: $("#llmModel").value || state.settings.llm_model,
    backbone: $("#backbone").value || state.settings.backbone,
    ollama_url: $("#ollamaUrl").value.trim(),
    ollama_model: $("#ollamaModel").value.trim(),
    openai_url: $("#openaiUrl").value.trim(),
    openai_model: $("#openaiModel").value.trim(),
    openai_key: $("#openaiKey").value,
    unload_llm_before_music: $("#unloadLlm").checked,
    keep_loaded: $("#keepLoaded").checked,
    use_companion: $("#useCompanion").checked,
    max_seq: $("#maxSeq").value ? Number($("#maxSeq").value) : null,
    output_format: $("#outputFormat").value,
    mp3_bitrate: Number($("#mp3Bitrate").value),
    engine_path: $("#enginePath").value.trim(),
    models_dir: $("#modelsDir").value.trim(),
  };
}

async function saveSettings(quiet = false) {
  try {
    state.settings = await invoke("save_settings", { settings: readSettings() });
    if (!quiet) {
      $("#saveState").textContent = "Saved";
      setTimeout(() => ($("#saveState").textContent = ""), 2000);
    }
    refreshEngine();
    refreshCatalog();
  } catch (e) {
    toast(String(e), true);
  }
}
$("#saveSettings").addEventListener("click", () => saveSettings());

$("#llmTest").addEventListener("click", async () => {
  await saveSettings(true);
  const out = $("#llmTestResult");
  out.textContent = "Loading the model and asking it something…";
  try {
    out.textContent = "✓ " + (await invoke("llm_test"));
  } catch (e) {
    out.textContent = "✗ " + e;
  }
});

$("#ollamaRefresh").addEventListener("click", async () => {
  try {
    const models = await invoke("ollama_models", { url: $("#ollamaUrl").value });
    $("#ollamaList").innerHTML = models.map((m) => `<option value="${esc(m)}">`).join("");
    toast(models.length ? `${models.length} Ollama models found` : "Ollama has no models yet: ollama pull gemma3:12b");
    if (models.length && !$("#ollamaModel").value) $("#ollamaModel").value = models[0];
  } catch (e) {
    toast(String(e), true);
  }
});

$("#engineStart").addEventListener("click", async () => {
  await saveSettings(true);
  toast("Starting the engine…");
  try {
    await invoke("engine_start");
    toast("Engine ready");
  } catch (e) {
    toast(String(e), true);
  }
  refreshEngine();
});
$("#engineStop").addEventListener("click", async () => {
  await invoke("engine_stop");
  refreshEngine();
});
$("#engineLogBtn").addEventListener("click", async () => {
  const log = $("#engineLog");
  log.hidden = !log.hidden;
  if (!log.hidden) {
    log.textContent = (await invoke("engine_log")).join("\n") || "The engine has not written anything yet.";
    log.scrollTop = log.scrollHeight;
  }
});

// ------------------------------------------------------------------ events

listen("job", (job) => {
  upsertJob(job);
  if (job.state === "done") {
    loadTracks().then(() => {
      toast(`“${job.title}” is ready`);
      if (!state.current || audio.paused) {
        const first = job.track_ids[0];
        if (first) playTrack(first, sessionTracks().map((t) => t.id));
      }
    });
  }
  if (job.state === "error") toast(`“${job.title}” failed`, true);
  if (job.state === "running") refreshEngine();
});
listen("library", () => loadTracks());
listen("engine", () => refreshEngine());
listen("download", (p) => {
  progress[p.id] = p;
  if (p.state === "done") toast(`${(state.catalog?.entries.find((e) => e.id === p.id) || {}).label || p.id} downloaded`);
  if (p.state === "error") toast(`Download failed: ${p.error}`, true);
  if (p.state !== "running") {
    refreshCatalog();
    refreshEngine();
    if (state.view === "settings") refreshStorage();
  } else if (state.view === "models") renderModels();
});
listen("engine-log", (line) => {
  const log = $("#engineLog");
  if (!log.hidden) {
    log.textContent += "\n" + line;
    log.scrollTop = log.scrollHeight;
  }
  // The latest engine line is the most honest progress there is.
  const running = state.jobs.find((j) => j.state === "running");
  if (running && /Composing|Starting|Re-render/.test(running.stage)) {
    running.detail = line.replace(/^\[[^\]]*\]\s*/, "").slice(0, 90);
    renderSession();
  }
});

// -------------------------------------------------------------------- boot

(async function boot() {
  applyTheme(savedTheme);
  updateCounters();
  $(".create").dataset.tab = "song";
  try {
    [state.settings, state.system] = await Promise.all([invoke("get_settings"), invoke("system_info")]);
    state.jobs = await invoke("list_jobs");
    await Promise.all([refreshCatalog(), refreshEngine(), loadTracks()]);
    const ready = state.engine.state !== "no_engine" && state.engine.state !== "no_models";
    show(ready ? "create" : "home");
  } catch (e) {
    toast(String(e), true);
  }
  drawWave();
})();
