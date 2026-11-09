//! Cover art inside an MP3: an ID3 picture frame. The audio is not touched.

use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use id3::frame::{Picture, PictureType};
use id3::{Error, ErrorKind, Tag, TagLike, Version};

/// Past this a cover outweighs the song, and some players refuse it.
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// JPEG and PNG are the two formats every player shows; judged by content,
/// since a file's extension can lie.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else {
        None
    }
}

fn is_mp3(path: &Path) -> Result<bool> {
    let mut head = [0u8; 3];
    let mut file = std::fs::File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    file.read(&mut head).with_context(|| format!("could not read {}", path.display()))?;
    Ok(&head == b"ID3" || (head[0] == 0xFF && head[1] & 0xE0 == 0xE0))
}

fn read_image(path: &Path) -> Result<(&'static str, Vec<u8>)> {
    let size = std::fs::metadata(path).with_context(|| format!("could not open {}", path.display()))?.len();
    if size > MAX_IMAGE_BYTES {
        bail!("that image is too large: keep it under 10 MB");
    }
    let data = std::fs::read(path).with_context(|| format!("could not open {}", path.display()))?;
    let mime = image_mime(&data).context("choose a JPEG or PNG image")?;
    Ok((mime, data))
}

fn write_picture(path: &Path, mime: &str, data: Vec<u8>) -> Result<()> {
    let mut tag = match Tag::read_from_path(path) {
        Ok(tag) => tag,
        Err(Error { kind: ErrorKind::NoTag, .. }) => Tag::new(),
        Err(error) => return Err(error).context("could not read the MP3's tags"),
    };
    tag.remove_all_pictures();
    tag.add_frame(Picture { mime_type: mime.into(), picture_type: PictureType::CoverFront, description: String::new(), data });
    // ID3v2.3 is the version Windows Explorer and older players read.
    tag.write_to_path(path, Version::Id3v23).context("could not write the cover into the MP3")
}

/// Writes `source` to `destination` with this cover, replacing any it had.
/// Works on a copy, so a failure leaves both files as they were; `source`
/// and `destination` may be the same file.
pub fn embed(source: &Path, image: &Path, destination: &Path) -> Result<()> {
    if !is_mp3(source)? {
        bail!("cover images can only be added to MP3 files");
    }
    let (mime, data) = read_image(image)?;
    let name = destination.file_name().context("no file name to save to")?.to_string_lossy();
    let temp = destination.with_file_name(format!("{name}.cover-tmp"));
    let result = std::fs::copy(source, &temp)
        .with_context(|| format!("could not save to {}", destination.display()))
        .and_then(|_| write_picture(&temp, mime, data))
        .and_then(|_| std::fs::rename(&temp, destination).with_context(|| format!("could not save to {}", destination.display())));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// The image in this MP3: the front cover, else the first picture.
pub fn read(path: &Path) -> Result<Vec<u8>> {
    let tag = Tag::read_from_path(path).context("could not read the MP3's tags")?;
    let picture = tag.pictures().find(|p| p.picture_type == PictureType::CoverFront).or_else(|| tag.pictures().next()).context("this MP3 has no cover")?;
    Ok(picture.data.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 9, 9];
    const AUDIO: &[u8] = &[0xFF, 0xFB, 0x90, 0x00, 7, 7, 7, 7, 0xFF, 0xFB, 0x90, 0x00, 8, 8];

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("cadenza-cover-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    #[test]
    fn adds_a_cover_and_keeps_the_audio_and_the_source() {
        let dir = Dir::new("add");
        let (mp3, image) = (dir.file("a.mp3", AUDIO), dir.file("c.png", PNG));
        let out = dir.0.join("b.mp3");
        embed(&mp3, &image, &out).unwrap();
        assert_eq!(read(&out).unwrap(), PNG);
        assert!(std::fs::read(&out).unwrap().ends_with(AUDIO));
        assert_eq!(std::fs::read(&mp3).unwrap(), AUDIO);
        assert!(!dir.0.join("b.mp3.cover-tmp").exists());
    }

    #[test]
    fn replaces_a_cover_in_place_and_keeps_other_tags() {
        let dir = Dir::new("replace");
        let mp3 = dir.file("a.mp3", AUDIO);
        let mut tag = Tag::new();
        tag.set_title("Keep me");
        tag.write_to_path(&mp3, Version::Id3v23).unwrap();
        embed(&mp3, &dir.file("c.png", PNG), &mp3).unwrap();
        embed(&mp3, &dir.file("c.jpg", JPEG), &mp3).unwrap();
        let tag = Tag::read_from_path(&mp3).unwrap();
        assert_eq!(tag.title(), Some("Keep me"));
        assert_eq!(tag.pictures().count(), 1);
        assert_eq!(read(&mp3).unwrap(), JPEG);
        assert!(std::fs::read(&mp3).unwrap().ends_with(AUDIO));
    }

    #[test]
    fn refuses_what_it_cannot_embed() {
        let dir = Dir::new("refuse");
        let (mp3, image) = (dir.file("a.mp3", AUDIO), dir.file("c.png", PNG));
        let wav = dir.file("a.wav", b"RIFF....WAVE");
        let fake = dir.file("fake.png", b"GIF89a not a png");
        let out = dir.0.join("b.mp3");
        assert!(embed(&wav, &image, &out).unwrap_err().to_string().contains("MP3"));
        assert!(embed(&mp3, &fake, &out).unwrap_err().to_string().contains("JPEG or PNG"));
        assert!(embed(&dir.file("short.mp3", b""), &image, &out).is_err());
        assert!(!out.exists());
        assert_eq!(std::fs::read(&mp3).unwrap(), AUDIO);
    }
}
