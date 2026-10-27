//! Resumable downloads with progress events and SHA-256 checks.
//!
//! A file downloads to `<name>.part` beside its destination and resumes from
//! there with an HTTP Range request; it is renamed into place only once its
//! size (and hash, when the catalog has one) matches.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::catalog::{Item, Kind};

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub id: String,
    pub state: &'static str,
    pub downloaded: u64,
    pub total: u64,
    pub bytes_per_sec: f64,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Downloads {
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
    last: Mutex<HashMap<String, Progress>>,
}

impl Downloads {
    pub fn is_active(&self, id: &str) -> bool {
        self.active.lock().unwrap().contains_key(id)
    }

    pub fn snapshot(&self) -> Vec<Progress> {
        self.last.lock().unwrap().values().cloned().collect()
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.active.lock().unwrap().get(id) {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// Starts `item` in the background; a second start of the same id is ignored.
    pub fn start(self: &Arc<Self>, app: AppHandle, item: Item, models_dir: PathBuf, runtime_dir: PathBuf) {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active.lock().unwrap();
            if active.contains_key(item.id) {
                return;
            }
            active.insert(item.id.to_string(), cancel.clone());
        }
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let id = item.id.to_string();
            let result = run(&this, &app, &item, &models_dir, &runtime_dir, &cancel).await;
            let progress = match result {
                Ok(()) => Progress { id: id.clone(), state: "done", downloaded: 0, total: 0, bytes_per_sec: 0.0, error: None },
                Err(_) if cancel.load(Ordering::SeqCst) => Progress { id: id.clone(), state: "cancelled", downloaded: 0, total: 0, bytes_per_sec: 0.0, error: None },
                Err(error) => Progress { id: id.clone(), state: "error", downloaded: 0, total: 0, bytes_per_sec: 0.0, error: Some(format!("{error:#}")) },
            };
            this.active.lock().unwrap().remove(&id);
            this.emit(&app, progress);
        });
    }

    fn emit(&self, app: &AppHandle, progress: Progress) {
        self.last.lock().unwrap().insert(progress.id.clone(), progress.clone());
        let _ = app.emit("download", progress);
    }
}

pub fn destination(item: &Item, models_dir: &Path, runtime_dir: &Path) -> PathBuf {
    match item.kind {
        Kind::LlmRuntime => runtime_dir.join(item.filename),
        _ => models_dir.join(item.filename),
    }
}

/// Whether the item is on disk and whole.
pub fn installed(item: &Item, models_dir: &Path, runtime_dir: &Path) -> bool {
    match item.kind {
        Kind::LlmRuntime => crate::llm::find_llama_server(runtime_dir).is_some(),
        _ => {
            let path = destination(item, models_dir, runtime_dir);
            match std::fs::metadata(&path) {
                Ok(meta) => item.bytes == 0 || meta.len() == item.bytes,
                Err(_) => false,
            }
        }
    }
}

async fn run(downloads: &Downloads, app: &AppHandle, item: &Item, models_dir: &Path, runtime_dir: &Path, cancel: &AtomicBool) -> Result<()> {
    let destination = destination(item, models_dir, runtime_dir);
    let parent = destination.parent().context("destination has no folder")?;
    tokio::fs::create_dir_all(parent).await?;
    let part = destination.with_file_name(format!("{}.part", item.filename));

    let client = reqwest::Client::builder().user_agent("Cadenza/0.1").connect_timeout(Duration::from_secs(20)).build()?;
    let mut have = tokio::fs::metadata(&part).await.map(|meta| meta.len()).unwrap_or(0);
    let mut request = client.get(&item.url);
    if have > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let response = request.send().await?;
    let status = response.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // the part is already whole
    } else if !status.is_success() {
        bail!("{} answered {status}", item.url);
    }
    let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !resumed && status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        have = 0;
    }
    let total = if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        have
    } else {
        response.content_length().map(|length| length + have).unwrap_or(item.bytes)
    };

    if status != reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let mut file = tokio::fs::OpenOptions::new().create(true).write(true).append(resumed).truncate(!resumed).open(&part).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = have;
        let started = Instant::now();
        let mut since_start = 0u64;
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::SeqCst) {
                file.flush().await?;
                bail!("cancelled");
            }
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            since_start += chunk.len() as u64;
            if last_emit.elapsed() > Duration::from_millis(250) {
                last_emit = Instant::now();
                let rate = since_start as f64 / started.elapsed().as_secs_f64().max(0.001);
                downloads.emit(app, Progress { id: item.id.into(), state: "running", downloaded, total, bytes_per_sec: rate, error: None });
            }
        }
        file.flush().await?;
        drop(file);
    }

    let size = tokio::fs::metadata(&part).await?.len();
    if item.bytes > 0 && size != item.bytes {
        if size > item.bytes {
            let _ = tokio::fs::remove_file(&part).await;
        }
        bail!("{} is {size} bytes, expected {}", item.filename, item.bytes);
    }
    if size == 0 {
        let _ = tokio::fs::remove_file(&part).await;
        bail!("{} downloaded empty", item.filename);
    }
    if let Some(expected) = item.sha256 {
        downloads.emit(app, Progress { id: item.id.into(), state: "verifying", downloaded: size, total: size, bytes_per_sec: 0.0, error: None });
        let actual = sha256_file(&part, cancel).await?;
        if actual != expected {
            let _ = tokio::fs::remove_file(&part).await;
            bail!("{} failed its SHA-256 check (got {actual})", item.filename);
        }
    }
    tokio::fs::rename(&part, &destination).await?;

    if item.kind == Kind::LlmRuntime {
        downloads.emit(app, Progress { id: item.id.into(), state: "verifying", downloaded: size, total: size, bytes_per_sec: 0.0, error: None });
        let archive = destination.clone();
        let target = runtime_dir.join("llama");
        tokio::task::spawn_blocking(move || unpack(&archive, &target)).await??;
        let _ = tokio::fs::remove_file(&destination).await;
        if crate::llm::find_llama_server(runtime_dir).is_none() {
            bail!("the llama.cpp archive holds no llama-server");
        }
    }
    Ok(())
}

async fn sha256_file(path: &Path, cancel: &AtomicBool) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 4 << 20];
    loop {
        if cancel.load(Ordering::SeqCst) {
            bail!("cancelled");
        }
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn unpack(archive: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        std::fs::remove_dir_all(target)?;
    }
    std::fs::create_dir_all(target)?;
    let file = std::fs::File::open(archive)?;
    if archive.extension().is_some_and(|ext| ext == "zip") {
        zip::ZipArchive::new(file)?.extract(target)?;
    } else {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        tar.set_preserve_permissions(true);
        tar.unpack(target)?;
    }
    Ok(())
}

pub fn delete(item: &Item, models_dir: &Path, runtime_dir: &Path) -> Result<()> {
    if item.kind == Kind::LlmRuntime {
        let dir = runtime_dir.join("llama");
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        return Ok(());
    }
    let path = destination(item, models_dir, runtime_dir);
    for candidate in [path.clone(), path.with_file_name(format!("{}.part", item.filename))] {
        if candidate.exists() {
            std::fs::remove_file(candidate)?;
        }
    }
    Ok(())
}

/// Bytes already downloaded into a `.part`, so the UI can offer "resume".
pub fn partial_bytes(item: &Item, models_dir: &Path, runtime_dir: &Path) -> u64 {
    let path = destination(item, models_dir, runtime_dir);
    std::fs::metadata(path.with_file_name(format!("{}.part", item.filename))).map(|meta| meta.len()).unwrap_or(0)
}
