//! The library: albums kept like books, each with its cover, its PDF tab
//! books and its tabs, and how much of every song has been learned.
//!
//! A song's blocks are the parts its tab marks (Intro, Verse...), or the
//! whole song when it marks none; learning them fills the song's progress
//! and the album's. The files are copied into the library, one folder per
//! album, next to `library.json`, which lists the albums.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::parser::parse_gp_data;
use crate::ui::files::is_tablature;

/// The list of the albums, in the library's folder.
const INDEX_FILE: &str = "library.json";
/// Side of the square cover kept, in pixels.
const COVER_SIZE: u32 = 512;
/// The cover's file in its album's folder.
const COVER_FILE: &str = "cover.jpg";
/// The block of a song whose tab marks no parts.
pub const WHOLE_SONG: &str = "Whole song";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub artist: String,
    /// The cover's file name in the album's folder.
    #[serde(default)]
    pub cover: Option<String>,
    /// The PDF tab books, by file name in the album's folder.
    #[serde(default)]
    pub pdfs: Vec<String>,
    #[serde(default)]
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
    /// The tab's file name in the album's folder.
    pub file: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

/// A part of a song to learn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    #[serde(default)]
    pub learned: bool,
}

/// Blocks learned out of all, as a share from 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Progress {
    pub learned: usize,
    pub total: usize,
}

impl Progress {
    pub fn share(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.learned as f32 / self.total as f32
        }
    }

    /// The share as a whole percentage, rounded down: 100% only when done.
    pub fn percent(self) -> u32 {
        (self.share() * 100.0).floor() as u32
    }
}

impl Song {
    pub fn progress(&self) -> Progress {
        Progress {
            learned: self.blocks.iter().filter(|block| block.learned).count(),
            total: self.blocks.len(),
        }
    }
}

impl Album {
    /// A new, empty album.
    pub fn new(title: &str) -> Self {
        Self {
            id: new_id(),
            title: title.to_string(),
            artist: String::new(),
            cover: None,
            pdfs: Vec::new(),
            songs: Vec::new(),
        }
    }

    /// Every block of every song: the one track the album is learned along.
    pub fn progress(&self) -> Progress {
        self.songs
            .iter()
            .map(Song::progress)
            .fold(Progress::default(), |all, song| Progress {
                learned: all.learned + song.learned,
                total: all.total + song.total,
            })
    }

    pub fn folder(&self, root: &Path) -> PathBuf {
        root.join(&self.id)
    }

    pub fn cover_path(&self, root: &Path) -> Option<PathBuf> {
        self.cover
            .as_ref()
            .map(|cover| self.folder(root).join(cover))
    }
}

/// An id for a new album: the time, which no two albums share.
fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("album-{nanos:x}")
}

/// The albums, kept in `root`.
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
    albums: Vec<Album>,
}

impl Library {
    /// The library in `root`, empty when it has none yet.
    pub fn open(root: PathBuf) -> Self {
        let albums = std::fs::read_to_string(root.join(INDEX_FILE))
            .ok()
            .and_then(|text| match serde_json::from_str(&text) {
                Ok(albums) => Some(albums),
                Err(err) => {
                    log::warn!("Could not read the library: {err}");
                    None
                }
            })
            .unwrap_or_default();
        Self { root, albums }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn albums(&self) -> &[Album] {
        &self.albums
    }

    pub fn album(&self, id: &str) -> Option<&Album> {
        self.albums.iter().find(|album| album.id == id)
    }

    /// Keep `album`, new or changed, and save the list.
    pub fn put(&mut self, album: Album) -> Result<(), String> {
        match self.albums.iter_mut().find(|known| known.id == album.id) {
            Some(known) => *known = album,
            None => self.albums.push(album),
        }
        self.save()
    }

    /// Mark a block of a song learned, or not any more.
    pub fn toggle_block(&mut self, id: &str, song: usize, block: usize) -> Result<(), String> {
        let Some(album) = self.albums.iter_mut().find(|album| album.id == id) else {
            return Ok(());
        };
        if let Some(block) = album
            .songs
            .get_mut(song)
            .and_then(|song| song.blocks.get_mut(block))
        {
            block.learned = !block.learned;
        }
        self.save()
    }

    /// Forget an album and delete its files.
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if let Some(album) = self.album(id) {
            let folder = album.folder(&self.root);
            if folder.exists() {
                std::fs::remove_dir_all(&folder)
                    .map_err(|err| format!("Could not delete {}: {err}", folder.display()))?;
            }
        }
        self.albums.retain(|album| album.id != id);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.root)
            .and_then(|()| {
                let text =
                    serde_json::to_string_pretty(&self.albums).map_err(std::io::Error::other)?;
                std::fs::write(self.root.join(INDEX_FILE), text)
            })
            .map_err(|err| format!("Could not save the library: {err}"))
    }
}

/// What a file brings to an album.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tab,
    Pdf,
    Image,
}

