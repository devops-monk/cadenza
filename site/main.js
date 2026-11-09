// Cadenza website: scroll reveals, the typing demo, example songs, and download links that
// point at the right file of the latest GitHub release.

// The GitHub repository whose Releases hold the installers.
const REPO = "devops-monk/cadenza";
const RELEASES = `https://github.com/${REPO}/releases/latest`;

// ------------------------------------------------------------- reveals
const observer = new IntersectionObserver(
  (entries) => entries.forEach((e) => { if (e.isIntersecting) { e.target.classList.add("in"); observer.unobserve(e.target); } }),
  { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
);
document.querySelectorAll(".reveal, .reveal-up").forEach((el) => observer.observe(el));

// ---------------------------------------------------------- typing demo
const demos = [
  { text: "compose a hindi song for my 3 years old son. his name is babu singh and he loves music and he goes to nursery and likes painting", title: "Babu Paints the Sky", meta: "Hindi · Children's pop · 2:55" },
  { text: "an upbeat 80s synth-pop anthem about driving through a neon city at midnight, female vocals", title: "Neon Midnight Drive", meta: "English · Synth-pop · 3:34" },
  { text: "a warm acoustic folk ballad about my grandmother's kitchen on Sunday mornings", title: "Grandma's Kitchen", meta: "English · Folk · 3:08" },
  { text: "a Punjabi bhangra party song with dhol, for my sister's wedding sangeet", title: "Nachdi Phiri", meta: "Punjabi · Bhangra · 3:21" },
];
const typed = document.getElementById("typed");
const result = document.querySelector(".result");
const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

async function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }

async function runDemo() {
  let i = 0;
  for (;;) {
    const demo = demos[i++ % demos.length];
    result.classList.remove("show");
    typed.textContent = "";
    if (reduce) typed.textContent = demo.text;
    else for (const ch of demo.text) { typed.textContent += ch; await sleep(28 + Math.random() * 30); }
    await sleep(500);
    document.getElementById("typedTitle").textContent = demo.title;
    document.getElementById("typedMeta").textContent = demo.meta;
    result.classList.add("show");
    await sleep(4200);
  }
}
const demoObserver = new IntersectionObserver((entries) => {
  if (entries.some((e) => e.isIntersecting)) { demoObserver.disconnect(); runDemo(); }
});
demoObserver.observe(document.querySelector(".typed-card"));

