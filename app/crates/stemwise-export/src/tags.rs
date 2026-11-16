//! Metadata (MAS-9) in each format's own tags, through `lofty`: RIFF `LIST/INFO` and ID3v2 for WAV, Vorbis comments
//! and a front-cover picture for FLAC, `ilst` atoms and cover art for M4A.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::{ItemKey, Tag, TagExt, TagType};
use stemwise_core::model::Metadata;

use crate::ExportError;

/// The container a tag goes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Wav,
    Flac,
    M4a,
}

/// A cover image: its bytes and MIME type ("image/jpeg" or "image/png").
pub struct Cover<'a> {
    pub data: &'a [u8],
    pub mime: &'a str,
}

// The ISRC rule lives in the core, which also enforces it on every master edit (MAS-9).
pub use stemwise_core::model::{normalise_isrc, valid_isrc};

fn tag_err(e: impl std::fmt::Display) -> ExportError {
    ExportError::Tags(e.to_string())
}

fn fill(tag: &mut Tag, meta: &Metadata, cover: &Option<Picture>) {
    let mut put = |key: ItemKey, v: &str| {
        let v = v.trim();
        if !v.is_empty() {
            tag.insert_text(key, v.to_string());
        }
    };
    put(ItemKey::TrackTitle, &meta.title);
    put(ItemKey::TrackArtist, &meta.artist);
    put(ItemKey::AlbumTitle, &meta.album);
    put(ItemKey::RecordingDate, &meta.year);
    put(ItemKey::Genre, &meta.genre);
    if let Ok(n) = meta.track_number.trim().parse::<u32>() {
        tag.insert_text(ItemKey::TrackNumber, n.to_string());
    }
    if let Some(isrc) = normalise_isrc(&meta.isrc) {
        tag.insert_text(ItemKey::Isrc, isrc);
    }
    if let Some(p) = cover {
        tag.push_picture(p.clone());
    }
}

/// Writes title, artist, album, year, genre, track number, ISRC and the cover into an existing file. Empty fields
/// are not written.
pub fn write_tags(path: &Path, container: Container, meta: &Metadata, cover: Option<Cover<'_>>) -> crate::Result<()> {
    let picture = cover.map(|c| {
        Picture::unchecked(c.data.to_vec())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::from_str(c.mime))
            .build()
    });
    let types: &[TagType] = match container {
        Container::Wav => &[TagType::RiffInfo, TagType::Id3v2],
        Container::Flac => &[TagType::VorbisComments],
        Container::M4a => &[TagType::Mp4Ilst],
    };
    for &tt in types {
        let mut tag = Tag::new(tt);
        // RIFF INFO cannot carry pictures or ISRC; the ID3v2 tag does.
        let pic = if tt == TagType::RiffInfo { &None } else { &picture };
        fill(&mut tag, meta, pic);
        if tt == TagType::Id3v2 {
            // The generic conversion orders frames through a map: sorted by id, the bytes are the same every time
            // (MAS-8).
            let mut frames: Vec<_> = lofty::id3::v2::Id3v2Tag::from(tag).into_iter().collect();
            frames.sort_by(|a, b| a.id_str().cmp(b.id_str()));
            let mut id3 = lofty::id3::v2::Id3v2Tag::new();
            for f in frames {
                id3.insert(f);
            }
            id3.save_to_path(path, WriteOptions::default()).map_err(tag_err)?;
        } else {
            tag.save_to_path(path, WriteOptions::default()).map_err(tag_err)?;
        }
    }
    Ok(())
}