impl Kind {
    pub fn of(path: &Path) -> Option<Self> {
        if is_tablature(path) {
            return Some(Self::Tab);
        }
        let extension = path.extension()?.to_str()?.to_lowercase();
        match extension.as_str() {
            "pdf" => Some(Self::Pdf),
            "jpg" | "jpeg" | "png" | "webp" => Some(Self::Image),
            _ => None,
        }
    }
}

/// `album` with `files` added: the tabs as songs, the PDFs as books, an
/// image as its cover. The files are copied into the album's folder; the
/// album's title and artist, when empty, come from the tabs.
pub fn add_files(root: &Path, mut album: Album, files: &[PathBuf]) -> Result<Album, String> {
    let folder = album.folder(root);
    std::fs::create_dir_all(&folder)
        .map_err(|err| format!("Could not make the album's folder: {err}"))?;
    let mut problems = Vec::new();
    for file in files {
        let added = match Kind::of(file) {
            Some(Kind::Tab) => add_tab(&folder, &mut album, file),
            Some(Kind::Pdf) => copy_in(&folder, file).map(|name| album.pdfs.push(name)),
            Some(Kind::Image) => make_cover(&folder, file).map(|name| album.cover = Some(name)),
            None => Err(format!("{} is no tab, PDF or image", file.display())),
        };
        if let Err(problem) = added {
            log::warn!("{problem}");
            problems.push(problem);
        }
    }
    // a file that could not be added says why, unless others were
    if problems.len() == files.len() && !problems.is_empty() {
        return Err(problems.join("\n"));
    }
    Ok(album)
}

/// A new album from a folder: its tabs, PDFs and the image that looks the
/// most like a cover, found down to a few folders deep. The album is named
/// after the folder.
pub fn album_from_folder(root: &Path, folder: &Path) -> Result<Album, String> {
    let mut files = Vec::new();
    collect(folder, 3, &mut files);
    files.sort_by(|a, b| natural_order(&a.to_string_lossy(), &b.to_string_lossy()));
    if !files.iter().any(|file| Kind::of(file) == Some(Kind::Tab)) {
        return Err(format!("{} holds no tabs", folder.display()));
    }
    // one cover: named so, or the first image
    let images: Vec<&PathBuf> = files
        .iter()
        .filter(|file| Kind::of(file) == Some(Kind::Image))
        .collect();
    let cover = images
        .iter()
        .find(|image| {
            let name = image
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            ["cover", "folder", "front", "artwork"]
                .iter()
                .any(|hint| name.contains(hint))
        })
        .or(images.first())
        .map(|image| (*image).clone());
    let mut chosen: Vec<PathBuf> = files
        .iter()
        .filter(|file| matches!(Kind::of(file), Some(Kind::Tab | Kind::Pdf)))
        .cloned()
        .collect();
    chosen.extend(cover);
    let title = folder
        .file_name()
        .map(|name| name.to_string_lossy().replace('_', " ").trim().to_string())
        .unwrap_or_default();
    add_files(root, Album::new(&title), &chosen)
}

/// The tabs, PDFs and images under `folder`, `depth` folders down.
fn collect(folder: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        if hidden {
            continue;
        }
        if path.is_dir() {
            if depth > 0 {
                collect(&path, depth - 1, files);
            }
        } else if Kind::of(&path).is_some() {
            files.push(path);
        }
    }
}

/// Copy `file` into `folder` under a name of its own; returns that name.
fn copy_in(folder: &Path, file: &Path) -> Result<String, String> {
    let name = unique_name(folder, file);
    std::fs::copy(file, folder.join(&name))
        .map_err(|err| format!("Could not copy {}: {err}", file.display()))?;
    Ok(name)
}

/// `file`'s name, numbered if the folder holds one already.
fn unique_name(folder: &Path, file: &Path) -> String {
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    if !folder.join(&name).exists() {
        return name;
    }
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = file
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| format!("{stem} ({n}){extension}"))
        .find(|candidate| !folder.join(candidate).exists())
        .unwrap_or(name)
}

/// Add a tab as a song: copied in, its parts read as blocks.
fn add_tab(folder: &Path, album: &mut Album, file: &Path) -> Result<(), String> {
    let data =
        std::fs::read(file).map_err(|err| format!("Could not read {}: {err}", file.display()))?;
    let song =
        parse_gp_data(&data).map_err(|err| format!("Could not read {}: {err}", file.display()))?;
    let name = copy_in(folder, file)?;
    let info = &song.song_info;
    let title = Some(info.name.trim())
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .or_else(|| file.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| name.clone());
    if album.artist.is_empty() {
        album.artist = info.artist.trim().to_string();
    }
    if album.title.trim().is_empty() {
        album.title = info.album.trim().to_string();
    }
    let mut blocks: Vec<Block> = song
        .measure_headers
        .iter()
        .filter_map(|header| header.marker.as_ref())
        .map(|marker| marker.title.trim().to_string())
        .filter(|name| !name.is_empty())
        .map(|name| Block {
            name,
            learned: false,
        })
        .collect();
    if blocks.is_empty() {
        blocks.push(Block {
            name: WHOLE_SONG.to_string(),
            learned: false,
        });
    }
    album.songs.push(Song {
        file: name,
        title,
        blocks,
    });
    Ok(())
}