// -------------------------------------------------------- example songs
// Waveforms come from site/tools/song-peaks.py.
const SONGS = [
  {
    src: "audio/shayad-kabhi-na-keh-sakoon.mp3", title: "Shayad Kabhi Na Keh Sakoon", about: "Hindi · Sad Hindi song", seconds: 180,
    colors: ["#5b3fd6", "#141238"],
    motif: '<path d="M60 22a29 29 0 1 0 18 50 25 25 0 0 1-18-50Z" fill="#fff"/>',
    peaks: [39, 37, 45, 48, 37, 50, 35, 45, 73, 68, 29, 68, 72, 57, 32, 74, 76, 68, 63, 71, 41, 88, 90, 94, 71, 90, 89, 81, 58, 34, 92, 95, 92, 83, 99, 92, 86, 89, 90, 91, 89, 100, 93, 93, 83, 51, 73, 76, 94, 72, 92, 87, 83, 78, 83, 84, 86, 91, 85, 84, 82, 76, 88, 90, 95, 70, 91, 83, 88, 53, 42, 88, 89, 90, 85, 95, 83, 81, 85, 85, 91, 75, 97, 86, 88, 92, 80, 83, 89, 91, 82, 82, 74, 76, 56, 47],
  },
  {
    src: "audio/baarish-mein-tum-milte.mp3", title: "Baarish Mein Tum Milte", about: "Hindi · Bollywood romantic", seconds: 180,
    colors: ["#18a0a8", "#123a5c"],
    motif: '<path d="M36 22 26 44M60 16 50 38M82 24 72 46M32 56 22 78M56 50 46 72M78 58 68 80" stroke="#fff" stroke-width="4" stroke-linecap="round"/>',
    peaks: [18, 19, 16, 28, 33, 32, 56, 69, 67, 56, 63, 66, 58, 77, 70, 77, 72, 70, 80, 82, 82, 84, 76, 74, 79, 75, 76, 61, 83, 88, 84, 88, 81, 84, 90, 84, 96, 88, 86, 67, 24, 89, 81, 86, 84, 80, 92, 89, 88, 95, 84, 89, 88, 84, 72, 69, 94, 99, 89, 98, 93, 94, 95, 89, 94, 92, 89, 71, 75, 91, 92, 84, 98, 90, 87, 97, 88, 91, 92, 91, 73, 67, 72, 75, 56, 74, 68, 61, 29, 60, 98, 100, 85, 100, 97, 88],
  },
  {
    src: "audio/my-happy-little-three.mp3", title: "My Happy Little Three", about: "English · Kids song", seconds: 165,
    colors: ["#ffb13b", "#f0457e"],
    motif: '<circle cx="50" cy="50" r="17" fill="#fff"/><path d="M50 14v10M50 76v10M14 50h10M76 50h10M24.5 24.5l7 7M68.5 68.5l7 7M75.5 24.5l-7 7M31.5 68.5l-7 7" stroke="#fff" stroke-width="4" stroke-linecap="round"/>',
    peaks: [51, 50, 53, 43, 62, 64, 55, 64, 50, 72, 62, 65, 63, 45, 83, 71, 88, 92, 91, 88, 85, 86, 85, 84, 85, 89, 79, 77, 77, 58, 74, 57, 70, 74, 60, 67, 61, 68, 68, 80, 92, 69, 63, 70, 78, 67, 66, 59, 61, 61, 53, 92, 94, 93, 85, 92, 87, 95, 86, 92, 95, 85, 90, 59, 56, 57, 51, 69, 74, 66, 74, 100, 90, 93, 93, 89, 90, 85, 93, 87, 92, 89, 90, 88, 88, 64, 66, 54, 56, 61, 69, 51, 60, 50, 61, 40],
  },
  {
    src: "audio/little-star-light.mp3", title: "Little Star Light", about: "English · Christmas pop", seconds: 137,
    colors: ["#d3263f", "#4a0f2a"],
    motif: '<path d="M46 16c3 25 13 35 38 38-25 3-35 13-38 38-3-25-13-35-38-38 25-3 35-13 38-38Z" fill="#ffd98a"/><path d="M80 14c1 7 4 10 11 11-7 1-10 4-11 11-1-7-4-10-11-11 7-1 10-4 11-11Z" fill="#ffd98a"/>',
    peaks: [8, 40, 42, 39, 65, 76, 57, 66, 68, 64, 64, 58, 69, 72, 55, 75, 71, 50, 87, 82, 80, 79, 92, 58, 78, 95, 100, 93, 93, 95, 96, 100, 98, 92, 99, 100, 70, 42, 59, 90, 88, 90, 92, 94, 90, 93, 93, 89, 94, 87, 83, 75, 67, 79, 80, 63, 62, 90, 97, 90, 89, 92, 96, 87, 91, 89, 91, 97, 78, 75, 64, 51, 69, 66, 54, 34, 78, 81, 91, 93, 87, 91, 89, 93, 90, 88, 95, 95, 76, 38, 72, 66, 47, 64, 58, 8],
  },
];

const stage = document.getElementById("songPlayer");
const player = new Audio();
player.preload = "none";
let shownSong = 0;
let loadedSong = -1;
let pendingSeek = null;

const clock = (seconds) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
const songPart = (name) => stage.querySelector(`.sp-${name}`);
const songMotif = (song) => `<svg viewBox="0 0 100 100" aria-hidden="true">${song.motif}</svg>`;
const songColors = (song) => `--a:${song.colors[0]};--b:${song.colors[1]}`;

songPart("list").innerHTML = SONGS.map((song, index) => `<li>
  <button class="sp-row" type="button" data-song="${index}">
    <span class="sp-row-art" style="${songColors(song)}">${songMotif(song)}</span>
    <span class="sp-row-text"><strong>${song.title}</strong><span>${song.about}</span></span>
    <span class="sp-eq" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
    <time>${clock(song.seconds)}</time>
  </button>
</li>`).join("");

