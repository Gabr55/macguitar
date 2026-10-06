//! The files opened lately: listed on the welcome screen, and in a menu
//! next to the open button.

use iced::advanced::text::Shaping;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, text};
use iced::{Element, Length};
use std::path::Path;

use crate::ui::app::App;
use crate::ui::app::message::Message;
use crate::ui::theme;
use crate::ui::widgets::{UI_FONT_BOLD, muted};

/// Width of the recent files menu.
pub(super) const RECENT_MENU_WIDTH: f32 = 380.0;

impl App {
    /// The first `count` recent files, each opening on a click.
    pub(super) fn recent_file_list(&self, count: usize) -> Element<'_, Message> {
        let entries = self
            .recent_files
            .iter()
            .take(count)
            .map(|path| recent_entry(path));
        // the heading lines up with the names, inside the entries' padding
        column![container(muted("Recent").size(12)).padding([0, 10])]
            .extend(entries)
            .spacing(2)
            .width(360)
            .into()
    }

    /// The recent files, as a menu under its button.
    pub(super) fn recent_menu(&self) -> Element<'_, Message> {
        container(column(self.recent_files.iter().map(|path| recent_entry(path))).spacing(2))
            .padding(6)
            .width(RECENT_MENU_WIDTH)
            .style(theme::menu_panel)
            .into()
    }
}

/// A recent file: its name, and the folder holding it below.
fn recent_entry(path: &Path) -> Element<'_, Message> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = path.parent().map(home_relative).unwrap_or_default();
    // a long folder is cut at the entry's edge rather than run past it
    let label = container(
        column![
            text(name)
                .size(13)
                .font(UI_FONT_BOLD)
                .shaping(Shaping::Advanced)
                .wrapping(Wrapping::None),
            muted(folder)
                .size(11)
                .shaping(Shaping::Advanced)
                .wrapping(Wrapping::None),
        ]
        .spacing(1),
    )
    .width(Length::Fill)
    .clip(true);
    button(label)
        .width(Length::Fill)
        .padding([6, 10])
        .style(theme::menu_item(false))
        .on_press(Message::OpenFile(path.to_path_buf()))
        .into()
}

/// Longest folder shown in full; a longer one keeps its last folders.
const MAX_FOLDER_CHARS: usize = 48;

/// `folder` as the entry shows it: the home directory written `~`, and a
/// long path cut from the start, keeping the folders nearest the file.
fn home_relative(folder: &Path) -> String {
    let full = std::env::home_dir()
        .and_then(|home| folder.strip_prefix(home).ok().map(Path::to_path_buf))
        .map_or_else(
            || folder.display().to_string(),
            |relative| format!("~/{}", relative.display()),
        );
    shorten_folder(&full, MAX_FOLDER_CHARS)
}

/// `folder` within `max` characters: whole, or "…/" and its last folders.
fn shorten_folder(folder: &str, max: usize) -> String {
    if folder.chars().count() <= max {
        return folder.to_string();
    }
    let mut kept = String::new();
    for part in folder.rsplit('/').filter(|part| !part.is_empty()) {
        let candidate = if kept.is_empty() {
            part.to_string()
        } else {
            format!("{part}/{kept}")
        };
        if candidate.chars().count() + 2 > max && !kept.is_empty() {
            break;
        }
        kept = candidate;
    }
    format!("\u{2026}/{kept}")
}

#[cfg(test)]
mod tests {
    use super::shorten_folder;

    #[test]
    fn long_folders_keep_their_end() {
        assert_eq!(shorten_folder("~/Music/Tabs", 48), "~/Music/Tabs");
        assert_eq!(
            shorten_folder(
                "~/Library/Some Very Long Folder/Guitar Pro Files/Collection",
                32
            ),
            "\u{2026}/Guitar Pro Files/Collection"
        );
    }
}
