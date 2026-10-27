//! The generation pipeline: write → start engine → compose & sing → save.
//!
//! Jobs run one at a time (the engine has one worker and the language model
//! and the engine share memory); later jobs wait as "queued". Every change
//! of a job is sent to the UI as a `job` event.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::library::{self, Track};
use crate::settings::{LlmProvider, Settings};
use crate::songwriter::{self, Brief, Written};
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    /// queued, running, done, error, cancelled
    pub state: String,
    pub stage: String,
    pub detail: String,
    pub started_at: i64,
    pub error: Option<String>,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SongRequest {
    pub description: String,
    #[serde(default)]
    pub lyrics: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub instrumental: bool,
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_duration")]
    pub duration: f32,
    #[serde(default = "default_cot")]
    pub cot: String,
    #[serde(default = "default_steps")]
    pub steps: u32,
    #[serde(default)]
    pub lm_seed: Option<i64>,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default = "one")]
    pub variations: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Covers: the recording whose melody is sung.
    #[serde(default)]
    pub cover_audio: Option<String>,
    #[serde(default)]
    pub melody_only: bool,
    /// Variations: the track whose composition is re-rendered.
    #[serde(default)]
    pub parent_id: Option<String>,
}

fn default_duration() -> f32 { 180.0 }
fn default_cot() -> String { "full".into() }
fn default_steps() -> u32 { 32 }
fn one() -> u32 { 1 }

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn random_seed() -> i64 {
    (uuid::Uuid::new_v4().as_u128() as u64 >> 33) as i64
}

fn update(app: &AppHandle, id: &str, change: impl FnOnce(&mut Job)) {
    let state = app.state::<AppState>();
    let mut jobs = state.jobs.lock().unwrap();
    if let Some(job) = jobs.iter_mut().find(|job| job.id == id) {
        change(job);
        let _ = app.emit("job", job.clone());
    }
}

fn stage(app: &AppHandle, id: &str, stage: &str, detail: &str) {
    update(app, id, |job| {
        job.state = "running".into();
        job.stage = stage.into();
        job.detail = detail.into();
    });
}

pub fn submit(app: AppHandle, request: SongRequest) -> Job {
    let kind = if request.parent_id.is_some() {
        "variation"
    } else if request.cover_audio.is_some() {
        "cover"
    } else {
        "song"
    };
    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        kind: kind.into(),
        title: if request.title.trim().is_empty() { songwriter::fallback_title(&request.description) } else { request.title.trim().into() },
        description: request.description.clone(),
        state: "queued".into(),
        stage: "Waiting for the previous song".into(),
        detail: String::new(),
        started_at: now_ms(),
        error: None,
        track_ids: Vec::new(),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<AppState>();
        state.jobs.lock().unwrap().insert(0, job.clone());
        state.cancels.lock().unwrap().insert(job.id.clone(), cancel.clone());
    }
    let _ = app.emit("job", job.clone());
    let id = job.id.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _turn = state.pipeline.lock().await;
        if cancel.load(Ordering::SeqCst) {
            update(&app, &id, |job| job.state = "cancelled".into());
            return;
        }
        update(&app, &id, |job| job.started_at = now_ms());
        let result = run(&app, &id, request, &cancel).await;
        match result {
            Ok(track_ids) => update(&app, &id, |job| {
                job.state = "done".into();
                job.stage = "Done".into();
                job.detail.clear();
                job.track_ids = track_ids;
            }),
            Err(_) if cancel.load(Ordering::SeqCst) => update(&app, &id, |job| {
                job.state = "cancelled".into();
                job.stage = "Cancelled".into();
            }),
            Err(error) => update(&app, &id, |job| {
                job.state = "error".into();
                job.stage = "Failed".into();
                job.error = Some(format!("{error:#}"));
            }),
        }
        state.cancels.lock().unwrap().remove(&id);
    });
    job
}

