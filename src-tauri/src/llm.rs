//! The language model that writes lyrics, titles and style captions.
//!
//! Built-in: a `llama-server` sidecar serving a downloaded GGUF. Otherwise
//! Ollama or any OpenAI-compatible server the user already runs, so a big
//! machine can use whatever model it likes.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::catalog;
use crate::settings::{LlmProvider, Settings};

pub const PORT: u16 = 18088;
const EXECUTABLE: &str = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };

#[derive(Default)]
pub struct LlmRuntime {
    child: Option<Child>,
    model: Option<PathBuf>,
}

/// `llama-server` anywhere under the unpacked runtime.
pub fn find_llama_server(runtime_dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: u32) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.file_name().is_some_and(|name| name == EXECUTABLE) {
                return Some(path);
            }
        }
        if depth == 0 {
            return None;
        }
        dirs.into_iter().find_map(|dir| walk(&dir, depth - 1))
    }
    walk(&runtime_dir.join("llama"), 4)
}

impl LlmRuntime {
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

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            crate::engine::terminate(&mut child);
        }
        self.model = None;
    }

    /// Starts llama-server for `model`; true when the caller must wait for it.
    pub fn spawn(&mut self, runtime_dir: &Path, model: &Path) -> Result<bool> {
        if self.running() && self.model.as_deref() == Some(model) {
            return Ok(false);
        }
        self.stop();
        let executable = find_llama_server(runtime_dir).context("the llama.cpp runtime is not downloaded (Models → Language model)")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&executable) {
                let mut permissions = meta.permissions();
                permissions.set_mode(permissions.mode() | 0o755);
                let _ = std::fs::set_permissions(&executable, permissions);
            }
        }
        let mut command = Command::new(&executable);
        command
            .arg("-m").arg(model)
            .args(["--host", "127.0.0.1", "--port", &PORT.to_string(), "-c", "8192", "-ngl", "999"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(dir) = executable.parent() {
            command.current_dir(dir);
        }
        crate::engine::hide_window(&mut command);
        self.child = Some(command.spawn().with_context(|| format!("could not start {}", executable.display()))?);
        self.model = Some(model.to_path_buf());
        Ok(true)
    }
}

pub fn builtin_model_path(settings: &Settings, models_dir: &Path) -> Result<PathBuf> {
    let item = catalog::find(&settings.llm_model).ok_or_else(|| anyhow!("unknown language model {}", settings.llm_model))?;
    let path = models_dir.join(item.filename);
    if !path.is_file() {
        bail!("{} is not downloaded yet (Models → Language model)", item.label);
    }
    Ok(path)
}

pub async fn wait_ready(timeout: Duration) -> Result<()> {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(response) = client.get(format!("http://127.0.0.1:{PORT}/health")).timeout(Duration::from_secs(1)).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("the language model did not load within {} s", timeout.as_secs())
}

fn v1(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.ends_with("/v1") {
        url.to_string()
    } else {
        format!("{url}/v1")
    }
}

/// (base url, model name, api key) for the configured provider.
pub fn endpoint(settings: &Settings) -> (String, String, String) {
    match settings.llm_provider {
        LlmProvider::Builtin => (format!("http://127.0.0.1:{PORT}/v1"), "local".into(), String::new()),
        LlmProvider::Ollama => (v1(&settings.ollama_url), settings.ollama_model.clone(), String::new()),
        LlmProvider::OpenAi => (v1(&settings.openai_url), settings.openai_model.clone(), settings.openai_key.clone()),
    }
}

pub async fn chat(settings: &Settings, system: &str, user: &str, temperature: f32) -> Result<String> {
    let (base, model, key) = endpoint(settings);
    let client = reqwest::Client::builder().timeout(Duration::from_secs(600)).build()?;
    let mut request = client.post(format!("{base}/chat/completions")).json(&json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": temperature,
        "max_tokens": 2048,
        "stream": false
    }));
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.with_context(|| format!("could not reach the language model at {base}"))?;
    let status = response.status();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let message = body.pointer("/error/message").or_else(|| body.get("error")).map(|v| v.to_string()).unwrap_or_default();
        bail!("language model answered {status} {message}");
    }
    let text = body.pointer("/choices/0/message/content").and_then(Value::as_str).unwrap_or_default();
    Ok(strip_thinking(text))
}

/// Models with a reasoning mode wrap it in tags; only the answer matters.
fn strip_thinking(text: &str) -> String {
    let mut out = text.to_string();
    for (open, close) in [("<think>", "</think>"), ("<thinking>", "</thinking>")] {
        while let (Some(start), Some(end)) = (out.find(open), out.find(close)) {
            if end < start {
                break;
            }
            out.replace_range(start..end + close.len(), "");
        }
    }
    out.trim().to_string()
}

/// Lists Ollama's pulled models for the Settings picker.
pub async fn ollama_models(url: &str) -> Result<Vec<String>> {
    let base = url.trim().trim_end_matches('/').trim_end_matches("/v1");
    let body: Value = reqwest::Client::new().get(format!("{base}/api/tags")).timeout(Duration::from_secs(3)).send().await?.json().await?;
    Ok(body.get("models").and_then(Value::as_array).map(|models| models.iter().filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_owned)).collect()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_is_removed_and_v1_is_added_once() {
        assert_eq!(strip_thinking("<think>hmm</think>\n{\"a\":1}"), "{\"a\":1}");
        assert_eq!(v1("http://x:1/"), "http://x:1/v1");
        assert_eq!(v1("http://x:1/v1"), "http://x:1/v1");
    }
}
