//! Every finished track: its audio file, and what it was made from, in
//! `<app data>/library/index.json`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    /// "song", "cover" or "variation".
    pub kind: String,
    pub description: String,
    pub style: String,
    pub lyrics: String,
    #[serde(default)]
    pub abc: String,
    pub instrumental: bool,
    pub created_at: String,
    /// File name inside the library folder.
    pub file: String,
    pub format: String,
    #[serde(default)]
    pub duration: f64,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub lm_seed: Option<i64>,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub backbone: String,
    /// The engine's replay request (score, seeds, semantic codes).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub replay: Value,
}

pub struct Library {
    pub dir: PathBuf,
    pub tracks: Vec<Track>,
}

impl Library {
    pub fn open(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let tracks = std::fs::read_to_string(dir.join("index.json")).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        Self { dir, tracks }
    }

    fn save(&self) -> Result<()> {
        let temp = self.dir.join("index.json.tmp");
        std::fs::write(&temp, serde_json::to_vec_pretty(&self.tracks)?)?;
        std::fs::rename(temp, self.dir.join("index.json"))?;
        Ok(())
    }

    pub fn add(&mut self, track: Track) -> Result<()> {
        self.tracks.insert(0, track);
        self.save()
    }

    pub fn get(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    pub fn path_of(&self, track: &Track) -> PathBuf {
        self.dir.join(&track.file)
    }

    pub fn update(&mut self, id: &str, title: Option<String>, favorite: Option<bool>) -> Result<Track> {
        let track = self.tracks.iter_mut().find(|track| track.id == id).context("no such track")?;
        if let Some(title) = title.filter(|t| !t.trim().is_empty()) {
            track.title = title.trim().to_string();
        }
        if let Some(favorite) = favorite {
            track.favorite = favorite;
        }
        let updated = track.clone();
        self.save()?;
        Ok(updated)
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(index) = self.tracks.iter().position(|track| track.id == id) {
            let track = self.tracks.remove(index);
            let _ = std::fs::remove_file(self.dir.join(&track.file));
            self.save()?;
        }
        Ok(())
    }
}

/// A file name safe on every file system, from a title.
pub fn safe_name(title: &str) -> String {
    // A blacklist rather than is_alphanumeric: the vowel signs of Devanagari
    // and other scripts are marks, not letters, and must survive.
    let cleaned: String = title.chars().map(|c| if c.is_control() || r#"/\:*?"<>|'.!,;"#.contains(c) { ' ' } else { c }).collect();
    let joined = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = joined.chars().take(60).collect();
    if short.is_empty() { "song".into() } else { short }
}

pub fn extension(format: &str) -> &'static str {
    if format.starts_with("wav") { "wav" } else { "mp3" }
}

/// Song length from the audio bytes: WAV header, or MP3 by bitrate of the
/// first frame (Cadenza asks the engine for constant bitrate).
pub fn duration_of(bytes: &[u8], format: &str, bitrate_kbps: u32) -> f64 {
    if format.starts_with("wav") && bytes.len() > 44 && &bytes[0..4] == b"RIFF" {
        let mut position = 12;
        let (mut byte_rate, mut data) = (0u32, 0u32);
        while position + 8 <= bytes.len() {
            let id = &bytes[position..position + 4];
            let size = u32::from_le_bytes(bytes[position + 4..position + 8].try_into().unwrap());
            if id == b"fmt " && position + 16 <= bytes.len() {
                byte_rate = u32::from_le_bytes(bytes[position + 16..position + 20].try_into().unwrap());
            }
            if id == b"data" {
                data = size;
                break;
            }
            position += 8 + size as usize + (size as usize & 1);
        }
        return if byte_rate > 0 { data as f64 / byte_rate as f64 } else { 0.0 };
    }
    if bitrate_kbps > 0 {
        bytes.len() as f64 * 8.0 / (bitrate_kbps as f64 * 1000.0)
    } else {
        0.0
    }
}

pub fn export(source: &Path, destination: &Path) -> Result<()> {
    std::fs::copy(source, destination).with_context(|| format!("could not save to {}", destination.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_safe() {
        assert_eq!(safe_name("Babu's / Painting: Song!"), "Babu s Painting Song");
        assert_eq!(safe_name("???"), "song");
        assert_eq!(safe_name("बाबू का गाना"), "बाबू का गाना");
    }
}