function showProgress(seconds) {
  const fraction = Math.min(1, seconds / SONGS[shownSong].seconds);
  songPart("wave").style.setProperty("--progress", `${fraction * 100}%`);
  songPart("seek").value = fraction * 1000;
  songPart("now").textContent = clock(seconds);
}

function showSong(index) {
  shownSong = index;
  const song = SONGS[index];
  stage.style.cssText = songColors(song);
  songPart("art").innerHTML = songMotif(song);
  songPart("about").textContent = song.about;
  songPart("title").textContent = song.title;
  songPart("total").textContent = clock(song.seconds);
  songPart("seek").setAttribute("aria-label", `Position in ${song.title}`);
  const bars = song.peaks.map((peak) => `<i style="height:${peak}%"></i>`).join("");
  stage.querySelectorAll(".sp-bars").forEach((layer) => (layer.innerHTML = bars));
  stage.querySelectorAll(".sp-row").forEach((row) => {
    const active = Number(row.dataset.song) === index;
    row.classList.toggle("active", active);
    if (active) row.setAttribute("aria-current", "true");
    else row.removeAttribute("aria-current");
  });
  showProgress(0);
}

function paintSongs() {
  stage.classList.toggle("playing", !player.paused);
  songPart("play").setAttribute("aria-label", player.paused ? "Play" : "Pause");
}

function playSong(index) {
  if (index !== loadedSong) {
    showSong(index);
    loadedSong = index;
    player.src = SONGS[index].src;
  }
  player.play().catch(songFailed);
}

function songFailed(error) {
  if (error?.name === "AbortError") return;
  songPart("about").textContent = "This song could not be played.";
}

songPart("play").addEventListener("click", () => (player.paused ? playSong(shownSong) : player.pause()));
songPart("next").addEventListener("click", () => playSong((shownSong + 1) % SONGS.length));
songPart("prev").addEventListener("click", () => {
  if (player.currentTime > 3 || shownSong === 0) player.currentTime = 0;
  else playSong(shownSong - 1);
});
songPart("list").addEventListener("click", (e) => {
  const row = e.target.closest(".sp-row");
  if (!row) return;
  const index = Number(row.dataset.song);
  if (index === loadedSong && !player.paused) player.pause();
  else playSong(index);
});
songPart("seek").addEventListener("input", (e) => {
  const seconds = (e.target.value / 1000) * SONGS[shownSong].seconds;
  showProgress(seconds);
  if (loadedSong === shownSong && player.readyState >= 1) {
    player.currentTime = seconds;
  } else {
    pendingSeek = seconds;
    playSong(shownSong);
  }
});
player.addEventListener("loadedmetadata", () => {
  if (pendingSeek !== null) player.currentTime = pendingSeek;
  pendingSeek = null;
});
player.addEventListener("play", paintSongs);
player.addEventListener("pause", paintSongs);
player.addEventListener("error", () => songFailed());
player.addEventListener("timeupdate", () => showProgress(player.currentTime));
player.addEventListener("ended", () => {
  if (shownSong < SONGS.length - 1) playSong(shownSong + 1);
  else {
    player.currentTime = 0;
    showProgress(0);
  }
});
showSong(0);

// ----------------------------------------------------------- downloads
const links = { mac: document.querySelector('[data-os="mac"]'), windows: document.querySelector('[data-os="windows"]'), linux: document.querySelector('[data-os="linux"]') };
Object.values(links).forEach((a) => (a.href = RELEASES));
document.getElementById("allReleases").href = `https://github.com/${REPO}/releases`;

const platform = (navigator.userAgentData?.platform || navigator.platform || navigator.userAgent).toLowerCase();
const mine = platform.includes("mac") ? "mac" : platform.includes("win") ? "windows" : platform.includes("linux") ? "linux" : null;
if (mine) links[mine].classList.add("mine");

fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
  .then((r) => (r.ok ? r.json() : null))
  .then((release) => {
    if (!release) return;
    const find = (re) => release.assets.find((a) => re.test(a.name))?.browser_download_url;
    const urls = { mac: find(/\.dmg$/i), windows: find(/setup\.exe$/i) || find(/\.msi$/i), linux: find(/\.AppImage$/i) || find(/\.deb$/i) };
    for (const [os, url] of Object.entries(urls)) if (url) links[os].href = url;
  })
  .catch(() => {});
