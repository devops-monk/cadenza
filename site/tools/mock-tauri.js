// A stand-in for the Rust backend so the real UI can be screenshotted for the
// website without models: demo tracks, a running job, a download in progress.
// Scene comes from the URL hash: #create, #library, #models, #drawer, #home, #settings, plus ",dark".

(function () {
  const [scene, theme] = (location.hash.slice(1) || "create").split(",");
  try { localStorage.setItem("cadenza-theme", theme === "dark" ? "dark" : "light"); } catch {}

  const now = Date.now();
  const iso = (minutesAgo) => new Date(now - minutesAgo * 60000).toISOString();
  const tracks = [
    { id: "a1f3c2d4-babu", title: "Babu Paints the Sky", kind: "song", duration: 175, created_at: iso(-1), favorite: true,
      description: "compose a hindi song for my 3 years old son. his name is babu singh and he loves music and he goes to nursery and likes painting",
      style: "A bright, playful Hindi children's pop song at 112 BPM. Ukulele, glockenspiel and hand claps over a bouncy bass; a warm, smiling female vocal sings simply and clearly. Verses stay light, the chorus opens up with a sing-along hook and gentle backing harmonies.",
      lyrics: "[Verse]\nछोटा सा बाबू, आँखों में तारे\nरंगों की डिब्बी, सपने हमारे\nनर्सरी जाए, गाना गाए\nहर दिन नई धुन बनाए\n\n[Chorus]\nबाबू सिंह, रंग भरो ना\nआसमान को नीला करो ना\nला ला ला, गाओ ज़रा\nबाबू का गाना सबसे प्यारा\n\n[Verse]\nब्रश ले के सूरज बनाए\nपीला, लाल, हरा सजाए\nमम्मी हँसे, पापा गाएँ\nबाबू के संग नाचे जाएँ\n\n[Chorus]\nबाबू सिंह, रंग भरो ना\nआसमान को नीला करो ना\nला ला ला, गाओ ज़रा\nबाबू का गाना सबसे प्यारा",
      abc: "X:1\nT:Babu Paints the Sky\nM:4/4\nL:1/8\nQ:1/4=112\nK:C\n% verse\n\"C\" E2 G2 G2 A2 | \"G\" G4 E4 | \"F\" F2 A2 A2 c2 | \"C\" G8 |\n% chorus\n\"F\" c2 c2 A2 c2 | \"G\" d4 B4 | \"Am\" c2 B2 A2 G2 | \"C\" E8 |",
      lm_seed: 482910, seed: 1193, backbone: "yue-q6", instrumental: false },
    { id: "b72e-neon", title: "Neon Midnight Drive", kind: "song", duration: 214, created_at: iso(-0.5), favorite: false,
      description: "an upbeat 80s synth-pop anthem about driving through a neon city at midnight, female vocals",
      style: "80s synth-pop, 118 BPM, gated drums, analog arpeggios, punchy bass, airy female lead.", lyrics: "[Verse]\nCity lights are calling...", abc: "", lm_seed: 1, seed: 2, backbone: "yue-q6", instrumental: false },
    { id: "c913-grandma", title: "Grandma's Kitchen", kind: "song", duration: 188, created_at: iso(25), favorite: true,
      description: "a warm acoustic folk ballad about my grandmother's kitchen on Sunday mornings", style: "acoustic folk", lyrics: "[Verse]\n...", abc: "", lm_seed: 3, seed: 4, backbone: "yue-q6", instrumental: false },
    { id: "d40a-monsoon", title: "Monsoon Reunion", kind: "cover", duration: 241, created_at: iso(60), favorite: false,
      description: "a Bollywood style romantic duet in the monsoon rain", style: "bollywood duet", lyrics: "[Verse]\n...", abc: "", lm_seed: 5, seed: 6, backbone: "yue-q8", instrumental: false },
    { id: "e55b-finish", title: "Finish Line Anthem", kind: "song", duration: 202, created_at: iso(180), favorite: false,
      description: "epic orchestral rock song for a marathon finish line", style: "orchestral rock", lyrics: "[Verse]\n...", abc: "", lm_seed: 7, seed: 8, backbone: "yue-q6", instrumental: false },
    { id: "f12c-lofi", title: "Late Library Lights", kind: "variation", duration: 156, created_at: iso(400), favorite: false,
      description: "lo-fi hip hop study beat with soft piano and vinyl crackle", style: "lofi", lyrics: "", abc: "", lm_seed: 9, seed: 10, backbone: "yue-q5", instrumental: true },
  ];
  const jobs = scene === "create" ? [{
    id: "job-1", kind: "song", title: "Sunday Pancake Song", description: "", state: "running",
    stage: "Composing & singing", detail: "semantic stage: 1840/4375 frames · 41 tok/s", started_at: now - 83000, error: null, track_ids: [],
  }] : [];

  const gb = (n) => Math.round(n * 1024 ** 3);
  const entry = (id, kind, label, bytes, ram_gb, note, installed, extra = {}) => ({
    id, kind, label, filename: id + ".gguf", url: "", bytes, ram_gb, note, license: "", installed, downloading: false, partial: 0, selected: false, ...extra,
  });
  const catalog = {
    models_dir: "~/Library/Application Support/app.cadenza.studio/models",
    free_bytes: gb(212),
    progress: [{ id: "gemma-12b-q4", state: "running", downloaded: gb(4.1), total: gb(6.5), bytes_per_sec: 38 * 1024 ** 2, error: null }],
    entries: [
      entry("yue-q5", "backbone", "YuE2 3B · Q5_K_M (Light)", gb(2.44), 8, "Smallest set that still sings in tune.", false),
      entry("yue-q6", "backbone", "YuE2 3B · Q6_K (Balanced)", gb(2.74), 16, "Close to Q8 quality at three quarters of its size.", true),
      entry("yue-q8", "backbone", "YuE2 3B · Q8_0 (Quality)", gb(3.55), 24, "The set the engine authors test with.", false),
      entry("yue-bf16", "backbone", "YuE2 3B · BF16 (Full precision)", gb(6.67), 32, "The original weights, untouched.", false),
      entry("yue-vae", "vae", "YuE2 VAE · F32", gb(0.49), 0, "Turns latents into 48 kHz stereo.", true),
      entry("yue-companion", "companion", "Realaudio decoder companion v9", gb(0.13), 0, "Merged under every render for cleaner audio.", true),
      entry("llama-runtime", "llm_runtime", "llama.cpp runtime", 0, 0, "Serves the built-in language model.", true),
      entry("gemma-e4b-q4", "llm", "Gemma 4 E4B IT · QAT q4_0", gb(4.8), 16, "Light and multilingual.", true),
      entry("gemma-12b-q4", "llm", "Gemma 4 12B IT · QAT q4_0", gb(6.5), 24, "Noticeably better rhyme and imagery.", false, { downloading: true }),
      entry("gemma-12b-q5", "llm", "Gemma 4 12B IT · Q5_K_M", gb(7.84), 32, "More accurate than q4_0.", false),
      entry("gemma-12b-q8", "llm", "Gemma 4 12B IT · Q8_0", gb(11.8), 48, "As close to the original as a quantisation gets.", false),
      entry("sheetsage-q8", "transcriber", "SheetSage2 transcriber · Q8_0", gb(0.1), 0, "Listens to a recording and writes its melody as a score.", false),
      entry("mert-q8", "transcriber_base", "MERT-v2 audio encoder · Q8_0", gb(0.84), 0, "The encoder the transcriber reads.", false),
    ],
  };
  const settings = {
    models_dir: "", backbone: "yue-q6", llm_provider: "builtin", llm_model: "gemma-e4b-q4",
    ollama_url: "http://127.0.0.1:11434", ollama_model: "qwen3:32b", openai_url: "", openai_model: "", openai_key: "",
    unload_llm_before_music: true, keep_loaded: false, max_seq: null, use_companion: true, engine_path: "", output_format: "mp3", mp3_bitrate: 256,
  };
  const system = { ram_gb: 16, cpu: "Apple M1 Pro", os: "macOS 14.5", arch: "aarch64", cores: 10, recommended_backbone: "yue-q6", recommended_llm: "gemma-e4b-q4" };

  // A few seconds of shaped noise so the player draws a real waveform.
  function wav() {
    const rate = 8000, seconds = 12, n = rate * seconds;
    const buffer = new ArrayBuffer(44 + n * 2);
    const v = new DataView(buffer);
    const s = (o, t) => [...t].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)));
    s(0, "RIFF"); v.setUint32(4, 36 + n * 2, true); s(8, "WAVE"); s(12, "fmt ");
    v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true); v.setUint32(24, rate, true);
    v.setUint32(28, rate * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true); s(36, "data"); v.setUint32(40, n * 2, true);
    let seed = 7;
    for (let i = 0; i < n; i++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      const t = i / rate;
      const env = 0.25 + 0.55 * Math.abs(Math.sin(t * 0.9)) * (0.6 + 0.4 * Math.sin(t * 7.3)) * Math.min(1, t / 1.5);
      v.setInt16(44 + i * 2, ((seed / 0x7fffffff) * 2 - 1) * env * 30000, true);
    }
    return buffer;
  }

  const handlers = {
    get_settings: () => settings,
    system_info: () => system,
    catalog: () => catalog,
    engine_status: () => ({ state: "running", executable: "/Applications/Cadenza.app/Contents/Resources/yue2-cpp/yue-server", backbone: "yue-q6", missing: [], covers: false, message: null }),
    list_jobs: () => jobs,
    list_tracks: () => tracks,
    track_audio: () => wav(),
    engine_log: () => ["[Server] listening on 127.0.0.1:18087", "[Load] YuE2-3B-Q6_K.gguf on MTL0 (Apple M1 Pro)"],
    library_dir: () => "",
    storage_info: () => ({
      data_dir: "~/Library/Application Support/app.cadenza.studio",
      models_dir: "~/Library/Application Support/app.cadenza.studio/models",
      models_bytes: gb(8.16), models_count: 4, runtime_bytes: gb(0.04),
      library_dir: "~/Library/Application Support/app.cadenza.studio/library",
      library_bytes: gb(0.031), library_count: tracks.length, free_bytes: gb(212),
    }),
  };

  window.__TAURI__ = {
    core: { invoke: async (cmd) => (handlers[cmd] ? handlers[cmd]() : null) },
    event: { listen: async () => () => {} },
  };

  // Stage the scene once the app has booted.
  window.addEventListener("load", () => setTimeout(async () => {
    const click = (sel) => document.querySelector(sel)?.click();
    if (scene !== "create") click(`.nav button[data-view="${scene === "drawer" ? "library" : scene}"]`);
    if (scene === "create") {
      document.querySelector("#description").value = tracks[0].description;
      document.querySelector("#description").dispatchEvent(new Event("input"));
    }
    if (scene === "home") document.querySelector("#heroPrompt").value = "A Hindi lullaby for my 3-year-old son Babu, who loves painting and music";
    click(`[data-play="${tracks[0].id}"]`);
    if (scene === "drawer") setTimeout(() => click(`[data-open="${tracks[0].id}"]`), 300);
    if (scene === "settings") click('#providerSeg [data-provider="ollama"]');
    setTimeout(() => { document.body.dataset.ready = "1"; }, 1600);
  }, 300));
})();
