//! The yue2.cpp `yue-server` process and its HTTP API.
//!
//! The server takes its models on the command line, so a different
//! backbone means a restart. One worker thread inside it runs jobs in FIFO
//! order; we submit with `POST /synth`, poll `GET /job` and collect the
//! multipart result.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::catalog;
use crate::settings::Settings;

pub const PORT: u16 = 18087;
const EXECUTABLE: &str = if cfg!(windows) { "yue-server.exe" } else { "yue-server" };

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub state: &'static str,
    pub executable: Option<String>,
    pub backbone: String,
    pub missing: Vec<String>,
    /// "Same melody" remakes: the transcriber pair is installed.
    pub covers: bool,
    /// "Continue from a section" remakes: the tokenizer pair is installed.
    pub remix: bool,
    pub message: Option<String>,
}

pub struct Engine {
    child: Option<Child>,
    fingerprint: String,
    starting: bool,
    pub last_error: Option<String>,
    pub log: Arc<Mutex<VecDeque<String>>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self { child: None, fingerprint: String::new(), starting: false, last_error: None, log: Arc::new(Mutex::new(VecDeque::new())) }
    }
}

/// The model files one engine start needs, or the names of those missing.
pub struct ModelFiles {
    pub backbone: PathBuf,
    pub vae: PathBuf,
    pub companion: Option<PathBuf>,
    pub transcriber: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
}

/// The first installed head of these pairs whose MERT encoder sits beside it.
fn installed_head(models_dir: &Path, pairs: [(&str, &str); 2]) -> Option<PathBuf> {
    pairs.into_iter().find_map(|(head, base)| {
        let head = catalog::find(head)?;
        let base = catalog::find(base)?;
        let head_path = models_dir.join(head.filename);
        (head_path.is_file() && models_dir.join(base.filename).is_file()).then_some(head_path)
    })
}

/// Ids still to download before a remake in this mode ("cover" or
/// "continue") can run; empty when one pair is complete.
pub fn remake_missing(models_dir: &Path, mode: &str) -> Vec<&'static str> {
    let pairs = catalog::remake_pairs(mode);
    if installed_head(models_dir, pairs).is_some() {
        return Vec::new();
    }
    let (head, base) = pairs[0];
    [head, base]
        .into_iter()
        .filter(|id| catalog::find(id).is_some_and(|item| !models_dir.join(item.filename).is_file()))
        .collect()
}

pub fn model_files(settings: &Settings, models_dir: &Path) -> std::result::Result<ModelFiles, Vec<String>> {
    let mut missing = Vec::new();
    let mut need = |id: &str| -> Option<PathBuf> {
        let item = catalog::find(id)?;
        let path = models_dir.join(item.filename);
        if std::fs::metadata(&path).map(|meta| meta.len() == item.bytes).unwrap_or(false) {
            Some(path)
        } else {
            missing.push(item.label.to_string());
            None
        }
    };
    let backbone = need(&settings.backbone);
    let vae = need("yue-vae");
    let companion = if settings.use_companion {
        catalog::find("yue-companion").map(|item| models_dir.join(item.filename)).filter(|path| path.is_file())
    } else {
        None
    };
    // Both heads read the MERT encoder from their own folder.
    let transcriber = installed_head(models_dir, catalog::transcriber_pairs());
    let tokenizer = installed_head(models_dir, catalog::tokenizer_pairs());
    match (backbone, vae) {
        (Some(backbone), Some(vae)) if missing.is_empty() => Ok(ModelFiles { backbone, vae, companion, transcriber, tokenizer }),
        _ => Err(missing),
    }
}

/// The `yue-server` to run: the one in Settings, the bundled one, or a
/// developer build next to the source tree.
pub fn locate_executable(settings: &Settings, resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    if !settings.engine_path.trim().is_empty() {
        let path = PathBuf::from(settings.engine_path.trim());
        return path.is_file().then_some(path);
    }
    let mut candidates = Vec::new();
    if let Some(resources) = resource_dir {
        candidates.push(resources.join("yue2-cpp").join(EXECUTABLE));
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("resources/yue2-cpp").join(EXECUTABLE));
    candidates.push(manifest.join("../engine-build/yue2.cpp/build").join(EXECUTABLE));
    candidates.push(manifest.join("../engine-build/yue2.cpp/build/Release").join(EXECUTABLE));
    candidates.into_iter().find(|path| path.is_file())
}

