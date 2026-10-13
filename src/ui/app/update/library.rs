//! The library: albums made, filled, learned and opened in the player.

use iced::Task;
use std::path::PathBuf;

use crate::library::{self, Album};
use crate::ui::app::App;
use crate::ui::app::message::{LibraryView, Message};
use crate::ui::files::{open_with_system, pick_album_files, pick_album_folder};

impl App {
    pub(super) fn show_library(&mut self, view: LibraryView) -> Task<Message> {
        self.menu = None;
        self.deleting_album = None;
        self.library_view = Some(view);
        Task::none()
    }

    pub(super) fn close_library(&mut self) -> Task<Message> {
        self.library_view = None;
        self.deleting_album = None;
        Task::none()
    }

    /// A new album, empty, opened to be filled.
    pub(super) fn new_album(&mut self) -> Task<Message> {
        let album = Album::new("New album");
        let id = album.id.clone();
        if let Err(err) = self.library.put(album) {
            return Task::done(Message::ReportError(err));
        }
        self.show_library(LibraryView::Album(id))
    }

    pub(super) fn import_folder(&self) -> Task<Message> {
        if self.library_busy {
            return Task::none();
        }
        Task::perform(pick_album_folder(), Message::FolderPicked)
    }

    /// An album from the folder picked, made off the interface's thread:
    /// the files are copied, the tabs read.
    pub(super) fn folder_picked(&mut self, folder: Option<PathBuf>) -> Task<Message> {
        let Some(folder) = folder else {
            return Task::none();
        };
        self.library_busy = true;
        let root = self.library.root().to_path_buf();
        Task::perform(
            blocking(move || library::album_from_folder(&root, &folder)),
            Message::AlbumReady,
        )
    }

    pub(super) fn add_to_album(&self, id: String) -> Task<Message> {
        if self.library_busy {
            return Task::none();
        }
        Task::perform(pick_album_files(), move |files| {
            Message::FilesPicked(id.clone(), files)
        })
    }

    pub(super) fn files_picked(&mut self, id: &str, files: Vec<PathBuf>) -> Task<Message> {
        let Some(album) = self.library.album(id).cloned() else {
            return Task::none();
        };
        if files.is_empty() {
            return Task::none();
        }
        self.library_busy = true;
        let root = self.library.root().to_path_buf();
        Task::perform(
            blocking(move || library::add_files(&root, album, &files)),
            Message::AlbumReady,
        )
    }

    /// An album made or filled: kept, and shown.
    pub(super) fn album_ready(&mut self, result: Result<Album, String>) -> Task<Message> {
        self.library_busy = false;
        match result {
            Ok(album) => {
                let id = album.id.clone();
                if let Err(err) = self.library.put(album) {
                    return Task::done(Message::ReportError(err));
                }
                self.show_library(LibraryView::Album(id))
            }
            Err(err) => Task::done(Message::ReportError(err)),
        }
    }

    /// Change an album's title or artist, kept as typed.
    pub(super) fn edit_album(&mut self, id: &str, edit: impl FnOnce(&mut Album)) -> Task<Message> {
        let Some(mut album) = self.library.album(id).cloned() else {
            return Task::none();
        };
        edit(&mut album);
        self.save_album(album)
    }

    fn save_album(&mut self, album: Album) -> Task<Message> {
        match self.library.put(album) {
            Ok(()) => Task::none(),
            Err(err) => Task::done(Message::ReportError(err)),
        }
    }

    pub(super) fn toggle_block(&mut self, id: &str, song: usize, block: usize) -> Task<Message> {
        match self.library.toggle_block(id, song, block) {
            Ok(()) => Task::none(),
            Err(err) => Task::done(Message::ReportError(err)),
        }
    }

    /// The song opens in the player, the library steps aside.
    pub(super) fn play_song(&mut self, id: &str, song: usize) -> Task<Message> {
        let Some(album) = self.library.album(id) else {
            return Task::none();
        };
        let Some(song) = album.songs.get(song) else {
            return Task::none();
        };
        let path = album.folder(self.library.root()).join(&song.file);
        self.library_view = None;
        self.open_file(path)
    }

    pub(super) fn open_pdf(&self, id: &str, pdf: usize) -> Task<Message> {
        let Some(album) = self.library.album(id) else {
            return Task::none();
        };
        let Some(name) = album.pdfs.get(pdf) else {
            return Task::none();
        };
        let path = album.folder(self.library.root()).join(name);
        match open_with_system(&path) {
            Ok(()) => Task::none(),
            Err(err) => Task::done(Message::ReportError(format!(
                "Could not open {name}: {err}"
            ))),
        }
    }

    /// Forget a song or a PDF of an album, and delete its file.
    pub(super) fn remove_from_album(
        &mut self,
        id: &str,
        remove: impl FnOnce(&mut Album) -> Option<String>,
    ) -> Task<Message> {
        let Some(mut album) = self.library.album(id).cloned() else {
            return Task::none();
        };
        let folder = album.folder(self.library.root());
        if let Some(file) = remove(&mut album)
            && let Err(err) = std::fs::remove_file(folder.join(&file))
        {
            log::warn!("Could not delete {file}: {err}");
        }
        self.save_album(album)
    }

    pub(super) fn delete_album(&mut self, id: &str) -> Task<Message> {
        self.deleting_album = None;
        if let Err(err) = self.library.remove(id) {
            return Task::done(Message::ReportError(err));
        }
        self.show_library(LibraryView::Shelf)
    }
}

/// Run `work`, which reads and writes files, off the interface's thread.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| format!("The library work stopped: {err}"))?
}