/// Flags the job and answers with its state as it now stands, so the UI never
/// keeps a stale card: a queued job is cancelled on the spot, a running one
/// stops at its next poll, a finished one is returned as it is.
pub fn cancel(app: &AppHandle, id: &str) -> Option<Job> {
    let state = app.state::<AppState>();
    let flag = state.cancels.lock().unwrap().get(id).cloned();
    if let Some(flag) = flag {
        flag.store(true, Ordering::SeqCst);
    }
    update(app, id, |job| match job.state.as_str() {
        "queued" => {
            job.state = "cancelled".into();
            job.stage = "Cancelled".into();
        }
        "running" => job.detail = "Cancelling…".into(),
        _ => {}
    });
    let job = state.jobs.lock().unwrap().iter().find(|job| job.id == id).cloned();
    job
}

fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        bail!("cancelled");
    }
    Ok(())
}

/// Runs a step that has no cancel point of its own (a model loading, a
/// language model writing) and abandons it as soon as the job is cancelled.
async fn cancellable<T>(cancel: &AtomicBool, step: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    let watch = async {
        while !cancel.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };
    tokio::select! {
        result = step => result,
        _ = watch => bail!("cancelled"),
    }
}

/// The language-model settings a job can use right now: the configured
/// provider, or Ollama when the built-in model is not downloaded but Ollama
/// is running with a model pulled - nothing to download in that case.
pub async fn resolve_llm(app: &AppHandle, settings: &Settings) -> Option<Settings> {
    match settings.llm_provider {
        LlmProvider::Builtin => {
            let state = app.state::<AppState>();
            if crate::llm::builtin_model_path(settings, &state.models_dir()).is_ok() && crate::llm::find_llama_server(&state.runtime_dir()).is_some() {
                return Some(settings.clone());
            }
            let models = crate::llm::ollama_models(&settings.ollama_url).await.ok()?;
            let model = models.iter().find(|name| **name == settings.ollama_model).or(models.first())?.clone();
            Some(Settings { llm_provider: LlmProvider::Ollama, ollama_model: model, ..settings.clone() })
        }
        LlmProvider::Ollama => (!settings.ollama_model.trim().is_empty()).then(|| settings.clone()),
        LlmProvider::OpenAi => (!settings.openai_url.trim().is_empty()).then(|| settings.clone()),
    }
}

/// Lyrics are written by a model unless the request brings them.
pub fn needs_writer(request: &SongRequest) -> bool {
    request.parent_id.is_none() && !request.instrumental && request.lyrics.trim().is_empty()
}

pub async fn ensure_llm(app: &AppHandle, settings: &Settings) -> Result<()> {
    if settings.llm_provider != LlmProvider::Builtin {
        return Ok(());
    }
    let state = app.state::<AppState>();
    let model = crate::llm::builtin_model_path(settings, &state.models_dir())?;
    let wait = state.llm.lock().unwrap().spawn(&state.runtime_dir(), &model)?;
    if wait {
        crate::llm::wait_ready(Duration::from_secs(300)).await?;
    }
    Ok(())
}

pub async fn ensure_engine(app: &AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let executable = crate::engine::locate_executable(&settings, state.resource_dir.clone())
        .context("the yue-server engine is not built yet: run scripts/build-engine.sh, or set its path in Settings")?;
    let wait = state.engine.lock().unwrap().spawn(app, &settings, &state.models_dir(), &executable)?;
    let _ = app.emit("engine", ());
    if wait {
        let result = state.client.wait_healthy(Duration::from_secs(600), || app.state::<AppState>().engine.lock().unwrap().running()).await;
        let mut engine = state.engine.lock().unwrap();
        match &result {
            Ok(()) => engine.mark_ready(),
            Err(error) => {
                let tail = engine.log_tail(6).join("\n");
                engine.last_error = Some(format!("{error}\n{tail}"));
                engine.stop();
            }
        }
        drop(engine);
        let _ = app.emit("engine", ());
        result?;
    }
    Ok(())
}

