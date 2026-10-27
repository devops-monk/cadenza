//! Every file Cadenza can download: the YuE2 music engine in each
//! quantisation, the optional cover transcriber, the language models that
//! write lyrics and the llama.cpp runtime that serves them.
//!
//! Hugging Face files are pinned to a revision so a re-upload upstream can
//! never swap the weights under a user. YuE2 files carry the SHA-256 the
//! YuE2 Studio project publishes; the rest are checked by size.

use serde::Serialize;

const YUE_REPO: &str = "Serveurperso/YuE2-GGUF";
const YUE_REV: &str = "e630f2b8f6aedf8254c3186438633c1b5eeaf80a";
const COMPANION_REPO: &str = "Mothersuperior/yue2-mothersuperior-realaudio-tokenizer-v4";
const COMPANION_REV: &str = "e2e63d859f3af879baf1b4d4e9f22d1eeda6fde5";
pub const LLAMA_BUILD: &str = "b11900";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// YuE2 backbone, one quantisation each; exactly one is used at a time.
    Backbone,
    Vae,
    /// Decoder LoRA merged under every render.
    Companion,
    /// SheetSage2 head, enables covers. Needs `TranscriberBase` beside it.
    Transcriber,
    TranscriberBase,
    /// GGUF text model served by llama-server.
    Llm,
    /// llama.cpp binaries, a tar.gz unpacked into `runtime/llama`.
    LlmRuntime,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: &'static str,
    pub kind: Kind,
    pub label: &'static str,
    pub filename: &'static str,
    pub url: String,
    pub bytes: u64,
    #[serde(skip)]
    pub sha256: Option<&'static str>,
    /// Unified memory / RAM the machine should have to use it comfortably.
    pub ram_gb: u32,
    pub note: &'static str,
    pub license: &'static str,
}

fn hf(repo: &str, rev: &str, file: &str) -> String {
    format!("https://huggingface.co/{repo}/resolve/{rev}/{file}?download=true")
}

#[allow(clippy::too_many_arguments)]
fn yue(id: &'static str, kind: Kind, label: &'static str, filename: &'static str, bytes: u64, sha: &'static str, ram_gb: u32, note: &'static str) -> Item {
    Item { id, kind, label, filename, url: hf(YUE_REPO, YUE_REV, filename), bytes, sha256: Some(sha), ram_gb, note, license: "CC BY-NC 4.0" }
}

fn llm(id: &'static str, label: &'static str, filename: &'static str, url: &str, bytes: u64, ram_gb: u32, note: &'static str) -> Item {
    Item { id, kind: Kind::Llm, label, filename, url: url.to_string(), bytes, sha256: None, ram_gb, note, license: "Gemma Terms of Use" }
}