impl Engine {
    pub fn running(&mut self) -> bool {
        match self.child.as_mut().map(|child| child.try_wait()) {
            Some(Ok(None)) => true,
            Some(_) => {
                self.child = None;
                false
            }
            None => false,
        }
    }

    pub fn status(&mut self, settings: &Settings, models_dir: &Path, executable: Option<PathBuf>) -> EngineStatus {
        let files = model_files(settings, models_dir);
        let running = self.running();
        let state = if running && self.starting {
            "starting"
        } else if running {
            "running"
        } else if executable.is_none() {
            "no_engine"
        } else if files.is_err() {
            "no_models"
        } else {
            "stopped"
        };
        let (missing, covers, remix) = match &files {
            Ok(files) => (Vec::new(), files.transcriber.is_some(), files.tokenizer.is_some()),
            Err(missing) => (missing.clone(), false, false),
        };
        EngineStatus {
            state,
            executable: executable.map(|path| path.display().to_string()),
            backbone: settings.backbone.clone(),
            missing,
            covers,
            remix,
            message: self.last_error.clone(),
        }
    }

    /// Launches the process if it is not already running with these settings.
    /// Returns true when the caller must wait for it to become healthy.
    pub fn spawn(&mut self, app: &AppHandle, settings: &Settings, models_dir: &Path, executable: &Path) -> Result<bool> {
        // Optional heads are flags at start, so a newly downloaded one restarts it.
        let optional = model_files(settings, models_dir).map(|files| format!("{:?}|{:?}", files.transcriber, files.tokenizer)).unwrap_or_default();
        let fingerprint = format!("{}|{optional}", settings.engine_fingerprint());
        if self.running() {
            if self.fingerprint == fingerprint {
                return Ok(self.starting);
            }
            self.stop();
        }
        let files = model_files(settings, models_dir).map_err(|missing| anyhow!("download these models first: {}", missing.join(", ")))?;
        let mut command = Command::new(executable);
        command
            .arg("--model").arg(&files.backbone)
            .arg("--vae").arg(&files.vae)
            .args(["--host", "127.0.0.1", "--port", &PORT.to_string()]);
        if let Some(companion) = &files.companion {
            command.arg("--companion").arg(companion);
        }
        if let Some(transcriber) = &files.transcriber {
            command.arg("--transcriber").arg(transcriber);
        }
        if let Some(tokenizer) = &files.tokenizer {
            command.arg("--tokenizer").arg(tokenizer);
        }
        if settings.keep_loaded {
            command.arg("--keep-loaded");
        }
        if let Some(max_seq) = settings.max_seq {
            command.arg("--max-seq").arg(max_seq.to_string());
        }
        if let Some(dir) = executable.parent() {
            command.current_dir(dir);
        }
        command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        hide_window(&mut command);
        let mut child = command.spawn().with_context(|| format!("could not start {}", executable.display()))?;
        self.log.lock().unwrap().clear();
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
            let log = self.log.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let line = line.trim_end().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    {
                        let mut log = log.lock().unwrap();
                        if log.len() >= 400 {
                            log.pop_front();
                        }
                        log.push_back(line.clone());
                    }
                    let _ = app.emit("engine-log", line);
                }
            });
        }
        self.child = Some(child);
        self.fingerprint = fingerprint;
        self.starting = true;
        self.last_error = None;
        Ok(true)
    }

    pub fn mark_ready(&mut self) {
        self.starting = false;
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            terminate(&mut child);
        }
        self.starting = false;
        self.fingerprint.clear();
    }

    pub fn log_tail(&self, lines: usize) -> Vec<String> {
        let log = self.log.lock().unwrap();
        log.iter().skip(log.len().saturating_sub(lines)).cloned().collect()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A console program started from a GUI app on Windows opens a console
/// window of its own unless told not to.
pub fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// SIGTERM lets the server cancel its job and free the GPU; kill after a grace period.
pub fn terminate(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
}

// ---------------------------------------------------------------- HTTP client

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
}

