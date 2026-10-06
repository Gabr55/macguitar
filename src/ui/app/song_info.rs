//! The details of the song shown in the header.

use crate::parser::model::{GpVersion, Song};

#[derive(Debug)]
pub(super) struct SongDisplayInfo {
    pub(super) name: String,
    pub(super) artist: String,
    pub(super) subtitle: String,
    pub(super) album: String,
    pub(super) author: String,
    pub(super) writer: String,
    pub(super) copyright: String,
    pub(super) gp_version: GpVersion,
    pub(super) file_name: String,
}

impl SongDisplayInfo {
    pub(super) fn new(song: &Song, file_name: String) -> Self {
        Self {
            name: song.song_info.name.clone(),
            artist: song.song_info.artist.clone(),
            subtitle: song.song_info.subtitle.clone(),
            album: song.song_info.album.clone(),
            author: song.song_info.author.clone(),
            writer: song.song_info.writer.clone(),
            copyright: song.song_info.copyright.clone(),
            gp_version: song.version,
            file_name,
        }
    }

    /// Metadata fields joined with a middle dot, skipping empty ones.
    /// Returns `None` if no metadata is available.
    pub(super) fn metadata_line(&self) -> Option<String> {
        let parts: Vec<&str> = [
            self.subtitle.as_str(),
            self.album.as_str(),
            self.author.as_str(),
            self.writer.as_str(),
            self.copyright.as_str(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("  \u{00b7}  "))
        }
    }
}
