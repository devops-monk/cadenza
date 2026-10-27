// Cadenza website: scroll reveals, the typing demo, and download links that
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