async fn run(app: &AppHandle, id: &str, request: SongRequest, cancel: &AtomicBool) -> Result<Vec<String>> {
    let state = app.state::<AppState>();
    let settings = state.settings();

    // A variation re-renders a stored composition with fresh noise.
    if let Some(parent_id) = &request.parent_id {
        let parent = state.library.lock().unwrap().get(parent_id).cloned().context("the original track is gone")?;
        if parent.replay.is_null() {
            bail!("this track has no stored composition to re-render");
        }
        let mut body = parent.replay.clone();
        body["seed"] = json!(request.seed.unwrap_or_else(random_seed));
        body["steps"] = json!(request.steps.max(1));
        body["synth_batch_size"] = json!(request.variations.clamp(1, 4));
        body["lm_batch_size"] = json!(1);
        body["output_format"] = json!(settings.output_format);
        body["mp3_bitrate"] = json!(settings.mp3_bitrate);
        let written = Written { title: parent.title.clone(), style: parent.style.clone(), lyrics: parent.lyrics.clone() };
        return sing(app, id, &settings, body, &written, &request, "variation", Some(parent.id.clone()), cancel).await;
    }

    // 1. Write what YuE2 reads.
    let has_lyrics = !request.lyrics.trim().is_empty() || request.instrumental;
    let has_style = !request.style.trim().is_empty();
    let written = if has_lyrics && has_style {
        Written {
            title: if request.title.trim().is_empty() { songwriter::fallback_title(&request.description) } else { request.title.trim().into() },
            style: request.style.trim().into(),
            lyrics: if request.instrumental { songwriter::INSTRUMENTAL_LYRICS.into() } else { songwriter::tidy_lyrics(&request.lyrics) },
        }
    } else if let Some(llm_settings) = resolve_llm(app, &settings).await {
        stage(app, id, "Writing lyrics", "Loading the language model");
        cancellable(cancel, ensure_llm(app, &llm_settings)).await?;
        stage(app, id, "Writing lyrics", "Choosing words, title and sound");
        let brief = Brief {
            description: &request.description,
            lyrics: &request.lyrics,
            title: &request.title,
            language: &request.language,
            instrumental: request.instrumental,
            duration: request.duration,
        };
        let mut written = cancellable(cancel, songwriter::write(&llm_settings, &brief)).await?;
        if has_style {
            written.style = request.style.trim().into();
        }
        written
    } else if has_lyrics {
        // No language model: the description itself is a fine style caption.
        Written {
            title: if request.title.trim().is_empty() { songwriter::fallback_title(&request.description) } else { request.title.trim().into() },
            style: if has_style { request.style.trim().into() } else { request.description.trim().into() },
            lyrics: if request.instrumental { songwriter::INSTRUMENTAL_LYRICS.into() } else { songwriter::tidy_lyrics(&request.lyrics) },
        }
    } else {
        bail!("to write lyrics from a description, download a language model (Models) or connect Ollama (Settings), or paste your own lyrics");
    };
    check(cancel)?;
    update(app, id, |job| job.title = written.title.clone());

    if settings.unload_llm_before_music {
        state.llm.lock().unwrap().stop();
    }

    // 2. Engine.
    stage(app, id, "Starting the music engine", "Loading YuE2 into memory");
    cancellable(cancel, ensure_engine(app)).await?;

    // 3. Covers: hear the melody first.
    let mut abc = String::new();
    if let Some(path) = &request.cover_audio {
        let covers = state.engine.lock().unwrap().status(&settings, &state.models_dir(), None).covers;
        if !covers {
            bail!("covers need the SheetSage2 transcriber and the MERT encoder (Models → Covers)");
        }
        stage(app, id, "Listening to the recording", "Transcribing its melody");
        let audio = tokio::fs::read(path).await.with_context(|| format!("could not read {path}"))?;
        let filename = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "song.mp3".into());
        let job = state.client.transcribe(audio, filename, request.melody_only).await?;
        if !state.client.wait(&job, || cancel.load(Ordering::SeqCst)).await? {
            bail!("cancelled");
        }
        abc = state.client.score(&job).await?;
    }

    let mut body = json!({
        "style": written.style,
        "lyrics": written.lyrics,
        "cot": if request.cover_audio.is_some() { if request.melody_only { "melody" } else { "full" } } else { request.cot.as_str() },
        "duration": request.duration.clamp(10.0, 600.0),
        "steps": request.steps.clamp(1, 200),
        "lm_seed": request.lm_seed.unwrap_or_else(random_seed),
        "seed": request.seed.unwrap_or_else(random_seed),
        "lm_batch_size": 1,
        "synth_batch_size": request.variations.clamp(1, 4),
        "output_format": settings.output_format,
        "mp3_bitrate": settings.mp3_bitrate,
    });
    if !abc.is_empty() {
        body["abc"] = json!(abc);
    }
    if let Some(temperature) = request.temperature {
        body["semantic_sampling"] = json!({ "temperature": temperature.clamp(0.1, 2.0) });
    }
    let kind = if request.cover_audio.is_some() { "cover" } else { "song" };
    sing(app, id, &settings, body, &written, &request, kind, None, cancel).await
}

