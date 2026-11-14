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
// Waveforms come from site/tools/song-peaks.py, lyric times from
// site/tools/lyrics-timing.py ([seconds, line, starts a section]).
const SONGS = [
  {
    src: "audio/shayad-kabhi-na-keh-sakoon.mp3", title: "Shayad Kabhi Na Keh Sakoon", about: "Hindi · Sad Hindi song", seconds: 180,
    colors: ["#5b3fd6", "#141238"],
    motif: '<path d="M60 22a29 29 0 1 0 18 50 25 25 0 0 1-18-50Z" fill="#fff"/>',
    peaks: [39, 37, 45, 48, 37, 50, 35, 45, 73, 68, 29, 68, 72, 57, 32, 74, 76, 68, 63, 71, 41, 88, 90, 94, 71, 90, 89, 81, 58, 34, 92, 95, 92, 83, 99, 92, 86, 89, 90, 91, 89, 100, 93, 93, 83, 51, 73, 76, 94, 72, 92, 87, 83, 78, 83, 84, 86, 91, 85, 84, 82, 76, 88, 90, 95, 70, 91, 83, 88, 53, 42, 88, 89, 90, 85, 95, 83, 81, 85, 85, 91, 75, 97, 86, 88, 92, 80, 83, 89, 91, 82, 82, 74, 76, 56, 47],
    synced: true,
    lyrics: [[17.71, "Shayad kabhi na keh sakoon main tumko", 1], [32.08, "Kahe bina samajh lo tum shayad", 0], [42.5, "Har pal tera hi khayal aata hai", 0], [49.38, "Dil yeh mera bas tujhe hi chahta hai", 0], [60, "Shayad mere khayal mein tum ik din", 1], [62.08, "Milo mujhe kahin pe ghum shaayad", 0], [66.67, "Yeh tanhaai ka safar hai kaisa", 0], [68.75, "Mujhe ab bas tera hi intezaar hai", 0], [73.33, "Jo tum na ho rahenge hum nahin", 1], [74.79, "Jo tum na ho rahenge hum nahin", 0], [76.67, "Na chahiye kuch tum se zyada tum se kam nahin", 0], [82.08, "Jo tum na ho toh hum bhi hum nahi", 0], [85.62, "Jo tum na ho toh hum bhi hum nahi", 0], [90, "Yaadein teri ab chubhti hain dil mein", 1], [100.42, "Hari pal teri kami hai har pal", 0], [112.29, "Aankhon ko khaab dena khud hi sawaal karke", 0], [122.92, "Khud hi jawaab dena teri taraf se bin kaam kaa", 0], [136.67, "Shayad mere khayal mein tum ik din", 1], [138.75, "Milo mujhe kahin pe ghum shaayad", 0], [143.33, "Yeh tanhaai ka safar hai kaisa", 0], [145.42, "Mujhe ab bas tera hi intezaar hai", 0], [150, "Jo tum na ho rahenge hum nahin", 1], [151.46, "Jo tum na ho rahenge hum nahin", 0], [155.62, "Na chahiye kuch tum se zyada tum se kam nahin", 0], [158.75, "Jo tum na ho toh hum bhi hum nahi", 0], [162.5, "Jo tum na ho toh hum bhi hum nahi", 0], [166.67, "Yeh rishta mera ab adhura hai", 1], [170.83, "Bina tum ke jeena mushkil hai", 0], [174.58, "Na koi shikwa na koi gila", 0], [177.5, "Bas teri hi yaad hai ab dil mein", 0]],
  },
  {
    src: "audio/baarish-mein-tum-milte.mp3", title: "Baarish Mein Tum Milte", about: "Hindi · Bollywood romantic", seconds: 180,
    colors: ["#18a0a8", "#123a5c"],
    motif: '<path d="M36 22 26 44M60 16 50 38M82 24 72 46M32 56 22 78M56 50 46 72M78 58 68 80" stroke="#fff" stroke-width="4" stroke-linecap="round"/>',
    peaks: [18, 19, 16, 28, 33, 32, 56, 69, 67, 56, 63, 66, 58, 77, 70, 77, 72, 70, 80, 82, 82, 84, 76, 74, 79, 75, 76, 61, 83, 88, 84, 88, 81, 84, 90, 84, 96, 88, 86, 67, 24, 89, 81, 86, 84, 80, 92, 89, 88, 95, 84, 89, 88, 84, 72, 69, 94, 99, 89, 98, 93, 94, 95, 89, 94, 92, 89, 71, 75, 91, 92, 84, 98, 90, 87, 97, 88, 91, 92, 91, 73, 67, 72, 75, 56, 74, 68, 61, 29, 60, 98, 100, 85, 100, 97, 88],
    synced: false,
    lyrics: [[27.69, "baar baar yeh baarish", 1], [30.58, "tum ki yaad aati hai", 0], [34.62, "kaise kahoon tum", 0], [36.92, "yeh dil kya jaati hai", 0], [41.15, "wahi gali, wahi mod", 1], [44.42, "wahi purani baatein", 0], [48.27, "mod ke mod pe mile hum", 0], [51.54, "jaise khwaab ki raatein", 0], [58.46, "baarish mein tum milte", 1], [63.85, "meri duniya ban jao", 0], [70.77, "tum hi toh ho mera saath", 0], [75.38, "meri har khushi ho", 0], [82.31, "thodi si bheeg gayi", 1], [85.38, "wo purani shaam", 0], [88.46, "tum ke bina yeh jeevan", 0], [90.77, "kitna hai be-naam", 0], [95, "wahi gali, wahi mod", 1], [98.85, "wahi purani baatein", 0], [102.69, "mod ke mod pe mile hum", 0], [108.46, "jaise khwaab ki raatein", 0], [114.04, "baarish mein tum milte", 1], [119.62, "meri duniya ban jao", 0], [125.77, "tum hi toh ho mera saath", 0], [157.31, "meri har khushi ho", 0], [160.58, "yeh baarish ka ehsaas", 1], [164.23, "tu mere paas hai", 0], [166.73, "yeh pal ruk jaye bas", 0], [173.46, "yeh mohabbat khaas hai", 0], [178.85, "baarish mein tum milte", 1]],
  },
  {
    src: "audio/my-happy-little-three.mp3", title: "My Happy Little Three", about: "English · Kids song", seconds: 164,
    colors: ["#ffb13b", "#f0457e"],
    motif: '<circle cx="50" cy="50" r="17" fill="#fff"/><path d="M50 14v10M50 76v10M14 50h10M76 50h10M24.5 24.5l7 7M68.5 68.5l7 7M75.5 24.5l-7 7M31.5 68.5l-7 7" stroke="#fff" stroke-width="4" stroke-linecap="round"/>',
    peaks: [51, 50, 53, 43, 62, 64, 55, 64, 50, 72, 62, 65, 63, 45, 83, 71, 88, 92, 91, 88, 85, 86, 85, 84, 85, 89, 79, 77, 77, 58, 74, 57, 70, 74, 60, 67, 61, 68, 68, 80, 92, 69, 63, 70, 78, 67, 66, 59, 61, 61, 53, 92, 94, 93, 85, 92, 87, 95, 86, 92, 95, 85, 90, 59, 56, 57, 51, 69, 74, 66, 74, 100, 90, 93, 93, 89, 90, 85, 93, 87, 92, 89, 90, 88, 88, 64, 66, 54, 56, 61, 69, 51, 60, 50, 61, 40],
    synced: true,
    lyrics: [[8, "You are three years old and sweet,", 1], [13.5, "With tiny dancing happy feet.", 0], [19.75, "A little bit of naughty fun,", 0], [25.12, "You are cutest under sun.", 0], [28, "Ready to play, ready to leap,", 1], [29, "Secrets that your heart does keep.", 0], [30, "Oh, my little star, oh so bright,", 1], [34, "Dancing and painting day and night.", 0], [38, "Pancakes taste so yummy too,", 0], [42.75, "My sweet little boy is you.", 0], [48, "Brush of color, bright and bold,", 1], [54, "Stories waiting to unfold.", 0], [59.75, "With a book in hand, eyes so keen,", 0], [65.12, "The cleverest reader ever seen.", 0], [70, "Ready to play, ready to leap,", 1], [80.5, "Secrets that your heart does keep.", 0], [90, "Oh, my little star, oh so bright,", 1], [94.5, "Dancing and painting day and night.", 0], [98.5, "Pancakes taste so yummy too,", 0], [102.88, "My sweet little boy is you.", 0], [108, "Spin around like a happy breeze,", 1], [115.5, "Bringing joy and gentle ease.", 0], [117.75, "Paint the sky, read the page,", 0], [123, "A wonderful, magical age.", 0], [128, "Oh, my little star, oh so bright,", 1], [132, "Dancing and painting day and night.", 0], [136, "Pancakes taste so yummy too,", 0], [141, "My sweet little boy is you.", 0], [146, "Pancakes and books and bright art,", 1], [151, "You have captured my heart.", 0], [154.5, "Little dancer, sweet and true,", 0], [158.5, "We love you, little you.", 0]],
  },
  {
    src: "audio/little-star-light.mp3", title: "Little Star Light", about: "English · Christmas pop", seconds: 136,
    colors: ["#d3263f", "#4a0f2a"],
    motif: '<path d="M46 16c3 25 13 35 38 38-25 3-35 13-38 38-3-25-13-35-38-38 25-3 35-13 38-38Z" fill="#ffd98a"/><path d="M80 14c1 7 4 10 11 11-7 1-10 4-11 11-1-7-4-10-11-11 7-1 10-4 11-11Z" fill="#ffd98a"/>',
    peaks: [8, 40, 42, 39, 65, 76, 57, 66, 68, 64, 64, 58, 69, 72, 55, 75, 71, 50, 87, 82, 80, 79, 92, 58, 78, 95, 100, 93, 93, 95, 96, 100, 98, 92, 99, 100, 70, 42, 59, 90, 88, 90, 92, 94, 90, 93, 93, 89, 94, 87, 83, 75, 67, 79, 80, 63, 62, 90, 97, 90, 89, 92, 96, 87, 91, 89, 91, 97, 78, 75, 64, 51, 69, 66, 54, 34, 78, 81, 91, 93, 87, 91, 89, 93, 90, 88, 95, 95, 76, 38, 72, 66, 47, 64, 58, 8],
    synced: true,
    lyrics: [[10, "Snow is falling outside my window pane", 1], [15, "Winter whispers sing a soft sweet refrain", 0], [18.5, "Tiny reindeer start to pull the sled", 0], [22.25, "Bringing gifts to everyone in bed", 0], [26, "Oh the magic of the season bright", 1], [30.5, "Filled with joy and pure delight", 0], [36, "Little star light shining in the night", 1], [39.88, "Merry Christmas, shining ever so bright", 0], [43.25, "Little star light, shining for all to see", 0], [48.38, "Happy holidays for you and me", 0], [56, "Santa Claus is coming through the chimney smoke", 1], [60.5, "Little cocoa treats that we all have stoked", 0], [65.38, "Bells are ringing out a happy sweet sound", 0], [68.5, "Joy and laughter all around the town", 0], [72, "Oh the magic of the season bright", 1], [76, "Filled with joy and pure delight", 0], [82, "Little star light shining in the night", 1], [86, "Merry Christmas, shining ever so bright", 0], [89.25, "Little star light, shining for all to see", 0], [94, "Happy holidays for you and me", 0], [98, "Wishing you a season filled with cheer", 1], [103, "Hoping that the magic is always near", 0], [110, "Little star light shining in the night", 1], [114, "Merry Christmas, shining ever so bright", 0], [117.25, "Little star light, shining for all to see", 0], [120.75, "Happy holidays for you and me", 0], [126, "Shining in the night", 1], [130, "Happy holidays tonight", 0], [133.5, "Little star light", 0]],
  },
  {
    src: "audio/new-year-dreams.mp3", title: "New Year Dreams", about: "English · Uplifting pop", seconds: 168,
    colors: ["#2b5cff", "#2a0b5c"],
    motif: '<path d="M50 36V20M64 50h16M50 64v16M36 50H20M59.9 40.1l11.3-11.3M59.9 59.9l11.3 11.3M40.1 59.9 28.8 71.2M40.1 40.1 28.8 28.8" stroke="#fff" stroke-width="4" stroke-linecap="round"/><g fill="#ffd98a"><circle cx="50" cy="12" r="3.5"/><circle cx="88" cy="50" r="3.5"/><circle cx="50" cy="88" r="3.5"/><circle cx="12" cy="50" r="3.5"/><circle cx="76.9" cy="23.1" r="3.5"/><circle cx="76.9" cy="76.9" r="3.5"/><circle cx="23.1" cy="76.9" r="3.5"/><circle cx="23.1" cy="23.1" r="3.5"/><circle cx="50" cy="50" r="5"/></g>',
    peaks: [21, 33, 28, 22, 42, 63, 63, 30, 31, 72, 56, 28, 41, 71, 80, 64, 85, 71, 63, 87, 91, 83, 93, 90, 96, 92, 91, 74, 92, 93, 94, 96, 95, 92, 87, 92, 70, 24, 66, 89, 92, 90, 94, 95, 93, 89, 91, 84, 82, 78, 87, 83, 85, 82, 77, 75, 92, 91, 91, 96, 94, 95, 90, 94, 91, 93, 90, 92, 91, 91, 95, 95, 87, 57, 62, 59, 66, 61, 64, 62, 72, 89, 92, 99, 100, 97, 96, 98, 100, 93, 45, 41, 34, 60, 37, 8],
    synced: true,
    lyrics: [[10.08, "Another year begins today", 1], [13.12, "With sunshine lighting up the way", 0], [20.39, "We shed the past like old clothes", 0], [29.06, "Ready for the way life goes", 0], [35.86, "New chances wait for us to find", 1], [40.08, "Leaving shadows far behind", 0], [42.66, "Hope is glowing in our hearts", 0], [46.88, "Ready for the brand new starts", 0], [51.33, "Happy New Year, shining bright", 1], [55.08, "Fill our future with delight", 0], [58.83, "New adventures, bold and true", 0], [62.58, "A brighter world for me and you", 0], [70.08, "We make a wish upon a star", 1], [74.77, "No matter where we are", 0], [77.81, "We smile at all that's yet to be", 0], [81.8, "Living life with melody", 0], [84.61, "New chances wait for us to find", 1], [88.83, "Leaving shadows far behind", 0], [91.41, "Hope is glowing in our hearts", 0], [95.62, "Ready for the brand new starts", 0], [100.08, "Happy New Year, shining bright", 1], [103.83, "Fill our future with delight", 0], [107.58, "New adventures, bold and true", 0], [110.62, "A brighter world for me and you", 0], [114.38, "Let's celebrate the turning page", 1], [119.53, "Write a story on this stage", 0], [123.05, "With laughter ringing loud and clear", 0], [126.56, "Chasing all the doubts and fear", 0], [129.38, "Happy New Year, shining bright", 1], [133.83, "Fill our future with delight", 0], [136.88, "New adventures, bold and true", 0], [140.62, "A brighter world for me and you", 0], [144.38, "Happy New Year to you", 1], [152.11, "Happy New Year, fresh and new", 0], [160.31, "Happy New Year to you", 0]],
  },
  {
    src: "audio/babu-singhs-happy-dance.mp3", title: "Babu Singh's Happy Dance", about: "English · Kids song", seconds: 141,
    colors: ["#22c55e", "#0e7490"],
    motif: '<path d="M38 70V30l34-9v41" stroke="#fff" stroke-width="5" fill="none" stroke-linejoin="round"/><circle cx="30" cy="70" r="9" fill="#fff"/><circle cx="64" cy="62" r="9" fill="#fff"/><path d="M80 18c1 6 3 8 9 9-6 1-8 3-9 9-1-6-3-8-9-9 6-1 8-3 9-9Z" fill="#fff"/>',
    peaks: [45, 44, 46, 88, 85, 88, 87, 92, 90, 86, 79, 94, 89, 94, 90, 79, 81, 75, 84, 89, 81, 91, 97, 87, 93, 93, 100, 76, 72, 71, 83, 92, 89, 80, 87, 87, 72, 84, 84, 86, 91, 85, 82, 85, 74, 87, 87, 75, 84, 94, 83, 90, 92, 100, 76, 71, 71, 79, 81, 73, 72, 73, 79, 86, 88, 84, 78, 90, 77, 85, 75, 81, 82, 86, 87, 94, 85, 90, 93, 88, 70, 73, 78, 68, 74, 62, 64, 71, 68, 84, 98, 94, 83, 92, 68, 16],
    synced: true,
    lyrics: [[6.67, "Babu Singh is three years old,", 1], [8.75, "A story of wonder to unfold.", 0], [10.97, "He loves to read a book so bright,", 0], [13.47, "Reading pages morning and night.", 0], [15.56, "But wait, there's something sweet to see,", 1], [18.33, "Pancakes piled up happily.", 0], [21.39, "Mmm, sweet and golden, soft and round,", 0], [24.72, "Best breakfast ever can be found.", 0], [28.89, "Babu Singh, Babu Singh, oh what a sight!", 1], [33.06, "Dancing and singing with all his might.", 0], [37.78, "Cute and naughty, happy and free,", 0], [41.94, "Babu Singh is the best for me!", 0], [46.67, "When the paint comes out to play,", 1], [48.89, "Bright colors brighten up the day.", 0], [51.11, "He paints a sun and a happy tree,", 0], [53.47, "A little messy, wild and free.", 0], [55.56, "But wait, there's something sweet to see,", 1], [58.33, "Pancakes piled up happily.", 0], [61.39, "Mmm, sweet and golden, soft and round,", 0], [64.72, "Best breakfast ever can be found.", 0], [68.89, "Babu Singh, Babu Singh, oh what a sight!", 1], [73.06, "Dancing and singing with all his might.", 0], [78.33, "Cute and naughty, happy and free,", 0], [81.94, "Babu Singh is the best for me!", 0], [86.67, "He spins and twirls when the music starts,", 1], [90.97, "A little dancer with joyful hearts.", 0], [94.44, "Naughty and sweet, a bundle of fun,", 0], [98.61, "He makes every playtime second second.", 0], [106.67, "Babu Singh, Babu Singh, oh what a sight!", 1], [110.83, "Dancing and singing with all his might.", 0], [116.11, "Cute and naughty, happy and free,", 0], [120, "Babu Singh is the best for me!", 0], [124.44, "Babu Singh, happy and bright,", 1], [128.89, "Dancing with all of his might.", 0], [133.06, "Sweet and cute, a joyful sound,", 0], [137.22, "Babu Singh spins around.", 0]],
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
const escapeText = (text) => text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
let lyricLine = -2;
// Lines light up this far ahead of their note, so the eye is there as the voice starts.
const LYRIC_LEAD = 0.5;

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
  followLyrics(seconds);
}

function showLyrics(song) {
  songPart("lines").innerHTML = song.lyrics
    .map(([, text, first], index) => `<p class="sp-line${first && index ? " sp-gap" : ""}">${escapeText(text)}</p>`)
    .join("");
  songPart("lyrics").classList.toggle("untimed", !song.synced);
  songPart("lyrics-note").textContent = song.synced ? "Follows the music" : "Not timed for this song";
  songPart("view").scrollTop = 0;
  lyricLine = -2;
}

// The line being sung lights up and sits a little above the middle, as in Apple Music.
function followLyrics(seconds) {
  const song = SONGS[shownSong];
  if (!song.synced) return;
  let index = -1;
  while (index + 1 < song.lyrics.length && song.lyrics[index + 1][0] <= seconds + LYRIC_LEAD) index++;
  if (index === lyricLine) return;
  lyricLine = index;
  const lines = [...songPart("lines").children];
  lines.forEach((line, i) => {
    line.classList.toggle("active", i === index);
    line.classList.toggle("past", i < index);
  });
  const offset = index < 0 ? 0 : lines[index].offsetTop - songPart("view").clientHeight * 0.36;
  songPart("lines").style.transform = `translateY(${-Math.max(0, offset)}px)`;
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
  showLyrics(song);
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
function seekTo(seconds, play) {
  showProgress(seconds);
  if (loadedSong === shownSong && player.readyState >= 1) {
    player.currentTime = seconds;
    if (play && player.paused) playSong(shownSong);
  } else {
    pendingSeek = seconds;
    playSong(shownSong);
  }
}

songPart("seek").addEventListener("input", (e) => seekTo((e.target.value / 1000) * SONGS[shownSong].seconds, false));
songPart("lines").addEventListener("click", (e) => {
  const line = e.target.closest(".sp-line");
  const song = SONGS[shownSong];
  if (line && song.synced) seekTo(song.lyrics[[...songPart("lines").children].indexOf(line)][0], true);
});
player.addEventListener("loadedmetadata", () => {
  if (pendingSeek !== null) player.currentTime = pendingSeek;
  pendingSeek = null;
});
// timeupdate comes only about four times a second; lyrics check every frame.
function followEachFrame() {
  if (player.paused) return;
  followLyrics(player.currentTime);
  requestAnimationFrame(followEachFrame);
}
player.addEventListener("play", paintSongs);
player.addEventListener("playing", () => requestAnimationFrame(followEachFrame));
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
