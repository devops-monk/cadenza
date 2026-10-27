//! Genre presets: the style tags YuE2 reads best for each, and what the lyric
//! writer should know about the genre.
//!
//! YuE2 was trained on short comma separated descriptors (the yue2.cpp
//! request reference and the official demos): genre first, then mood,
//! a few instruments, the voice, a tempo in bpm and the language sung.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub language: &'static str,
    pub tags: &'static str,
    /// Lyric guidance for the language model.
    #[serde(skip)]
    pub writing: &'static str,
}

const ENGLISH: &str = "Hollywood / English";
const INDIAN: &str = "Bollywood / Indian";

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "pop",
        group: ENGLISH,
        label: "Pop",
        language: "English",
        tags: "modern pop, catchy, upbeat, bright synths, punchy drums, electric bass, clean electric guitar, polished female vocals, 112 bpm, English",
        writing: "A big repeated hook in the chorus, conversational verses, a pre-chorus that lifts into the chorus.",
    },
    Preset {
        id: "love-ballad",
        group: ENGLISH,
        label: "Love ballad",
        language: "English",
        tags: "pop ballad, romantic, heartfelt, piano, acoustic guitar, lush strings, soft drums, warm male vocals, 72 bpm, English",
        writing: "Tender, specific images of two people; long vowels at line ends that are easy to hold; the chorus says the love plainly.",
    },
    Preset {
        id: "rock",
        group: ENGLISH,
        label: "Rock anthem",
        language: "English",
        tags: "rock, anthemic, energetic, distorted electric guitar, driving drums, bass guitar, powerful male vocals, gang vocals, 128 bpm, English",
        writing: "Punchy short lines, defiant energy, a shout-along chorus.",
    },
    Preset {
        id: "hip-hop",
        group: ENGLISH,
        label: "Hip hop",
        language: "English",
        tags: "hip hop, trap, confident, 808 bass, crisp hi-hats, dark piano loop, male rap vocals, melodic hook, 90 bpm, English",
        writing: "Rapped verses with internal rhymes and a steady flow, a short sung hook for the chorus.",
    },
    Preset {
        id: "rnb",
        group: ENGLISH,
        label: "R&B soul",
        language: "English",
        tags: "R&B, neo soul, smooth, romantic, Rhodes electric piano, groovy bass, soft drums, silky female vocals, 84 bpm, English",
        writing: "Smooth, intimate lines with room for runs; a sensual, repeated chorus.",
    },
    Preset {
        id: "edm",
        group: ENGLISH,
        label: "Dance / EDM",
        language: "English",
        tags: "EDM, dance pop, euphoric, four on the floor, synth bass, bright synth leads, big drop, female vocals, 124 bpm, English",
        writing: "Few words, very repeatable; a chorus of one or two lines sung again and again.",
    },
    Preset {
        id: "country",
        group: ENGLISH,
        label: "Country",
        language: "English",
        tags: "country pop, storytelling, warm, acoustic guitar, banjo, fiddle, steel guitar, male vocals, 100 bpm, English",
        writing: "Tell a small story with concrete places and objects; a chorus that sums it up.",
    },
    Preset {
        id: "folk",
        group: ENGLISH,
        label: "Acoustic folk",
        language: "English",
        tags: "indie folk, acoustic, nostalgic, fingerpicked acoustic guitar, soft percussion, vocal harmonies, gentle female vocals, 96 bpm, English",
        writing: "Plain, honest words and nature images; a gentle sing-along chorus.",
    },
    Preset {
        id: "jazz",
        group: ENGLISH,
        label: "Jazz",
        language: "English",
        tags: "jazz, swing, smoky, upright bass, brushed drums, warm piano, muted trumpet, female vocals, 120 bpm, English",
        writing: "Witty, classic phrasing; a short refrain that comes back.",
    },
    Preset {
        id: "kids",
        group: ENGLISH,
        label: "Kids song",
        language: "English",
        tags: "children's song, playful, happy, ukulele, glockenspiel, hand claps, bright vocals, simple melody, 110 bpm, English",
        writing: "Very simple words a small child can sing, lots of repetition, a chorus with a name or a counting game.",
    },
    Preset {
        id: "bollywood-romantic",
        group: INDIAN,
        label: "Bollywood romantic",
        language: "Hindi",
        tags: "Bollywood romantic, filmi, emotional, soft male vocals, acoustic guitar, lush strings, piano, tabla, bansuri flute, 84 bpm, Hindi",
        writing: "Hindustani love words (dil, pyaar, ishq, saathiya, khwaab, baarish, chaand). The chorus is the mukhda: a 2-4 line hook that returns after every verse (antara). Rhyme the line ends.",
    },
    Preset {
        id: "bollywood-dance",
        group: INDIAN,
        label: "Bollywood dance",
        language: "Hindi",
        tags: "Bollywood dance, party, energetic, dhol, tabla, electronic beat, brass hits, synth, catchy hook, female vocals, 128 bpm, Hindi",
        writing: "Celebration and dance words, a chant-like chorus that repeats, short punchy lines with Hinglish phrases welcome.",
    },
    Preset {
        id: "hindi-sad",
        group: INDIAN,
        label: "Sad Hindi song",
        language: "Hindi",
        tags: "Hindi sad song, heartbreak, melancholic, emotional male vocals, piano, strings, soft acoustic guitar, 70 bpm, Hindi",
        writing: "Longing and separation (judaai, yaadein, aansu, tanha); a slow mukhda hook that returns; rhymed couplets.",
    },
    Preset {
        id: "sufi",
        group: INDIAN,
        label: "Sufi",
        language: "Hindi",
        tags: "Sufi, devotional, spiritual, harmonium, tabla, hand claps, qawwali chorus, powerful male vocals, 96 bpm, Hindi",
        writing: "Devotional love for the divine (maula, rabba, ishq, noor, sajda); a chorus repeated like a chant that builds.",
    },
    Preset {
        id: "ghazal",
        group: INDIAN,
        label: "Ghazal",
        language: "Urdu",
        tags: "ghazal, soulful, poetic, harmonium, tabla, sarangi, soft strings, gentle male vocals, 66 bpm, Urdu",
        writing: "Rhymed couplets (sher) that end on the same refrain word (radif); Urdu poetic words (husn, mehfil, sharaab, chaandni).",
    },
    Preset {
        id: "punjabi",
        group: INDIAN,
        label: "Punjabi bhangra",
        language: "Punjabi",
        tags: "Punjabi pop, bhangra, high energy, dhol, tumbi, heavy bass, party anthem, male vocals, 100 bpm, Punjabi",
        writing: "Punjabi party words (balle balle, nachna, yaar, gabru, kudi), a shout-along chorus.",
    },
    Preset {
        id: "hindi-indie",
        group: INDIAN,
        label: "Hindi indie pop",
        language: "Hindi",
        tags: "Hindi indie pop, breezy, acoustic, ukulele, acoustic guitar, light drums, soft female vocals, 104 bpm, Hindi",
        writing: "Everyday, conversational Hindi with a little English; a light, hummable chorus.",
    },
    Preset {
        id: "desi-hip-hop",
        group: INDIAN,
        label: "Desi hip hop",
        language: "Hindi",
        tags: "Hindi hip hop, desi rap, trap beat, 808 bass, tabla sample, male rap vocals, melodic hook, 92 bpm, Hindi",
        writing: "Rapped verses in Hindi with internal rhymes and street pride, a short sung hook.",
    },
    Preset {
        id: "bhajan",
        group: INDIAN,
        label: "Bhajan",
        language: "Hindi",
        tags: "bhajan, devotional, peaceful, harmonium, tabla, manjira, bansuri flute, temple bells, warm vocals, 80 bpm, Hindi",
        writing: "Simple devotional praise, a chorus that repeats the name being praised.",
    },
    Preset {
        id: "lori",
        group: INDIAN,
        label: "Lori (lullaby)",
        language: "Hindi",
        tags: "lullaby, lori, tender, soothing, soft female vocals, music box, acoustic guitar, bansuri flute, soft strings, 64 bpm, Hindi",
        writing: "Soft, sleepy words (so ja, chanda, nindiya, pari); short lines and a gentle repeated chorus.",
    },
    Preset {
        id: "hindi-kids",
        group: INDIAN,
        label: "Kids song (Hindi)",
        language: "Hindi",
        tags: "children's song, nursery rhyme, playful, happy, ukulele, xylophone, hand claps, dholak, bright vocals, 112 bpm, Hindi",
        writing: "Very simple Hindi a small child can sing, lots of repetition, the child's name in the chorus.",
    },
    Preset {
        id: "classical-fusion",
        group: INDIAN,
        label: "Indian classical fusion",
        language: "Hindi",
        tags: "Indian classical fusion, raga, meditative, sitar, tabla, tanpura drone, modern electronic beat, female vocals with classical ornaments, 90 bpm, Hindi",
        writing: "Poetic, nature-filled imagery (saawan, megha, payal); long vowels to ornament; a refrain that returns.",
    },
];

