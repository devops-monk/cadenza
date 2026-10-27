//! Recordings for remakes. The engine decodes only WAV and MP3, so anything
//! else (FLAC, M4A/AAC, OGG, AIFF, ALAC) is decoded here and handed over as a
//! 16-bit mono WAV, which is all the transcriber and the tokenizer listen to.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::{Hint, ProbeResult};

/// Extensions the file picker offers.
pub const EXTENSIONS: &[&str] = &["mp3", "wav", "flac", "m4a", "aac", "mp4", "ogg", "oga", "aif", "aiff"];

#[derive(Debug, Clone, Serialize)]
pub struct AudioInfo {
    pub name: String,
    pub duration: f64,
}

fn probe(path: &Path) -> Result<ProbeResult> {
    let file = std::fs::File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| anyhow::anyhow!("this file is not audio Cadenza can read (try MP3, WAV, FLAC, M4A or OGG)"))
}

/// Decodes the first audio track to mono f32 samples and their rate.
pub fn decode(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut format = probe(path)?.format;
    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).context("the file has no audio track")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| anyhow::anyhow!("this audio codec is not supported (try MP3, WAV, FLAC, M4A or OGG)"))?;
    let mut rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut mono = Vec::new();
    let mut buffer: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            // A damaged frame is skipped, as players do.
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let spec = *decoded.spec();
        rate = spec.rate;
        let channels = spec.channels.count().max(1);
        let buf = buffer.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        if buf.capacity() < decoded.capacity() * channels {
            *buf = SampleBuffer::new(decoded.capacity() as u64, spec);
        }
        buf.copy_interleaved_ref(decoded);
        mono.extend(buf.samples().chunks(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32));
    }
    if mono.is_empty() || rate == 0 {
        bail!("the recording decoded to silence");
    }
    Ok((mono, rate))
}

/// Name and length, for the form. Reads the header when it says, else decodes.
pub fn info(path: &Path) -> Result<AudioInfo> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let format = probe(path)?.format;
    let from_header = format.default_track().and_then(|track| {
        let params = &track.codec_params;
        let frames = params.n_frames?;
        let rate = params.sample_rate?;
        (frames > 0 && rate > 0).then(|| frames as f64 / rate as f64)
    });
    drop(format);
    let duration = match from_header {
        Some(duration) => duration,
        None => {
            let (samples, rate) = decode(path)?;
            samples.len() as f64 / rate as f64
        }
    };
    Ok(AudioInfo { name, duration })
}

/// The bytes and file name to upload to the engine: MP3 as it is, anything
/// else as a WAV it is sure to read.
pub fn for_engine(path: &Path) -> Result<(Vec<u8>, String)> {
    let is_mp3 = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("mp3"));
    if is_mp3 {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "source.mp3".into());
        return Ok((std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?, name));
    }
    let (samples, rate) = decode(path)?;
    Ok((wav_16(&samples, rate), "source.wav".into()))
}

fn wav_16(samples: &[f32], rate: u32) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip() {
        let rate = 22050;
        let samples: Vec<f32> = (0..rate).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        let dir = std::env::temp_dir().join(format!("cadenza-audio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tone.wav");
        std::fs::write(&path, wav_16(&samples, rate)).unwrap();
        let info = info(&path).unwrap();
        assert!((info.duration - 1.0).abs() < 0.01, "{}", info.duration);
        let (decoded, decoded_rate) = decode(&path).unwrap();
        assert_eq!(decoded_rate, rate);
        assert_eq!(decoded.len(), samples.len());
        let (bytes, name) = for_engine(&path).unwrap();
        assert_eq!(name, "source.wav");
        assert_eq!(&bytes[..4], b"RIFF");
        std::fs::remove_dir_all(dir).ok();
    }
}

#[cfg(test)]
mod real_files {
    /// `CADENZA_AUDIO_DIR=… cargo test real_files -- --ignored --nocapture`:
    /// reads every file there and writes what the engine would receive.
    #[test]
    #[ignore]
    fn converts_every_file() {
        let dir = std::path::PathBuf::from(std::env::var("CADENZA_AUDIO_DIR").expect("CADENZA_AUDIO_DIR"));
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "out") {
                continue;
            }
            let info = super::info(&path).unwrap();
            let (bytes, name) = super::for_engine(&path).unwrap();
            println!("{:<16} {:>6.2}s -> {name} {} bytes", info.name, info.duration, bytes.len());
            std::fs::write(path.with_extension(format!("{}.out", path.extension().unwrap().to_string_lossy())), bytes).unwrap();
        }
    }
}
