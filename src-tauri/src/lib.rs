//! Cadenza: prompt to full song, generated on your own machine.

mod audio;
mod catalog;
mod downloads;
mod engine;
mod jobs;
mod library;
mod llm;
mod settings;
mod songwriter;
mod styles;
mod system;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::catalog::Item;
use crate::settings::Settings;

pub struct AppState {
    pub data_dir: PathBuf,
    pub resource_dir: Option<PathBuf>,
    settings: Mutex<Settings>,
    pub engine: Mutex<engine::Engine>,
    pub llm: Mutex<llm::LlmRuntime>,
    pub library: Mutex<library::Library>,
    pub downloads: Arc<downloads::Downloads>,
    pub client: engine::Client,
    pub jobs: Mutex<Vec<jobs::Job>>,
    pub cancels: Mutex<std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>>,
    /// One generation at a time.
    pub pipeline: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn models_dir(&self) -> PathBuf {
        self.settings.lock().unwrap().models_dir(&self.data_dir)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.data_dir.join("runtime")
    }

    fn settings_path(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
}

type CommandResult<T> = Result<T, String>;

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

// ------------------------------------------------------------------ settings

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> CommandResult<Settings> {
    settings.save(&state.settings_path()).map_err(err)?;
    let previous = std::mem::replace(&mut *state.settings.lock().unwrap(), settings.clone());
    if previous.llm_model != settings.llm_model || previous.llm_provider != settings.llm_provider {
        state.llm.lock().unwrap().stop();
    }
    if previous.engine_fingerprint() != settings.engine_fingerprint() {
        state.engine.lock().unwrap().stop();
    }
    let _ = app.emit("engine", ());
    Ok(settings)
}

#[tauri::command]
fn system_info() -> system::SystemInfo {
    system::info()
}

// -------------------------------------------------------------------- models

#[derive(Serialize)]
struct CatalogEntry {
    #[serde(flatten)]
    item: Item,
    installed: bool,
    downloading: bool,
    partial: u64,
    selected: bool,
}

#[derive(Serialize)]
struct CatalogView {
    entries: Vec<CatalogEntry>,
    models_dir: String,
    free_bytes: Option<u64>,
    progress: Vec<downloads::Progress>,
}

fn free_space(path: &std::path::Path) -> Option<u64> {
    #[cfg(unix)]
    {
        let mut probe = path.to_path_buf();
        while !probe.exists() {
            probe = probe.parent()?.to_path_buf();
        }
        let c_path = std::ffi::CString::new(probe.to_string_lossy().as_bytes()).ok()?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
            return Some(stat.f_bavail as u64 * stat.f_frsize as u64);
        }
        None
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

#[tauri::command]
fn catalog(state: State<AppState>) -> CatalogView {
    let settings = state.settings();
    let models_dir = state.models_dir();
    let runtime_dir = state.runtime_dir();
    let entries = catalog::items()
        .into_iter()
        .map(|item| CatalogEntry {
            installed: downloads::installed(&item, &models_dir, &runtime_dir),
            downloading: state.downloads.is_active(item.id),
            partial: downloads::partial_bytes(&item, &models_dir, &runtime_dir),
            selected: item.id == settings.backbone || item.id == settings.llm_model,
            item,
        })
        .collect();
    CatalogView { entries, free_bytes: free_space(&models_dir), models_dir: models_dir.display().to_string(), progress: state.downloads.snapshot() }
}

#[tauri::command]
fn download_start(app: AppHandle, state: State<AppState>, id: String) -> CommandResult<Vec<String>> {
    let item = catalog::find(&id).ok_or("unknown model")?;
    // A model alone cannot make anything: bring along the files it works with,
    // so one click gets a working set. Returns every id that started.
    let mut ids = vec![id.clone()];
    match item.kind {
        catalog::Kind::Backbone => ids.extend(["yue-vae".to_string(), "yue-companion".to_string()]),
        catalog::Kind::Llm => ids.push("llama-runtime".into()),
        catalog::Kind::Transcriber => ids.extend(catalog::transcriber_pairs().iter().filter(|(t, _)| *t == id).map(|(_, base)| base.to_string())),
        catalog::Kind::Tokenizer => ids.extend(catalog::tokenizer_pairs().iter().filter(|(t, _)| *t == id).map(|(_, base)| base.to_string())),
        _ => {}
    }
    let (models_dir, runtime_dir) = (state.models_dir(), state.runtime_dir());
    let mut started = Vec::new();
    for want in ids {
        let Some(item) = catalog::find(&want) else { continue };
        if want == id || !downloads::installed(&item, &models_dir, &runtime_dir) {
            state.downloads.start(app.clone(), item, models_dir.clone(), runtime_dir.clone());
            started.push(want);
        }
    }
    Ok(started)
}

#[tauri::command]
fn download_cancel(state: State<AppState>, id: String) {
    state.downloads.cancel(&id);
}

#[tauri::command]
fn model_delete(app: AppHandle, state: State<AppState>, id: String) -> CommandResult<()> {
    let item = catalog::find(&id).ok_or("unknown model")?;
    let settings = state.settings();
    if id == settings.backbone || id == "yue-vae" || id.starts_with("sheetsage") || id.starts_with("mert") || id == "yue-companion" {
        state.engine.lock().unwrap().stop();
    }
    if id == settings.llm_model || id == "llama-runtime" {
        state.llm.lock().unwrap().stop();
    }
    downloads::delete(&item, &state.models_dir(), &state.runtime_dir()).map_err(err)?;
    let _ = app.emit("engine", ());
    Ok(())
}

// ------------------------------------------------------------------- storage

fn dir_size(path: &std::path::Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path).map(|entries| entries.flatten().map(|entry| dir_size(&entry.path())).sum()).unwrap_or(0)
}

#[derive(Serialize)]
struct StorageInfo {
    data_dir: String,
    models_dir: String,
    /// Only the files Cadenza downloaded, not whatever else shares the folder.
    models_bytes: u64,
    models_count: usize,
    runtime_bytes: u64,
    library_dir: String,
    library_bytes: u64,
    library_count: usize,
    free_bytes: Option<u64>,
}

#[tauri::command]
fn storage_info(state: State<AppState>) -> StorageInfo {
    let models_dir = state.models_dir();
    let mut models_bytes = 0;
    let mut models_count = 0;
    for item in catalog::items().iter().filter(|item| item.kind != catalog::Kind::LlmRuntime) {
        let path = models_dir.join(item.filename);
        let size = dir_size(&path) + dir_size(&path.with_file_name(format!("{}.part", item.filename)));
        if size > 0 {
            models_bytes += size;
            models_count += 1;
        }
    }
    let library = state.library.lock().unwrap();
    StorageInfo {
        data_dir: state.data_dir.display().to_string(),
        models_dir: models_dir.display().to_string(),
        models_bytes,
        models_count,
        runtime_bytes: dir_size(&state.runtime_dir()),
        library_dir: library.dir.display().to_string(),
        library_bytes: dir_size(&library.dir),
        library_count: library.tracks.len(),
        free_bytes: free_space(&models_dir),
    }
}

/// Deletes every model and the llama.cpp runtime Cadenza downloaded, by
/// catalog file name: a custom models folder may hold other files too.
#[tauri::command]
fn remove_all_models(app: AppHandle, state: State<AppState>) -> CommandResult<u64> {
    for item in catalog::items() {
        state.downloads.cancel(item.id);
    }
    state.engine.lock().unwrap().stop();
    state.llm.lock().unwrap().stop();
    let before = storage_info(state.clone());
    let (models_dir, runtime_dir) = (state.models_dir(), state.runtime_dir());
    for item in catalog::items() {
        downloads::delete(&item, &models_dir, &runtime_dir).map_err(err)?;
    }
    let after = storage_info(state);
    let _ = app.emit("engine", ());
    Ok((before.models_bytes + before.runtime_bytes).saturating_sub(after.models_bytes + after.runtime_bytes))
}

// -------------------------------------------------------------------- engine

#[tauri::command]
fn engine_status(state: State<AppState>) -> engine::EngineStatus {
    let settings = state.settings();
    let executable = engine::locate_executable(&settings, state.resource_dir.clone());
    state.engine.lock().unwrap().status(&settings, &state.models_dir(), executable)
}

#[tauri::command]
async fn engine_start(app: AppHandle) -> CommandResult<()> {
    jobs::ensure_engine(&app).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn engine_stop(app: AppHandle, state: State<AppState>) {
    state.engine.lock().unwrap().stop();
    state.llm.lock().unwrap().stop();
    let _ = app.emit("engine", ());
}

#[tauri::command]
fn engine_log(state: State<AppState>) -> Vec<String> {
    state.engine.lock().unwrap().log_tail(200)
}

#[tauri::command]
async fn llm_test(app: AppHandle) -> CommandResult<String> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    jobs::ensure_llm(&app, &settings).await.map_err(|e| format!("{e:#}"))?;
    llm::chat(&settings, "Answer in one short sentence.", "Say hello to a musician.", 0.5).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn ollama_models(url: String) -> CommandResult<Vec<String>> {
    llm::ollama_models(&url).await.map_err(|e| format!("could not reach Ollama: {e}"))
}

// ---------------------------------------------------------------------- jobs

#[tauri::command]
async fn create_song(app: AppHandle, request: jobs::SongRequest) -> CommandResult<jobs::Job> {
    if request.description.trim().is_empty() && request.lyrics.trim().is_empty() && request.parent_id.is_none() {
        return Err("Describe the song, or paste lyrics.".into());
    }
    if request.description.chars().count() > 1000 {
        return Err("Keep the description under 1000 characters.".into());
    }
    // Refuse what cannot work now, instead of queueing a job that fails.
    let (settings, resource_dir, models_dir) = {
        let state = app.state::<AppState>();
        (state.settings(), state.resource_dir.clone(), state.models_dir())
    };
    if engine::locate_executable(&settings, resource_dir).is_none() {
        return Err("The music engine is missing from this install. Reinstall Cadenza, or set the yue-server path in Settings.".into());
    }
    if let Err(missing) = engine::model_files(&settings, &models_dir) {
        return Err(format!("Still needed before making music: {}. Open Models and press “Download what's needed”.", missing.join(", ")));
    }
    if let Some(path) = &request.cover_audio {
        if !std::path::Path::new(path).is_file() {
            return Err("The recording is gone. Choose it again.".into());
        }
        let missing = engine::remake_missing(&models_dir, &request.remake);
        if !missing.is_empty() {
            let labels: Vec<_> = missing.iter().filter_map(|id| catalog::find(id)).map(|item| item.label).collect();
            return Err(format!("Remakes need the {}. Press “Download what's needed” below, or open Models.", labels.join(" and ")));
        }
    }
    if jobs::needs_writer(&request) && jobs::resolve_llm(&app, &settings).await.is_none() {
        return Err("Nothing can write the lyrics yet. Open Models and press “Download what's needed”, start Ollama, or paste your own lyrics.".into());
    }
    Ok(jobs::submit(app, request))
}

/// Name and length of a recording to remake, read before it is submitted.
#[tauri::command]
async fn audio_info(app: AppHandle, path: String) -> CommandResult<serde_json::Value> {
    let models_dir = app.state::<AppState>().models_dir();
    let info = tokio::task::spawn_blocking(move || audio::info(std::path::Path::new(&path))).await.map_err(err)?.map_err(err)?;
    Ok(serde_json::json!({
        "name": info.name,
        "duration": info.duration,
        "missing": {
            "cover": engine::remake_missing(&models_dir, "cover"),
            "continue": engine::remake_missing(&models_dir, "continue"),
        },
    }))
}

#[tauri::command]
fn audio_extensions() -> Vec<&'static str> {
    audio::EXTENSIONS.to_vec()
}

#[tauri::command]
fn style_presets() -> Vec<styles::Preset> {
    styles::PRESETS.to_vec()
}

#[derive(Serialize)]
struct LlmStatus {
    ready: bool,
    /// "builtin", "ollama" or "open_ai"
    provider: Option<settings::LlmProvider>,
    model: String,
    /// Ollama stands in for the built-in model, which is not downloaded.
    fallback: bool,
}

#[tauri::command]
async fn llm_status(app: AppHandle) -> LlmStatus {
    let settings = app.state::<AppState>().settings();
    match jobs::resolve_llm(&app, &settings).await {
        Some(resolved) => {
            let model = match resolved.llm_provider {
                settings::LlmProvider::Builtin => catalog::find(&resolved.llm_model).map(|item| item.label.to_string()).unwrap_or_default(),
                settings::LlmProvider::Ollama => resolved.ollama_model.clone(),
                settings::LlmProvider::OpenAi => resolved.openai_model.clone(),
            };
            LlmStatus { ready: true, fallback: resolved.llm_provider != settings.llm_provider, provider: Some(resolved.llm_provider), model }
        }
        None => LlmStatus { ready: false, provider: None, model: String::new(), fallback: false },
    }
}

/// Starts every download a first song needs: the chosen YuE2 size, the VAE,
/// the companion and, unless another language model is ready, the llama.cpp
/// runtime with the chosen lyric model.
/// With `remake` ("cover" or "continue"), also the models that remake needs.
#[tauri::command]
async fn download_required(app: AppHandle, remake: Option<String>) -> CommandResult<Vec<String>> {
    let settings = app.state::<AppState>().settings();
    let mut ids = vec![settings.backbone.clone(), "yue-vae".into(), "yue-companion".into()];
    if let Some(mode) = &remake {
        ids.extend(engine::remake_missing(&app.state::<AppState>().models_dir(), mode).into_iter().map(String::from));
    }
    if jobs::resolve_llm(&app, &settings).await.is_none() {
        ids.push("llama-runtime".into());
        ids.push(settings.llm_model.clone());
    }
    let state = app.state::<AppState>();
    let (models_dir, runtime_dir) = (state.models_dir(), state.runtime_dir());
    let mut started = Vec::new();
    for id in ids {
        let Some(item) = catalog::find(&id) else { continue };
        if !downloads::installed(&item, &models_dir, &runtime_dir) {
            state.downloads.start(app.clone(), item, models_dir.clone(), runtime_dir.clone());
            started.push(id);
        }
    }
    Ok(started)
}

#[tauri::command]
async fn write_lyrics(app: AppHandle, request: jobs::SongRequest) -> CommandResult<songwriter::Written> {
    if request.description.trim().is_empty() {
        return Err("describe the song first".into());
    }
    jobs::write_only(&app, request).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn cancel_job(app: AppHandle, id: String) -> Option<jobs::Job> {
    jobs::cancel(&app, &id)
}

#[tauri::command]
fn list_jobs(state: State<AppState>) -> Vec<jobs::Job> {
    state.jobs.lock().unwrap().clone()
}

#[tauri::command]
fn dismiss_job(state: State<AppState>, id: String) {
    state.jobs.lock().unwrap().retain(|job| job.id != id || matches!(job.state.as_str(), "queued" | "running"));
}

// ------------------------------------------------------------------- library

#[tauri::command]
fn list_tracks(state: State<AppState>) -> Vec<library::Track> {
    state.library.lock().unwrap().tracks.iter().map(|track| library::Track { replay: serde_json::Value::Null, ..track.clone() }).collect()
}

#[tauri::command]
fn track_audio(state: State<AppState>, id: String) -> CommandResult<tauri::ipc::Response> {
    let library = state.library.lock().unwrap();
    let track = library.get(&id).ok_or("no such track")?;
    let bytes = std::fs::read(library.path_of(track)).map_err(err)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
fn update_track(state: State<AppState>, id: String, title: Option<String>, favorite: Option<bool>) -> CommandResult<library::Track> {
    state.library.lock().unwrap().update(&id, title, favorite).map(|t| library::Track { replay: serde_json::Value::Null, ..t }).map_err(err)
}

#[tauri::command]
fn delete_track(app: AppHandle, state: State<AppState>, id: String) -> CommandResult<()> {
    state.library.lock().unwrap().remove(&id).map_err(err)?;
    let _ = app.emit("library", ());
    Ok(())
}

#[tauri::command]
fn export_track(state: State<AppState>, id: String, destination: String) -> CommandResult<()> {
    let library = state.library.lock().unwrap();
    let track = library.get(&id).ok_or("no such track")?;
    library::export(&library.path_of(track), std::path::Path::new(&destination)).map_err(err)
}

#[tauri::command]
fn track_file_name(state: State<AppState>, id: String) -> CommandResult<String> {
    let library = state.library.lock().unwrap();
    let track = library.get(&id).ok_or("no such track")?;
    Ok(format!("{}.{}", library::safe_name(&track.title), library::extension(&track.format)))
}

#[tauri::command]
fn library_dir(state: State<AppState>) -> String {
    state.library.lock().unwrap().dir.display().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = Settings::load(&data_dir.join("settings.json"));
            let library = library::Library::open(data_dir.join("library"));
            app.manage(AppState {
                resource_dir: app.path().resource_dir().ok(),
                settings: Mutex::new(settings),
                engine: Mutex::new(engine::Engine::default()),
                llm: Mutex::new(llm::LlmRuntime::default()),
                library: Mutex::new(library),
                downloads: Arc::new(downloads::Downloads::default()),
                client: engine::Client::default(),
                jobs: Mutex::new(Vec::new()),
                cancels: Mutex::new(Default::default()),
                pipeline: tokio::sync::Mutex::new(()),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings, save_settings, system_info,
            catalog, download_start, download_cancel, model_delete, storage_info, remove_all_models,
            engine_status, engine_start, engine_stop, engine_log, llm_test, ollama_models,
            create_song, write_lyrics, llm_status, download_required, audio_info, audio_extensions, style_presets, cancel_job, list_jobs, dismiss_job,
            list_tracks, track_audio, update_track, delete_track, export_track, track_file_name, library_dir,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cadenza")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                state.engine.lock().unwrap().stop();
                state.llm.lock().unwrap().stop();
            }
        });
}
