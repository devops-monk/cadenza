//! Turns a description into what YuE2 reads: a style caption, lyrics with
//! section tags, and a title for the library.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::Settings;
use crate::styles::{self, Preset};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Written {
    pub title: String,
    pub style: String,
    pub lyrics: String,
}

pub struct Brief<'a> {
    pub description: &'a str,
    pub lyrics: &'a str,
    pub title: &'a str,
    pub language: &'a str,
    pub instrumental: bool,
    pub duration: f32,
    pub preset: Option<&'static Preset>,
}

const SYSTEM: &str = "You are a hit songwriter and music producer for Hollywood and Bollywood. You answer with one JSON object and nothing else: no markdown fences, no commentary.";

/// Section tags without words: YuE2 plays the structure with no singing.
pub const INSTRUMENTAL_LYRICS: &str = "[Intro]\n\n[Verse]\n\n[Chorus]\n\n[Verse]\n\n[Chorus]\n\n[Bridge]\n\n[Chorus]\n\n[Outro]";

fn lines_for(duration: f32) -> &'static str {
    match duration as u32 {
        0..=75 => "[Verse 1] and [Chorus], 8-10 lines in total",
        76..=150 => "[Verse 1], [Chorus], [Verse 2], [Chorus], 16-20 lines in total",
        151..=240 => "[Verse 1], [Pre-Chorus], [Chorus], [Verse 2], [Pre-Chorus], [Chorus], [Bridge], [Chorus], [Outro], 24-32 lines in total",
        _ => "three verses ([Verse 1] to [Verse 3]), a [Pre-Chorus] before each [Chorus], a [Bridge] and an [Outro], 32-40 lines in total",
    }
}

/// The language of the song, as the brief or the preset names it.
fn language_of<'a>(brief: &'a Brief) -> Option<&'a str> {
    let chosen = brief.language.trim();
    if !chosen.is_empty() && chosen != "auto" {
        return Some(chosen);
    }
    brief.preset.map(|p| p.language)
}

fn user_prompt(brief: &Brief) -> String {
    let language = match language_of(brief) {
        Some(language) => styles::script_rule(language),
        None => format!(
            "the language the description asks for (English when it names none). {}; any other language in its native script",
            styles::script_rule("Hindi").replacen("Hindi", "Hindi, Punjabi, Urdu and other Indian languages are", 1)
        ),
    };
    let mut prompt = format!("Song request: \"{}\"\n\n", brief.description.trim());
    if let Some(preset) = brief.preset {
        prompt.push_str(&format!("Genre: {}. Start from these style tags: \"{}\". {}\n\n", preset.label, preset.tags, preset.writing));
    }
    prompt.push_str("Return JSON with exactly these keys:\n");
    prompt.push_str("- \"title\": a short catchy song title (2-6 words) in the song's language and script.\n");
    let sung = language_of(brief).map(styles::language_tag).unwrap_or("the language of the lyrics, e.g. English or Hindi");
    prompt.push_str(&format!(
        "- \"style\": comma separated English tags for a music model, 8-14 short tags and no sentences, in this order: genre and sub-genre, mood, 3-5 key instruments, the voice (e.g. \"soft male vocals\", \"powerful female vocals\"), the tempo as \"<number> bpm\", and last the language sung ({sung}). Keep the request's details (a child's song stays playful). No artist or film names.\n"
    ));
    if brief.instrumental {
        prompt.push_str("- \"lyrics\": the empty string \"\". The piece is instrumental: put \"instrumental\" among the tags and no voice or language tag.\n");
    } else if !brief.lyrics.trim().is_empty() {
        prompt.push_str("- \"lyrics\": the lyrics below, copied exactly, word for word.\n");
        prompt.push_str(&format!("\nLyrics:\n{}\n", brief.lyrics.trim()));
    } else {
        prompt.push_str(&format!(
            "- \"lyrics\": original singable lyrics in {language}. Structure: {}. Put each section tag alone on its own line in square brackets and a blank line between sections. Lines of 6-10 syllables with a steady rhythm, simple vivid words and rhymes at the line ends. Every [Chorus] repeats the same words and holds the hook. Use every specific detail from the request (names, ages, places, hobbies). No stage directions, no parentheses, no line numbers.\n",
            lines_for(brief.duration)
        ));
    }
    if !brief.title.trim().is_empty() {
        prompt.push_str(&format!("\nThe title is fixed: \"{}\".\n", brief.title.trim()));
    }
    prompt
}

/// The first `{ ... }` object in the reply, read leniently.
fn parse(reply: &str) -> Option<Written> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    if end <= start {
        return None;
    }
    let value: Value = serde_json::from_str(&reply[start..=end]).ok()?;
    let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let written = Written { title: text("title"), style: text("style"), lyrics: text("lyrics") };
    (!written.style.is_empty()).then_some(written)
}