/// The cover from `image`: cut square from its middle, scaled down, kept as
/// a JPEG in `folder`.
fn make_cover(folder: &Path, image: &Path) -> Result<String, String> {
    let picture =
        image::open(image).map_err(|err| format!("Could not read {}: {err}", image.display()))?;
    let side = picture.width().min(picture.height());
    let square = picture.crop_imm(
        (picture.width() - side) / 2,
        (picture.height() - side) / 2,
        side,
        side,
    );
    let size = COVER_SIZE.min(side);
    let cover = square
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .to_rgb8();
    let path = folder.join(COVER_FILE);
    cover
        .save(&path)
        .map_err(|err| format!("Could not keep the cover: {err}"))?;
    Ok(COVER_FILE.to_string())
}

/// Names in the order a person reads them: case aside, and "2" before "10".
pub fn natural_order(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let number = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(digit) = chars.next_if(char::is_ascii_digit) {
                        digits.push(digit);
                    }
                    digits
                };
                let (x, y) = (number(&mut a), number(&mut b));
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_in_reading_order() {
        let mut names = vec!["Folder 10", "folder 2", "Folder 1", "02 b", "1 a"];
        names.sort_by(|a, b| natural_order(a, b));
        assert_eq!(names, ["1 a", "02 b", "Folder 1", "folder 2", "Folder 10"]);
    }

    /// A folder of its own for a test, empty.
    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("macguitar-library-{name}-{}", new_id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_tab_brings_its_parts_as_blocks() {
        let root = scratch("blocks");
        let album = add_files(
            &root,
            Album::new(""),
            &[PathBuf::from("test-files/effects.gp5")],
        )
        .unwrap();
        assert_eq!(album.songs.len(), 1);
        let song = &album.songs[0];
        assert!(album.folder(&root).join(&song.file).exists());
        assert!(!song.blocks.is_empty());
        assert_eq!(album.progress().learned, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn progress_adds_the_blocks_of_every_song() {
        let block = |learned| Block {
            name: "A".into(),
            learned,
        };
        let mut album = Album::new("Book");
        album.songs = vec![
            Song {
                file: "a.gp5".into(),
                title: "A".into(),
                blocks: vec![block(true), block(false)],
            },
            Song {
                file: "b.gp5".into(),
                title: "B".into(),
                blocks: vec![block(true), block(true), block(false)],
            },
        ];
        let progress = album.progress();
        assert_eq!((progress.learned, progress.total), (3, 5));
        assert_eq!(progress.percent(), 60);
        assert_eq!(Progress::default().percent(), 0);
    }

    #[test]
    fn the_library_keeps_its_albums_between_starts() {
        let root = scratch("index");
        let mut library = Library::open(root.clone());
        let album = add_files(
            &root,
            Album::new("Book"),
            &[PathBuf::from("test-files/effects.gp5")],
        )
        .unwrap();
        let id = album.id.clone();
        library.put(album).unwrap();
        library.toggle_block(&id, 0, 0).unwrap();

        let reopened = Library::open(root.clone());
        let album = reopened.album(&id).unwrap();
        assert_eq!(album.title, "Book");
        assert!(album.songs[0].blocks[0].learned);

        let mut reopened = reopened;
        reopened.remove(&id).unwrap();
        assert!(reopened.albums().is_empty());
        assert!(!root.join(&id).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_folder_makes_an_album_named_after_it() {
        let root = scratch("folder-root");
        let source = scratch("folder-source").join("Some_Band_Album");
        std::fs::create_dir_all(source.join("tabs")).unwrap();
        std::fs::copy("test-files/effects.gp5", source.join("tabs/effects.gp5")).unwrap();
        std::fs::write(source.join("book.pdf"), b"%PDF-1.4").unwrap();
        let album = album_from_folder(&root, &source).unwrap();
        assert_eq!(album.title, "Some Band Album");
        assert_eq!(album.songs.len(), 1);
        assert_eq!(album.pdfs, vec!["book.pdf".to_string()]);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(source.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_name_taken_gets_a_number() {
        let folder = scratch("names");
        std::fs::write(folder.join("song.gp5"), b"").unwrap();
        assert_eq!(
            unique_name(&folder, Path::new("other/song.gp5")),
            "song (2).gp5"
        );
        assert_eq!(unique_name(&folder, Path::new("other/new.gp5")), "new.gp5");
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_cover_is_cut_square_and_scaled_down() {
        let folder = scratch("cover");
        let picture = image::RgbImage::from_pixel(1200, 800, image::Rgb([200, 80, 40]));
        let source = folder.join("art.png");
        picture.save(&source).unwrap();
        let name = make_cover(&folder, &source).unwrap();
        let cover = image::open(folder.join(name)).unwrap();
        assert_eq!((cover.width(), cover.height()), (512, 512));
        std::fs::remove_dir_all(folder).unwrap();
    }
}
