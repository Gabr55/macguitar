//! The library over the player: the albums as covers on a shelf, and an
//! album's page with its songs, their blocks and how much is learned.

use iced::advanced::text::Shaping;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, center, column, container, image, progress_bar, row, scrollable, text,
    text_input,
};
use iced::{Alignment, ContentFit, Element, Length};
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::library::{Album, Progress};
use crate::ui::app::App;
use crate::ui::app::message::{LibraryView, Message};
use crate::ui::icons::{Icon, icon};
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::{UI_FONT_BOLD, icon_button, muted};

/// Width of an album on the shelf.
const CARD_WIDTH: f32 = 196.0;
/// Side of the cover on the album's page.
const HERO_COVER: f32 = 188.0;

impl App {
    pub(super) fn library_layer<'a>(
        &'a self,
        view: &'a LibraryView,
        tokens: Tokens,
    ) -> Element<'a, Message> {
        let content = match view {
            LibraryView::Shelf => self.shelf(tokens),
            LibraryView::Album(id) => self.library.album(id).map_or_else(
                || self.shelf(tokens),
                |album| self.album_page(album, tokens),
            ),
        };
        container(content)
            .padding(if self.is_compact() { 12 } else { 24 })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::panel)
            .into()
    }

    /// The albums, as covers in rows.
    fn shelf(&self, tokens: Tokens) -> Element<'_, Message> {
        let albums = self.library.albums();
        let learned = albums
            .iter()
            .map(Album::progress)
            .fold(Progress::default(), |all, album| Progress {
                learned: all.learned + album.learned,
                total: all.total + album.total,
            });
        let summary = match albums.len() {
            0 => String::new(),
            1 => format!("1 album \u{00b7} {}% learned", learned.percent()),
            count => format!("{count} albums \u{00b7} {}% learned", learned.percent()),
        };
        let heading = column![
            text("Library").size(24).font(UI_FONT_BOLD),
            muted(summary).size(13),
        ]
        .spacing(2)
        .width(Length::Fill);
        let close = icon_button(
            Icon::Close,
            "Back to the player (Esc)",
            Some(Message::CloseLibrary),
            tokens,
        );
        // an empty library has its buttons in the middle, not twice
        let actions = if albums.is_empty() {
            row![self.busy_note()]
        } else {
            row![
                self.busy_note(),
                labelled_button(
                    Icon::Folder,
                    "Import a folder",
                    Message::ImportFolder,
                    false,
                    tokens
                ),
                labelled_button(Icon::Plus, "New album", Message::NewAlbum, true, tokens),
            ]
        }
        .spacing(10)
        .align_y(Alignment::Center);
        // a phone held upright puts the actions under the title
        let top: Element<'_, Message> = if self.is_narrow() {
            column![row![heading, close].align_y(Alignment::Center), actions]
                .spacing(12)
                .into()
        } else {
            row![heading, actions, close]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        };

        let body: Element<'_, Message> = if albums.is_empty() {
            center(
                column![
                    icon(Icon::Library, 46.0, tokens.muted),
                    text("Your library is empty").size(18).font(UI_FONT_BOLD),
                    container(
                        muted(
                            "Make an album of a folder of tabs: its PDF tab books and its cover \
                             come along. Or start one and add the files you like."
                        )
                        .size(14)
                        .align_x(Alignment::Center)
                    )
                    .max_width(420),
                    Space::new().height(4),
                    row![
                        labelled_button(
                            Icon::Folder,
                            "Import a folder",
                            Message::ImportFolder,
                            false,
                            tokens
                        ),
                        labelled_button(Icon::Plus, "New album", Message::NewAlbum, true, tokens),
                    ]
                    .spacing(10),
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            )
            .into()
        } else {
            let cards = albums.iter().map(|album| self.album_card(album));
            scrollable(row(cards).spacing(18).wrap().vertical_spacing(18))
                .height(Length::Fill)
                .style(theme::scrollable_style)
                .into()
        };
        column![top, body].spacing(22).into()
    }

    fn album_card<'a>(&'a self, album: &'a Album) -> Element<'a, Message> {
        let progress = album.progress();
        let content = column![
            self.cover(album, CARD_WIDTH - 24.0),
            column![
                text(&album.title)
                    .size(15)
                    .font(UI_FONT_BOLD)
                    .shaping(Shaping::Advanced)
                    .wrapping(Wrapping::None),
                muted(if album.artist.is_empty() {
                    "\u{2014}"
                } else {
                    album.artist.as_str()
                })
                .size(12)
                .shaping(Shaping::Advanced)
                .wrapping(Wrapping::None),
            ]
            .spacing(2),
            progress_line(progress),
        ]
        .spacing(10);
        button(container(content).width(Length::Fill).clip(true))
            .width(CARD_WIDTH)
            .padding(12)
            .style(theme::album_card)
            .on_press(Message::ShowLibrary(LibraryView::Album(album.id.clone())))
            .into()
    }

    /// The album's cover, square, or a gradient with its initials.
    fn cover<'a>(&'a self, album: &'a Album, side: f32) -> Element<'a, Message> {
        if let Some(path) = album
            .cover_path(self.library.root())
            .filter(|path| Path::exists(path))
        {
            return image(path)
                .width(side)
                .height(side)
                .content_fit(ContentFit::Cover)
                .border_radius(theme::CONTROL_RADIUS)
                .into();
        }
        let initials: String = album
            .title
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        album.id.hash(&mut hasher);
        container(
            text(initials)
                .size(side * 0.3)
                .font(UI_FONT_BOLD)
                .shaping(Shaping::Advanced),
        )
        .center(side)
        .style(theme::cover_placeholder(hasher.finish() as u32))
        .into()
    }

    /// An album's page: its cover and progress, its PDFs, and its songs with
    /// their blocks to tick.
    fn album_page<'a>(&'a self, album: &'a Album, tokens: Tokens) -> Element<'a, Message> {
        let id = album.id.clone();
        let deleting = self.deleting_album.as_deref() == Some(album.id.as_str());
        let delete: Element<'_, Message> = if deleting {
            row![
                muted("Delete the album and its files?").size(13),
                button(text("Delete").size(13).font(UI_FONT_BOLD))
                    .padding([7, 14])
                    .style(theme::danger_button)
                    .on_press(Message::DeleteAlbum(id.clone())),
                button(text("Cancel").size(13))
                    .padding([7, 14])
                    .style(theme::secondary_button)
                    .on_press(Message::CancelDelete),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            icon_button(
                Icon::Trash,
                "Delete the album",
                Some(Message::AskDeleteAlbum(id.clone())),
                tokens,
            )
        };
        let top = row![
            button(
                row![
                    icon(Icon::Back, 16.0, tokens.text),
                    text("Library").size(14)
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .padding([6, 10])
            .style(theme::ghost_button)
            .on_press(Message::ShowLibrary(LibraryView::Shelf)),
            Space::new().width(Length::Fill),
            self.busy_note(),
            delete,
            icon_button(
                Icon::Close,
                "Back to the player (Esc)",
                Some(Message::CloseLibrary),
                tokens
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let progress = album.progress();
        let songs = album.songs.len();
        let counts = format!(
            "{} of {} blocks learned \u{00b7} {} {}",
            progress.learned,
            progress.total,
            songs,
            if songs == 1 { "song" } else { "songs" }
        );
        let title_id = id.clone();
        let artist_id = id.clone();
        let details = column![
            text_input("Album title", &album.title)
                .on_input(move |title| Message::AlbumTitleChanged(title_id.clone(), title))
                .size(26)
                .font(UI_FONT_BOLD)
                .padding([2, 6])
                .style(theme::title_input),
            text_input("Artist", &album.artist)
                .on_input(move |artist| Message::AlbumArtistChanged(artist_id.clone(), artist))
                .size(15)
                .padding([2, 6])
                .style(theme::title_input),
            row![
                text(format!("{}%", progress.percent()))
                    .size(34)
                    .font(UI_FONT_BOLD)
                    .color(tokens.accent),
                column![
                    progress_bar(0.0..=1.0, progress.share())
                        .girth(6)
                        .length(Length::Fixed(260.0))
                        .style(theme::progress),
                    muted(counts).size(12),
                ]
                .spacing(6),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            row![labelled_button(
                Icon::Plus,
                "Add tabs, PDFs or a cover",
                Message::AddToAlbum(id.clone()),
                true,
                tokens
            )]
            .push(pdf_buttons(album, tokens))
            .spacing(8)
            .wrap(),
        ]
        .spacing(10);
        let hero: Element<'_, Message> = if self.is_narrow() {
            column![self.cover(album, HERO_COVER * 0.7), details]
                .spacing(14)
                .into()
        } else {
            row![self.cover(album, HERO_COVER), details]
                .spacing(24)
                .align_y(Alignment::Center)
                .into()
        };

        let list: Element<'_, Message> = if album.songs.is_empty() {
            container(
                muted(
                    "No songs yet: add the album's tabs. Each part their tabs mark \
                     (Intro, Verse, Solo...) becomes a block to learn.",
                )
                .size(14),
            )
            .padding(18)
            .style(theme::song_card)
            .width(Length::Fill)
            .into()
        } else {
            column(
                album
                    .songs
                    .iter()
                    .enumerate()
                    .map(|(index, _)| song_row(album, index, tokens)),
            )
            .spacing(10)
            .into()
        };

        column![
            top,
            scrollable(column![hero, list].spacing(24).padding([0, 4]))
                .height(Length::Fill)
                .style(theme::scrollable_style),
        ]
        .spacing(18)
        .into()
    }

    /// "Adding..." while an album is being made or filled.
    fn busy_note(&self) -> Element<'_, Message> {
        if self.library_busy {
            muted("Adding\u{2026}").size(13).into()
        } else {
            Space::new().into()
        }
    }
}

/// A learned share, as a thin bar and its percentage.
fn progress_line<'a>(progress: Progress) -> Element<'a, Message> {
    row![
        progress_bar(0.0..=1.0, progress.share())
            .girth(4)
            .length(Length::Fill)
            .style(theme::progress),
        text(format!("{}%", progress.percent()))
            .size(11)
            .font(UI_FONT_BOLD),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// A button with an icon and a label: the accent one, or a plain one.
fn labelled_button<'a>(
    glyph: Icon,
    label: &'a str,
    message: Message,
    primary: bool,
    tokens: Tokens,
) -> Element<'a, Message> {
    let color = if primary {
        tokens.on_accent
    } else {
        tokens.text
    };
    button(
        row![
            icon(glyph, 16.0, color),
            text(label).size(13).font(UI_FONT_BOLD)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .style(if primary {
        theme::primary_button
    } else {
        theme::secondary_button
    })
    .on_press(message)
    .into()
}

/// The PDF tab books, each opening in the system's viewer.
fn pdf_buttons(album: &Album, tokens: Tokens) -> Element<'_, Message> {
    row(album.pdfs.iter().enumerate().map(|(index, name)| {
        let open = button(
            row![
                icon(Icon::Document, 16.0, tokens.text),
                text(name.trim_end_matches(".pdf"))
                    .size(13)
                    .shaping(Shaping::Advanced)
                    .wrapping(Wrapping::None),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([7, 12])
        .style(theme::secondary_button)
        .on_press(Message::OpenPdf(album.id.clone(), index));
        row![
            open,
            icon_button(
                Icon::Close,
                "Remove the PDF",
                Some(Message::RemovePdf(album.id.clone(), index)),
                tokens
            )
        ]
        .align_y(Alignment::Center)
        .into()
    }))
    .spacing(8)
    .wrap()
    .into()
}

/// A song: its title and progress, its blocks to tick, and its way to
/// the player.
fn song_row(album: &Album, index: usize, tokens: Tokens) -> Element<'_, Message> {
    let song = &album.songs[index];
    let progress = song.progress();
    let id = &album.id;
    let heading = row![
        muted(format!("{:02}", index + 1)).size(13).width(26),
        text(&song.title)
            .size(15)
            .font(UI_FONT_BOLD)
            .shaping(Shaping::Advanced)
            .wrapping(Wrapping::None)
            .width(Length::Fill),
        text(format!("{}%", progress.percent()))
            .size(14)
            .font(UI_FONT_BOLD)
            .color(
                if progress.learned == progress.total && progress.total > 0 {
                    tokens.accent
                } else {
                    tokens.muted
                }
            ),
        icon_button(
            Icon::Play,
            "Open in the player",
            Some(Message::PlaySong(id.clone(), index)),
            tokens
        ),
        icon_button(
            Icon::Trash,
            "Remove the song",
            Some(Message::RemoveSong(id.clone(), index)),
            tokens
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let blocks = row(song.blocks.iter().enumerate().map(|(block, part)| {
        let label: Element<'_, Message> = if part.learned {
            row![
                icon(Icon::Check, 13.0, tokens.on_accent),
                text(&part.name).size(12).shaping(Shaping::Advanced),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(&part.name).size(12).shaping(Shaping::Advanced).into()
        };
        button(label)
            .padding([5, 12])
            .style(theme::block_chip(part.learned))
            .on_press(Message::ToggleBlock(id.clone(), index, block))
            .into()
    }))
    .spacing(6)
    .wrap()
    .vertical_spacing(6);
    container(column![heading, blocks].spacing(10))
        .padding([12, 14])
        .width(Length::Fill)
        .clip(true)
        .style(theme::song_card)
        .into()
}
