//! Picking files in dialogs and reading the tablatures.

use std::path::{Path, PathBuf};

/// File extensions supported by the parser; used by both the dialog filter
/// and the validation of files loaded directly (command line, drag and drop,
/// recent files).
const SUPPORTED_EXTENSIONS: [&str; 5] = ["gp5", "gp4", "gp3", "gpx", "gp"];

/// A tablature file read from disk.
#[derive(Debug, Clone)]
pub struct LoadedFile {
    /// Absolute path of the file.
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

impl LoadedFile {
    /// The file name, as shown in the window title.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The folder holding the file, where the next dialog opens.
    pub fn folder(&self) -> Option<PathBuf> {
        self.path.parent().map(Path::to_path_buf)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FileError {
    #[error("unsupported file extension: {0:?}")]
    UnsupportedExtension(String),
    #[error("{path}: {reason}")]
    Unreadable { path: PathBuf, reason: String },
}

/// Whether a file looks like a tablature the parser reads.
pub fn is_tablature(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SUPPORTED_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
}

/// Asks for a tablature, starting in `folder`; `None` when dismissed.
pub async fn pick_tablature(folder: Option<PathBuf>) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("Guitar Pro files", &SUPPORTED_EXTENSIONS)
        .set_title("Select a Guitar Pro file");
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    dialog
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

/// Asks for a SoundFont; `None` when dismissed.
pub async fn pick_sound_font() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("SoundFont", &["sf2"])
        .set_title("Select a SoundFont")
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

/// Reads the tablature at `path`.
pub async fn load_tablature(path: PathBuf) -> Result<LoadedFile, FileError> {
    if !is_tablature(&path) {
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Err(FileError::UnsupportedExtension(extension));
    }
    let unreadable = |error: std::io::Error| FileError::Unreadable {
        path: path.clone(),
        reason: error.to_string(),
    };
    // a relative path from the command line is kept absolute, for the
    // recent files and the next dialog
    let path = tokio::fs::canonicalize(&path).await.map_err(unreadable)?;
    log::info!("Loading file {}", path.display());
    let contents = tokio::fs::read(&path).await.map_err(unreadable)?;
    Ok(LoadedFile { path, contents })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_tablatures_by_extension() {
        assert!(is_tablature(Path::new("song.gp5")));
        assert!(is_tablature(Path::new("SONG.GPX")));
        assert!(is_tablature(Path::new("dir/song.gp")));
        assert!(!is_tablature(Path::new("song.pdf")));
        assert!(!is_tablature(Path::new("song")));
    }
}
