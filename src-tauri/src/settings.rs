//! Preferences, saved as JSON in the app data folder.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    /// llama-server sidecar running a downloaded GGUF.
    Builtin,
    /// Ollama's OpenAI-compatible endpoint.
    Ollama,
    /// Any server speaking `/v1/chat/completions`.
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Where model weights live; empty = `<app data>/models`.
    pub models_dir: String,
    /// Catalog id of the YuE2 backbone the engine loads.
    pub backbone: String,
    pub llm_provider: LlmProvider,
    /// Catalog id of the GGUF the built-in runtime serves.
    pub llm_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub openai_url: String,
    pub openai_model: String,
    pub openai_key: String,
    /// Stop llama-server before music starts so the two never share memory.
    pub unload_llm_before_music: bool,
    /// `--keep-loaded`: keep every engine module resident between stages.
    pub keep_loaded: bool,
    /// `--max-seq`: smaller KV cache, less memory, shorter maximum songs.
    pub max_seq: Option<u32>,
    /// Use the companion decoder LoRA when it is downloaded.
    pub use_companion: bool,
    /// Explicit `yue-server`; empty = the bundled one.
    pub engine_path: String,
    pub output_format: String,
    pub mp3_bitrate: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let ram = crate::system::ram_gb();
        Self {
            models_dir: String::new(),
            backbone: crate::catalog::recommended_backbone(ram).into(),
            llm_provider: LlmProvider::Builtin,
            llm_model: crate::catalog::recommended_llm(ram).into(),
            ollama_url: "http://127.0.0.1:11434".into(),
            ollama_model: "gemma3:4b".into(),
            openai_url: "http://127.0.0.1:1234".into(),
            openai_model: String::new(),
            openai_key: String::new(),
            unload_llm_before_music: ram < 32.0,
            keep_loaded: ram >= 32.0,
            max_seq: None,
            use_companion: true,
            engine_path: String::new(),
            output_format: "mp3".into(),
            mp3_bitrate: 256,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
    }

    pub fn models_dir(&self, data_dir: &Path) -> PathBuf {
        if self.models_dir.trim().is_empty() {
            data_dir.join("models")
        } else {
            PathBuf::from(self.models_dir.trim())
        }
    }

    /// The settings the running engine process was started with; a change
    /// to any of them means a restart.
    pub fn engine_fingerprint(&self) -> String {
        format!("{}|{}|{:?}|{}|{}|{}", self.backbone, self.keep_loaded, self.max_seq, self.use_companion, self.engine_path, self.models_dir)
    }
}