#[derive(Debug, Deserialize)]
struct Submitted {
    id: String,
}

#[derive(Debug, Deserialize)]
struct JobState {
    status: String,
}

/// One generated track from a job result.
pub struct Track {
    pub replay: Value,
    pub audio: Vec<u8>,
    pub content_type: String,
}

impl Default for Client {
    fn default() -> Self {
        // The server drops idle connections after a few seconds; no pooling.
        let http = reqwest::Client::builder().pool_max_idle_per_host(0).build().expect("http client");
        Self { http, base: format!("http://127.0.0.1:{PORT}") }
    }
}

impl Client {
    pub async fn healthy(&self) -> bool {
        self.http.get(format!("{}/health", self.base)).timeout(Duration::from_millis(800)).send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub async fn wait_healthy(&self, timeout: Duration, mut alive: impl FnMut() -> bool) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.healthy().await {
                return Ok(());
            }
            if !alive() {
                bail!("the engine exited while starting");
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        bail!("the engine did not answer within {} s", timeout.as_secs())
    }

    async fn json<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            let message = serde_json::from_str::<Value>(&body).ok().and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned)).unwrap_or(body);
            bail!("engine: {status}: {message}");
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn submit(&self, request: &Value) -> Result<String> {
        let response = self.http.post(format!("{}/synth", self.base)).json(request).send().await?;
        Ok(Self::json::<Submitted>(response).await?.id)
    }

    pub async fn transcribe(&self, audio: Vec<u8>, filename: String, melody_only: bool) -> Result<String> {
        let mut form = reqwest::multipart::Form::new().part("audio", reqwest::multipart::Part::bytes(audio).file_name(filename));
        if melody_only {
            form = form.text("melody_only", "1");
        }
        let response = self.http.post(format!("{}/transcribe", self.base)).multipart(form).send().await?;
        Ok(Self::json::<Submitted>(response).await?.id)
    }

    pub async fn tokenize(&self, audio: Vec<u8>, filename: String) -> Result<String> {
        let form = reqwest::multipart::Form::new().part("audio", reqwest::multipart::Part::bytes(audio).file_name(filename));
        let response = self.http.post(format!("{}/tokenize", self.base)).multipart(form).send().await?;
        Ok(Self::json::<Submitted>(response).await?.id)
    }

    pub async fn status(&self, id: &str) -> Result<String> {
        let response = self.http.get(format!("{}/job", self.base)).query(&[("id", id)]).send().await?;
        Ok(Self::json::<JobState>(response).await?.status)
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        self.http.post(format!("{}/job", self.base)).query(&[("id", id), ("cancel", "1")]).send().await?;
        Ok(())
    }

    /// Waits for the job to end; `Ok(false)` when it was cancelled.
    pub async fn wait(&self, id: &str, mut cancelled: impl FnMut() -> bool) -> Result<bool> {
        let mut misses = 0;
        loop {
            if cancelled() {
                let _ = self.cancel(id).await;
                return Ok(false);
            }
            match self.status(id).await {
                Ok(status) => {
                    misses = 0;
                    match status.as_str() {
                        "done" => return Ok(true),
                        "failed" => bail!("the engine failed this song (see the engine log)"),
                        "cancelled" => return Ok(false),
                        _ => {}
                    }
                }
                Err(error) => {
                    misses += 1;
                    if misses > 10 {
                        return Err(error);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(700)).await;
        }
    }

    async fn result_raw(&self, id: &str) -> Result<(String, Vec<u8>)> {
        let response = self.http.get(format!("{}/job", self.base)).query(&[("id", id), ("result", "1")]).send().await?;
        let status = response.status();
        if !status.is_success() {
            bail!("engine result: {status}: {}", response.text().await.unwrap_or_default());
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        Ok((content_type, response.bytes().await?.to_vec()))
    }

    pub async fn tracks(&self, id: &str) -> Result<Vec<Track>> {
        let (content_type, body) = self.result_raw(id).await?;
        let boundary = boundary_of(&content_type).unwrap_or_else(|| "yue2-batch-boundary".into());
        let mut tracks = Vec::new();
        let mut replay: Option<Value> = None;
        for (headers, data) in split_multipart(&body, &boundary) {
            let kind = header(&headers, "content-type").unwrap_or_default().to_ascii_lowercase();
            if kind.starts_with("application/json") {
                replay = serde_json::from_slice(data).ok();
            } else if kind.starts_with("audio/") {
                tracks.push(Track { replay: replay.take().unwrap_or(Value::Null), audio: data.to_vec(), content_type: kind });
            }
        }
        if tracks.is_empty() {
            bail!("the engine returned no audio");
        }
        Ok(tracks)
    }

    pub async fn score(&self, id: &str) -> Result<String> {
        let (_, body) = self.result_raw(id).await?;
        let value: Value = serde_json::from_slice(&body)?;
        value.get("abc").and_then(Value::as_str).map(str::to_owned).context("the transcriber returned no score")
    }

    pub async fn codes(&self, id: &str) -> Result<String> {
        let (_, body) = self.result_raw(id).await?;
        let value: Value = serde_json::from_slice(&body)?;
        value.get("codes").and_then(Value::as_str).filter(|codes| !codes.is_empty()).map(str::to_owned).context("the tokenizer returned no codes")
    }
}

fn boundary_of(content_type: &str) -> Option<String> {
    content_type.split(';').map(str::trim).find_map(|part| part.strip_prefix("boundary=")).map(|b| b.trim_matches('"').to_string())
}

fn header(headers: &str, name: &str) -> Option<String> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim().to_string())
    })
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    haystack[from..].windows(needle.len()).position(|window| window == needle).map(|i| i + from)
}