/// Section tags on lines of their own, as YuE2 was trained on.
pub fn tidy_lyrics(lyrics: &str) -> String {
    let mut out = Vec::new();
    for line in lyrics.replace("\r\n", "\n").lines() {
        let line = line.trim();
        let bare = line.trim_matches(|c| c == '*' || c == '#').trim();
        if bare.starts_with('[') && bare.ends_with(']') {
            if out.last().is_some_and(|last: &String| !last.is_empty()) {
                out.push(String::new());
            }
            out.push(bare.to_string());
        } else {
            out.push(line.to_string());
        }
    }
    let text = out.join("\n");
    let text = text.trim();
    if text.starts_with('[') || text.is_empty() {
        text.to_string()
    } else {
        format!("[Verse]\n{text}")
    }
}

pub async fn write(settings: &Settings, brief: &Brief<'_>) -> Result<Written> {
    let prompt = user_prompt(brief);
    let mut last_reply = String::new();
    for attempt in 0..3 {
        let temperature = if attempt == 0 { 0.8 } else { 0.6 };
        let reply = crate::llm::chat(settings, SYSTEM, &prompt, temperature).await?;
        if let Some(mut written) = parse(&reply) {
            if brief.instrumental {
                written.lyrics = INSTRUMENTAL_LYRICS.into();
            } else if !brief.lyrics.trim().is_empty() {
                written.lyrics = tidy_lyrics(brief.lyrics);
            } else if written.lyrics.is_empty() {
                last_reply = reply;
                continue;
            } else {
                written.lyrics = tidy_lyrics(&written.lyrics);
            }
            if !brief.title.trim().is_empty() {
                written.title = brief.title.trim().into();
            }
            if written.title.is_empty() {
                written.title = fallback_title(brief.description);
            }
            return Ok(written);
        }
        last_reply = reply;
    }
    bail!("the language model did not return a usable song (last reply: {})", last_reply.chars().take(200).collect::<String>())
}

/// A title from the first words of the description, when nothing better exists.
pub fn fallback_title(description: &str) -> String {
    let words: Vec<&str> = description.split_whitespace().take(5).collect();
    let title = words.join(" ");
    let mut chars = title.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Untitled".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json_inside_chatter() {
        let reply = "Sure!\n```json\n{\"title\":\"Babu Paints\",\"style\":\"warm acoustic\",\"lyrics\":\"**[Verse]**\\nline\"}\n```";
        let written = parse(reply).unwrap();
        assert_eq!(written.title, "Babu Paints");
        assert_eq!(tidy_lyrics(&written.lyrics), "[Verse]\nline");
    }

    #[test]
    fn untagged_lyrics_get_a_verse_and_sections_get_space() {
        assert_eq!(tidy_lyrics("a\nb"), "[Verse]\na\nb");
        assert_eq!(tidy_lyrics("[Verse]\na\n[Chorus]\nb"), "[Verse]\na\n\n[Chorus]\nb");
        assert_eq!(fallback_title("compose a hindi song"), "Compose a hindi song");
    }
}

#[cfg(test)]
mod live {
    use super::*;
    use crate::settings::LlmProvider;

    /// `CADENZA_LLM_URL=http://127.0.0.1:8080 cargo test live -- --ignored --nocapture`:
    /// writes a few songs with a running OpenAI-compatible server.
    #[tokio::test]
    #[ignore]
    async fn writes_tags_and_lyrics() {
        let settings = Settings {
            llm_provider: LlmProvider::OpenAi,
            openai_url: std::env::var("CADENZA_LLM_URL").expect("CADENZA_LLM_URL"),
            openai_model: "local".into(),
            ..Settings::default()
        };
        let briefs = [
            ("compose a hindi song for my 3 years old son. his name is babu singh and he loves music and he goes to nursery and likes painting", "auto", None),
            ("meeting again in the monsoon rain after five years", "auto", Some("bollywood-romantic")),
            ("a song about finding peace at dawn", "Hindi (Devanagari)", Some("sufi")),
            ("driving through a neon city at midnight", "auto", Some("pop")),
        ];
        for (description, language, preset) in briefs {
            let brief = Brief { description, lyrics: "", title: "", language, instrumental: false, duration: 120.0, preset: preset.and_then(styles::find) };
            let written = write(&settings, &brief).await.unwrap();
            println!("=== {description}\nTITLE {}\nSTYLE {}\n{}\nSECTIONS {:?}\n", written.title, written.style, written.lyrics, styles::score_sections(&written.lyrics));
            assert!(written.style.contains(','), "tags, not prose: {}", written.style);
        }
    }
}
