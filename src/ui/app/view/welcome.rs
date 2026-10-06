//! The welcome screen, until a file is opened.

use iced::widget::{Space, center, column, container, responsive, row, text};
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

/// Height the welcome screen takes without the recent files card and the
/// shortcuts, its padding and the gap before the card included.
const WELCOME_BASE_HEIGHT: f32 = 350.0;
/// The same, on a phone: no logo nor subtitle.
const COMPACT_BASE_HEIGHT: f32 = 170.0;
/// Height of the shortcuts, with the space above them.
const SHORTCUTS_HEIGHT: f32 = 210.0;
/// The recent files card shows about four files before scrolling.
const RECENT_CARD_MAX_HEIGHT: f32 = 250.0;
/// Below this, the card is left out rather than squeezed to a sliver.
const RECENT_CARD_MIN_HEIGHT: f32 = 110.0;

impl App {
    /// Shown in place of the tablature until a file is opened: how to open
    /// one, the files opened lately, and the shortcuts. It fits the window:
    /// the recent files scroll in a card of their own, and a short window
    /// leaves the shortcuts out instead of cutting them.
    pub(super) fn welcome(&self, tokens: Tokens) -> Element<'_, Message> {
        responsive(move |size| self.welcome_content(tokens, size.height)).into()
    }

    fn welcome_content(&self, tokens: Tokens, height: f32) -> Element<'_, Message> {
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
                shortcut(&["+", "\u{2212}"], "Zoom in / out"),
            ]
            .spacing(6),
            column![
                shortcut(&["S"], "Solo the track"),
                shortcut(&["M"], "Mute the track"),
                shortcut(&["L"], "Loop the measure"),
                shortcut(&["\u{2303}\u{2318}F"], "Fullscreen, Esc to leave"),
            ]
            .spacing(6),
        ]
        .spacing(28);

        // a phone held across keeps the room for the recent files
        let compact = height < 600.0;
        let mut content = if compact {
            column![text("MacGuitar").size(22).font(UI_FONT_BOLD)]
        } else {
            column![
                logo(88.0),
                Space::new().height(6),
                text("MacGuitar").size(30).font(UI_FONT_BOLD),
                muted("A Guitar Pro tablature player").size(15),
                Space::new().height(18),
            ]
        };
        content = content
            .push(open)
            .push(muted("or drop a .gp, .gpx, .gp5, .gp4 or .gp3 file onto the window").size(13))
            .spacing(8)
            .align_x(Alignment::Center);
        let recent_height = if self.recent_files.is_empty() {
            0.0
        } else {
            RECENT_CARD_MAX_HEIGHT
        };
        let shows_shortcuts = height
            >= WELCOME_BASE_HEIGHT + recent_height.min(RECENT_CARD_MIN_HEIGHT) + SHORTCUTS_HEIGHT;
        let base = if compact {
            COMPACT_BASE_HEIGHT
        } else {
            WELCOME_BASE_HEIGHT
        };
        let room = height
            - base
            - if shows_shortcuts {
                SHORTCUTS_HEIGHT
            } else {
                0.0
            };
        if !self.recent_files.is_empty() && room >= RECENT_CARD_MIN_HEIGHT {
            content = content
                .push(Space::new().height(22))
                .push(self.recent_file_list(room.min(RECENT_CARD_MAX_HEIGHT)));
        }
        if shows_shortcuts {
            content = content.push(Space::new().height(30)).push(shortcuts);
        }

        center(content).padding(24).into()
    }
}