/// Splits a multipart body into (headers, body) pairs.
fn split_multipart<'a>(body: &'a [u8], boundary: &str) -> Vec<(String, &'a [u8])> {
    let delimiter = format!("--{boundary}");
    let delimiter = delimiter.as_bytes();
    let mut parts = Vec::new();
    let Some(mut position) = find(body, delimiter, 0) else { return parts };
    loop {
        let start = position + delimiter.len();
        if body[start..].starts_with(b"--") {
            break;
        }
        let Some(next) = find(body, delimiter, start) else { break };
        let mut part = &body[start..next];
        part = part.strip_prefix(b"\r\n").unwrap_or(part);
        part = part.strip_suffix(b"\r\n").unwrap_or(part);
        if let Some(split) = find(part, b"\r\n\r\n", 0) {
            parts.push((String::from_utf8_lossy(&part[..split]).into_owned(), &part[split + 4..]));
        } else if part.starts_with(b"\r\n") {
            parts.push((String::new(), &part[2..]));
        }
        position = next;
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remake_needs_a_complete_pair() {
        let dir = std::env::temp_dir().join(format!("cadenza-remake-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let touch = |id: &str| std::fs::write(dir.join(catalog::find(id).unwrap().filename), b"").unwrap();
        assert_eq!(remake_missing(&dir, "continue"), vec!["tokenizer-q8", "mert-q8"]);
        touch("mert-q8");
        assert_eq!(remake_missing(&dir, "continue"), vec!["tokenizer-q8"]);
        assert_eq!(remake_missing(&dir, "cover"), vec!["sheetsage-q8"]);
        touch("tokenizer-q8");
        assert!(remake_missing(&dir, "continue").is_empty());
        // An F32 head does not pair with the Q8 encoder.
        touch("sheetsage-f32");
        assert_eq!(remake_missing(&dir, "cover"), vec!["sheetsage-q8"]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn splits_a_two_track_result() {
        let body = b"--b\r\nContent-Type: application/json\r\n\r\n{\"seed\":1}\r\n--b\r\nContent-Type: audio/mpeg\r\n\r\nID3\r\n\r\nxx\r\n--b\r\nContent-Type: application/octet-stream\r\n\r\n\x00\x01\r\n--b--\r\n";
        let parts = split_multipart(body, "b");
        assert_eq!(parts.len(), 3);
        assert_eq!(header(&parts[0].0, "content-type").as_deref(), Some("application/json"));
        assert_eq!(parts[1].1, b"ID3\r\n\r\nxx");
        assert_eq!(boundary_of("multipart/mixed; boundary=yue2-batch-boundary").as_deref(), Some("yue2-batch-boundary"));
    }
}
