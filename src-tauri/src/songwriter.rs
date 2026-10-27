//! Turns a description into what YuE2 reads: a style caption, lyrics with
//! section tags, and a title for the library.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::Settings;

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
}

const SYSTEM: &str = "You are a hit songwriter and music producer. You answer with one JSON object and nothing else: no markdown fences, no commentary.";

/// Section tags without words: YuE2 plays the structure with no singing.
pub const INSTRUMENTAL_LYRICS: &str = "[Intro]\n\n[Verse]\n\n[Chorus]\n\n[Verse]\n\n[Chorus]\n\n[Bridge]\n\n[Chorus]\n\n[Outro]";

fn lines_for(duration: f32) -> &'static str {
    match duration as u32 {
        0..=75 => "2 short sections (one verse and one chorus), 8-10 lines in total",
        76..=150 => "verse, chorus, verse, chorus, 16-20 lines in total",
        151..=240 => "verse, chorus, verse, chorus, bridge, chorus, outro, 22-30 lines in total",
        _ => "intro, two or three verses, repeated choruses, a bridge and an outro, 30-40 lines in total",
    }
}

fn user_prompt(brief: &Brief) -> String {
    let language = if brief.language.trim().is_empty() || brief.language == "auto" {
        "the language the description asks for (English when it names none). Write it in that language's native script".to_string()
    } else {
        format!("{} (in its native script)", brief.language)
    };
    let mut prompt = format!("Song request: \"{}\"\n\n", brief.description.trim());
    prompt.push_str("Return JSON with exactly these keys:\n");
    prompt.push_str("- \"title\": a short catchy song title (2-6 words) in the song's language.\n");
    prompt.push_str("- \"style\": ONE English paragraph of 30-60 words describing the production for a music model: genre and sub-genre, era, tempo (slow / mid / fast and a BPM), key instruments, the singer (gender, age feel, timbre, delivery), mood, and the arrangement arc. No artist names.\n");
    if brief.instrumental {
        prompt.push_str("- \"lyrics\": the empty string \"\". The piece is instrumental: say \"instrumental, no vocals\" in the style.\n");
    } else if !brief.lyrics.trim().is_empty() {
        prompt.push_str("- \"lyrics\": the lyrics below, copied exactly, word for word.\n");
        prompt.push_str(&format!("\nLyrics:\n{}\n", brief.lyrics.trim()));
    } else {
        prompt.push_str(&format!(
            "- \"lyrics\": original singable lyrics in {language}. Structure: {}. Put each section tag on its own line, exactly one of [Verse], [Pre-Chorus], [Chorus], [Bridge], [Outro], and a blank line between sections. Short lines with a steady syllable count, simple vivid words, rhymes, and a chorus that repeats a hook. Use every specific detail from the request (names, ages, places, hobbies). No stage directions, no parentheses, no line numbers.\n",
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