pub fn find(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Languages YuE2 sings better from Latin letters: its text side was trained
/// on English and Chinese, and Indian film lyrics circulate in Roman script
/// (the official Punjabi demo is written that way).
const ROMANIZED: &[&str] = &["hindi", "punjabi", "urdu", "bengali", "tamil", "telugu", "marathi", "gujarati", "kannada", "malayalam"];

/// How the lyrics of a language are written: "Hindi" in Roman letters,
/// "Hindi (Devanagari)" and every other language in its own script.
pub fn script_rule(language: &str) -> String {
    let language = language.trim();
    if let Some(name) = language.strip_suffix(')').and_then(|l| l.split_once(" (")) {
        return format!("{} written in {} script", name.0, name.1);
    }
    if ROMANIZED.contains(&language.to_lowercase().as_str()) {
        format!("{language} written in Roman letters (no native script, no diacritics), the way film lyric sites print songs, e.g. \"Tum hi ho, ab tum hi ho\"; spell simply and the same way every time (aa, ee, oo for long vowels)")
    } else {
        format!("{language} in its native script")
    }
}

/// The language as a style tag: "Hindi (Devanagari)" is sung as Hindi.
pub fn language_tag(language: &str) -> &str {
    language.split(" (").next().unwrap_or(language).trim()
}

/// The section names the planned score writes after "%", in lyric order:
/// the planner only knows these, so other tags ([Male Vocals]) are skipped.
/// An intro and an outro frame the song, as the planner does on its own.
pub fn score_sections(lyrics: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lyrics.lines() {
        let line = line.trim();
        let Some(tag) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else { continue };
        let tag = tag.to_lowercase();
        let name = if tag.starts_with("intro") || tag.starts_with("instrumental intro") {
            "intro"
        } else if tag.starts_with("pre-chorus") || tag.starts_with("prechorus") || tag.starts_with("pre chorus") {
            "pre-chorus"
        } else if tag.starts_with("post-chorus") || tag.starts_with("post chorus") {
            "post-chorus"
        } else if tag.contains("chorus") || tag.starts_with("hook") || tag.starts_with("refrain") {
            "chorus"
        } else if tag.starts_with("verse") {
            "verse"
        } else if tag.starts_with("bridge") {
            "bridge"
        } else if tag.starts_with("interlude") || tag.contains("solo") || tag.contains("break") || tag.starts_with("instrumental") {
            "interlude"
        } else if tag.starts_with("outro") || tag == "end" {
            "outro"
        } else {
            continue;
        };
        out.push(name.to_string());
    }
    if out.is_empty() {
        return out;
    }
    if out[0] != "intro" {
        out.insert(0, "intro".into());
    }
    if out.last().map(String::as_str) != Some("outro") {
        out.push("outro".into());
    }
    out.truncate(64);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_tags_with_a_tempo_and_a_language() {
        let mut ids = std::collections::HashSet::new();
        for p in PRESETS {
            assert!(ids.insert(p.id), "duplicate {}", p.id);
            let tags: Vec<&str> = p.tags.split(", ").collect();
            assert!((7..=14).contains(&tags.len()), "{}: {} tags", p.id, tags.len());
            assert!(tags.iter().any(|t| t.ends_with(" bpm")), "{}", p.id);
            assert_eq!(*tags.last().unwrap(), p.language, "{}", p.id);
            assert!(!p.writing.is_empty());
        }
    }

    #[test]
    fn indian_languages_are_romanized_unless_asked() {
        assert!(script_rule("Hindi").contains("Roman letters"));
        assert!(script_rule("Punjabi").contains("Roman letters"));
        assert_eq!(script_rule("Hindi (Devanagari)"), "Hindi written in Devanagari script");
        assert_eq!(script_rule("Japanese"), "Japanese in its native script");
        assert_eq!(language_tag("Hindi (Devanagari)"), "Hindi");
    }

    #[test]
    fn sections_follow_the_lyrics_inside_an_intro_and_outro() {
        let lyrics = "[Verse 1]\na\n\n[Pre-Chorus]\nb\n\n[Chorus]\nc\n\n[Male Vocals]\n\n[Guitar Solo]\n\n[Final Chorus]\nd";
        assert_eq!(score_sections(lyrics), ["intro", "verse", "pre-chorus", "chorus", "interlude", "chorus", "outro"]);
        assert_eq!(score_sections("[Intro]\n\n[Verse]\nx\n\n[Outro]"), ["intro", "verse", "outro"]);
        assert!(score_sections("no tags").is_empty());
    }
}
