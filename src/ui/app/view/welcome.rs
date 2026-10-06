//! The welcome screen, until a file is opened.

use iced::widget::{Space, center, column, container, row, text};
use iced::{Alignment, Element};

use crate::ui::app::App;
use crate::ui::app::message::Message;
use crate::ui::icons::logo;
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::{UI_FONT_BOLD, muted, open_button};

/// A keyboard key.
fn kbd<'a>(key: &'a str) -> Element<'a, Message> {
    container(text(key).size(12).font(UI_FONT_BOLD))
        .padding([3, 8])
        .style(theme::kbd)
        .into()
}

fn shortcut<'a>(keys: &[&'a str], action: &'a str) -> Element<'a, Message> {
    let keys = row(keys.iter().map(|k| kbd(k))).spacing(4);
    // rows of one width, so the two columns balance around the centre
    row![
        container(keys).width(110).align_x(Alignment::End),
        muted(action).size(13)
    ]
    .spacing(12)
    .width(300)
    .align_y(Alignment::Center)
    .into()
}

impl App {
    /// Shown in place of the tablature until a file is opened: how to open
    /// one, the files opened lately, and the shortcuts.
    pub(super) fn welcome(&self, tokens: Tokens) -> Element<'_, Message> {
        let is_loading = self.is_loading;
        let open: Element<'_, Message> = if is_loading {
            muted("Loading\u{2026}").size(14).into()
        } else {
            open_button("Open a tab", true, tokens)
        };

        let shortcuts = row![
            column![
                shortcut(&["Space"], "Play / pause"),
                shortcut(&["\u{2190}", "\u{2192}"], "Previous / next measure"),
                shortcut(&["Ctrl", "\u{2191}\u{2193}"], "Tempo up / down"),
                shortcut(&["Right drag"], "Loop over measures"),
            ]
            .spacing(10),
            column![
                shortcut(&["S"], "Solo the track"),
                shortcut(&["M"], "Mute the track"),
                shortcut(&["L"], "Loop the measure"),
                shortcut(&["\u{2303}\u{2318}F"], "Fullscreen, Esc to leave"),
            ]
            .spacing(10),
        ]
        .spacing(28);

        let mut content = column![
            logo(88.0),
            Space::new().height(6),
            text("MacGuitar").size(30).font(UI_FONT_BOLD),
            muted("A Guitar Pro tablature player").size(15),
            Space::new().height(18),
            open,
            muted("or drop a .gp, .gpx, .gp5, .gp4 or .gp3 file onto the window").size(13),
        ]
        .spacing(8)
        .align_x(Alignment::Center);
        if !self.recent_files.is_empty() {
            content = content
                .push(Space::new().height(22))
                .push(self.recent_file_list(6));
        }
        content = content.push(Space::new().height(30)).push(shortcuts);

        center(content).padding(24).into()
    }
}