pub fn items() -> Vec<Item> {
    vec![
        yue("yue-q5", Kind::Backbone, "YuE2 3B · Q5_K_M (Light)", "YuE2-3B-Q5_K_M.gguf", 2_622_936_832,
            "cd3efd250b734a229172800f08f33a893ad1f66b521666e176cdae4a0729b281", 8,
            "Smallest set that still sings in tune. Below Q5 the audio-code model breaks."),
        yue("yue-q6", Kind::Backbone, "YuE2 3B · Q6_K (Balanced)", "YuE2-3B-Q6_K.gguf", 2_943_331_072,
            "42068d5e57713df11b9f6a5a3751072d9bbefe231d1d088f9c100e026ffa4ef9", 16,
            "Close to Q8 quality at three quarters of its size."),
        yue("yue-q8", Kind::Backbone, "YuE2 3B · Q8_0 (Quality)", "YuE2-3B-Q8_0.gguf", 3_810_232_064,
            "41121ce97786d7795a325bcf123ca196956c03bb252c9e75384cfc1f2fc19e6b", 24,
            "The set the engine authors test with."),
        yue("yue-bf16", Kind::Backbone, "YuE2 3B · BF16 (Full precision)", "YuE2-3B-BF16.gguf", 7_166_072_352,
            "668ca9ffa4622449916aae5068b5737363e7ec186a824cd3d7a1a8a95269f166", 32,
            "The original weights, untouched. For machines with memory to spare."),
        yue("yue-vae", Kind::Vae, "YuE2 VAE · F32", "YuE2-Vae-F32.gguf", 530_497_344,
            "93e49dfb1970e89ad64cacb17cf13b5d05f6bb30ef7ed3adae3050bcb728638a", 0,
            "Turns latents into 48 kHz stereo. Always F32: its weights are the audio."),
        Item {
            id: "yue-companion",
            kind: Kind::Companion,
            label: "Realaudio decoder companion v9",
            filename: "nar_lora_joint_v9.safetensors",
            url: hf(COMPANION_REPO, COMPANION_REV, "nar_lora_joint_v9.safetensors"),
            bytes: 140_560_592,
            sha256: Some("585f303da1d5252d228d1e8ac6d4c4d11d970df9297406935cc8bdafa49cfa7e"),
            ram_gb: 0,
            note: "Mothersuperior's decoder LoRA, merged under every render for cleaner audio.",
            license: "CC BY-NC 4.0",
        },
        yue("sheetsage-q8", Kind::Transcriber, "SheetSage2 transcriber · Q8_0", "SheetSage2-Q8_0.gguf", 105_629_856,
            "95d8fa8761753b4833782a52b68d5c0afa3bd3cfbc03d3dcbbf8ded0c13f611e", 0,
            "Optional. Listens to a recording and writes its melody as a score, for covers."),
        yue("sheetsage-f32", Kind::Transcriber, "SheetSage2 transcriber · F32", "SheetSage2-F32.gguf", 228_738_176,
            "4988c01b64c2812dd71eb89a9ab1937a3385ff2ba869bf495ccb69af4ec744cd", 0,
            "Optional, full precision transcriber head."),
        yue("mert-q8", Kind::TranscriberBase, "MERT-v2 audio encoder · Q8_0", "MERT-v2-FullSong-Q8_0.gguf", 902_283_712,
            "35a6032ec7e8123c02becdefc7bfe10a3adb33faa8a1951dc0af864b4494b8ef", 0,
            "Optional. The encoder the transcriber reads; needed for covers."),
        yue("mert-f32", Kind::TranscriberBase, "MERT-v2 audio encoder · F32", "MERT-v2-FullSong-F32.gguf", 2_529_780_448,
            "695b247660405474fffa7bab4d019227325b8d169f96aaa96bad488033fec637", 0,
            "Optional, full precision encoder."),
        llm("gemma-e4b-q4", "Gemma 4 E4B IT · QAT q4_0", "gemma-4-E4B_q4_0-it.gguf",
            "https://huggingface.co/google/gemma-4-E4B-it-qat-q4_0-gguf/resolve/4b4a2c1d584be7264f87aac328a1bc739ce81b6c/gemma-4-E4B_q4_0-it.gguf",
            5_154_941_280, 16, "Light and multilingual. Good lyrics in English, Hindi, Spanish, Chinese and more."),
        llm("gemma-12b-q4", "Gemma 4 12B IT · QAT q4_0", "gemma-4-12b-it-qat-q4_0.gguf",
            "https://huggingface.co/google/gemma-4-12B-it-qat-q4_0-gguf/resolve/29d097773436b69ff9feafd636ab4cf873786537/gemma-4-12b-it-qat-q4_0.gguf",
            6_975_879_296, 24, "Noticeably better rhyme and imagery."),
        llm("gemma-12b-q5", "Gemma 4 12B IT · Q5_K_M", "gemma-4-12b-it-Q5_K_M.gguf",
            "https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/fc034cfff751157913579611efad8462ac1be606/gemma-4-12b-it-Q5_K_M.gguf",
            8_413_576_000, 32, "More accurate than q4_0."),
        llm("gemma-12b-q6", "Gemma 4 12B IT · Q6_K", "gemma-4-12b-it-Q6_K.gguf",
            "https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/fc034cfff751157913579611efad8462ac1be606/gemma-4-12b-it-Q6_K.gguf",
            9_786_022_720, 32, "Closer again to the full model."),
        llm("gemma-12b-q8", "Gemma 4 12B IT · Q8_0", "gemma-4-12b-it-Q8_0.gguf",
            "https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/resolve/fc034cfff751157913579611efad8462ac1be606/gemma-4-12b-it-Q8_0.gguf",
            12_669_647_680, 48, "As close to the original as a quantisation gets."),
        Item {
            id: "llama-runtime",
            kind: Kind::LlmRuntime,
            label: "llama.cpp runtime",
            filename: if cfg!(windows) { "llama-runtime.zip" } else { "llama-runtime.tar.gz" },
            url: llama_runtime_url(),
            bytes: 0,
            sha256: None,
            ram_gb: 0,
            note: "Serves the built-in language model. Not needed when you use Ollama or another server.",
            license: "MIT",
        },
    ]
}

/// The llama.cpp build for this platform: Metal on macOS, Vulkan elsewhere,
/// which runs on NVIDIA, AMD and Intel cards alike.
fn llama_runtime_url() -> String {
    let asset = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-arm64.tar.gz"
    } else if cfg!(target_os = "macos") {
        "macos-x64.tar.gz"
    } else if cfg!(windows) {
        "win-vulkan-x64.zip"
    } else {
        "ubuntu-vulkan-x64.tar.gz"
    };
    format!("https://github.com/ggml-org/llama.cpp/releases/download/{LLAMA_BUILD}/llama-{LLAMA_BUILD}-bin-{asset}")
}

pub fn find(id: &str) -> Option<Item> {
    items().into_iter().find(|item| item.id == id)
}

/// The backbone that fits this machine's memory best.
pub fn recommended_backbone(ram_gb: f64) -> &'static str {
    match ram_gb {
        r if r >= 48.0 => "yue-bf16",
        r if r >= 24.0 => "yue-q8",
        r if r >= 15.0 => "yue-q6",
        _ => "yue-q5",
    }
}

pub fn recommended_llm(ram_gb: f64) -> &'static str {
    match ram_gb {
        r if r >= 64.0 => "gemma-12b-q8",
        r if r >= 32.0 => "gemma-12b-q5",
        r if r >= 24.0 => "gemma-12b-q4",
        _ => "gemma-e4b-q4",
    }
}

/// The transcriber and the encoder beside it, matched by precision.
pub fn transcriber_pairs() -> [(&'static str, &'static str); 2] {
    [("sheetsage-q8", "mert-q8"), ("sheetsage-f32", "mert-f32")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_recommendations_exist() {
        let all = items();
        for (i, a) in all.iter().enumerate() {
            assert!(all.iter().skip(i + 1).all(|b| b.id != a.id), "duplicate id {}", a.id);
        }
        for ram in [8.0, 16.0, 24.0, 32.0, 64.0, 128.0] {
            assert!(find(recommended_backbone(ram)).is_some());
            assert!(find(recommended_llm(ram)).is_some());
        }
        assert_eq!(recommended_backbone(16.0), "yue-q6");
    }
}