#[allow(clippy::too_many_arguments)]
async fn sing(app: &AppHandle, id: &str, settings: &Settings, body: Value, written: &Written, request: &SongRequest, kind: &str, parent_id: Option<String>, cancel: &AtomicBool) -> Result<Vec<String>> {
    let state = app.state::<AppState>();
    if parent_id.is_some() {
        stage(app, id, "Starting the music engine", "Loading YuE2 into memory");
        cancellable(cancel, ensure_engine(app)).await?;
    }
    stage(app, id, if parent_id.is_some() { "Re-rendering" } else { "Composing & singing" }, "Writing the score, then the audio codes");
    let job = state.client.submit(&body).await?;
    if !state.client.wait(&job, || cancel.load(Ordering::SeqCst)).await? {
        bail!("cancelled");
    }
    stage(app, id, "Saving", "");
    let tracks = state.client.tracks(&job).await?;
    let count = tracks.len();
    let mut ids = Vec::new();
    for (index, track) in tracks.into_iter().enumerate() {
        let track_id = uuid::Uuid::new_v4().to_string();
        let format = if track.content_type.contains("wav") { "wav".to_string() } else { "mp3".to_string() };
        let title = if count > 1 { format!("{} ({})", written.title, index + 1) } else { written.title.clone() };
        let file = format!("{}-{}.{}", &track_id[..8], library::safe_name(&title), library::extension(&format));
        let mut library = state.library.lock().unwrap();
        std::fs::write(library.dir.join(&file), &track.audio)?;
        let duration = library::duration_of(&track.audio, &format, settings.mp3_bitrate);
        let replay = track.replay;
        library.add(Track {
            id: track_id.clone(),
            title,
            kind: kind.into(),
            description: request.description.clone(),
            style: written.style.clone(),
            lyrics: written.lyrics.clone(),
            abc: replay.get("abc").and_then(Value::as_str).unwrap_or_default().to_string(),
            instrumental: request.instrumental,
            created_at: chrono::Local::now().to_rfc3339(),
            file,
            format,
            duration,
            favorite: false,
            lm_seed: replay.get("lm_seed").and_then(Value::as_i64),
            seed: replay.get("seed").and_then(Value::as_i64),
            parent_id: parent_id.clone(),
            backbone: settings.backbone.clone(),
            replay,
        })?;
        let _ = app.emit("library", ());
        ids.push(track_id);
    }
    Ok(ids)
}

/// Lyrics (and a style) for the form, without making the song.
pub async fn write_only(app: &AppHandle, request: SongRequest) -> Result<Written> {
    let state = app.state::<AppState>();
    let Some(settings) = resolve_llm(app, &state.settings()).await else {
        bail!("download a language model (Models) or start Ollama first");
    };
    let _turn = state.pipeline.lock().await;
    ensure_llm(app, &settings).await?;
    let brief = Brief {
        description: &request.description,
        lyrics: "",
        title: &request.title,
        language: &request.language,
        instrumental: false,
        duration: request.duration,
    };
    songwriter::write(&settings, &brief).await
}
