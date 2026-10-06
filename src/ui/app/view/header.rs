//! The header: the song, the track and sound menus, the file actions.

use iced::advanced::text::Shaping;
use iced::widget::text::Wrapping;
use iced::widget::{Id, button, column, container, mouse_area, pick_list, row, text, tooltip};
use iced::{Alignment, Element, Length};

use crate::ui::app::message::Menu;
use crate::ui::app::message::{Message, SoundChoice};
use crate::ui::app::{App, TITLE_BAR_INSET};
use crate::ui::icons::{Icon, icon, logo};
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::{
    UI_FONT_BOLD, divider, icon_button, muted, open_button, toggle_button, with_tooltip,
};

impl App {
    /// Song title, track selection and the file actions.
    pub(super) fn header(&self, tokens: Tokens) -> Element<'_, Message> {
        let (title, subtitle) = match &self.song_info {
            Some(song) => {
                let title = if song.name.trim().is_empty() {
                    song.file_name.clone()
                } else {
                    song.name.clone()
                };
                // metadata may hold line breaks: keep it to one line
                let subtitle = [Some(song.artist.clone()), song.metadata_line()]
                    .into_iter()
                    .flatten()
                    .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("  \u{00b7}  ");
                (title, subtitle)
            }
            None => ("MacGuitar".to_string(), "No tab loaded".to_string()),
        };
        let mut title_block = column![
            text(title)
                .size(15)
                .font(UI_FONT_BOLD)
                .shaping(Shaping::Advanced)
                .wrapping(Wrapping::None),
        ]
        .spacing(2);
        // a song without details keeps its title centred
        if !subtitle.is_empty() {
            title_block = title_block.push(
                muted(subtitle)
                    .size(12)
                    .shaping(Shaping::Advanced)
                    .wrapping(Wrapping::None),
            );
        }

        let mut header = row![
            logo(30.0),
            container(title_block).width(Length::Fill).clip(true)
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        if let Some(song) = &self.song_info {
            header = header.push(
                container(
                    text(format!("{:?}", song.gp_version))
                        .size(11)
                        .font(UI_FONT_BOLD),
                )
                .padding([3, 7])
                .style(theme::badge),
            );
        }

        if !self.all_tracks.is_empty() {
            // a menu of its own rather than a list: each track carries its
            // solo and mute switches
            let track_menu_button = container(
                button(
                    row![
                        container(
                            text(self.track_selection.to_string())
                                .size(13)
                                .shaping(Shaping::Advanced)
                                .wrapping(Wrapping::None)
                        )
                        .width(Length::Fill)
                        .clip(true),
                        icon(Icon::Down, 15.0, tokens.muted),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .width(250)
                .padding([7, 12])
                .style(theme::select_button)
                .on_press(
                    if self.menu.is_some_and(|(menu, _)| menu == Menu::Tracks) {
                        Message::CloseMenu
                    } else {
                        Message::OpenMenu(Menu::Tracks)
                    },
                ),
            )
            .id(Id::new(Menu::Tracks.anchor_id()));

            let solo = self
                .audio_player
                .as_ref()
                .is_some_and(|p| p.solo_track_id() == Some(self.track_selection.index));
            let muted_track = self
                .audio_player
                .as_ref()
                .is_some_and(|p| p.is_track_muted(self.track_selection.index));

            header = header.push(divider()).push(
                row![
                    track_menu_button,
                    toggle_button(Icon::Solo, "Solo (S)", Message::ToggleSolo, solo, tokens),
                    toggle_button(
                        Icon::Mute,
                        "Mute (M)",
                        Message::ToggleMute,
                        muted_track,
                        tokens
                    ),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }

        let current_sound = self
            .sound_font_file
            .clone()
            .map_or(SoundChoice::BuiltIn, SoundChoice::File);
        let mut sounds = vec![SoundChoice::BuiltIn];
        sounds.extend(
            self.installed_sound_fonts
                .iter()
                .cloned()
                .map(SoundChoice::File),
        );
        if !sounds.contains(&current_sound) {
            sounds.push(current_sound.clone());
        }
        sounds.push(SoundChoice::Browse);
        // tooltips stay off the lists: they would cover the open menu
        let sound = row![
            with_tooltip(
                icon(Icon::Sound, 17.0, tokens.muted),
                "SoundFont the song plays with",
                tooltip::Position::Bottom,
            ),
            pick_list(sounds, Some(current_sound), Message::SoundSelected)
                .text_size(13)
                .padding([7, 12])
                .width(160)
                .style(theme::pick_list_style)
                .menu_style(theme::menu),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        header = header.push(divider()).push(sound).push(divider()).push(
            row![
                open_button("Open", !self.is_loading, tokens),
                self.recent_files_button(tokens),
                icon_button(
                    Icon::Fullscreen,
                    "Fullscreen (\u{2303}\u{2318}F)",
                    Some(Message::ToggleFullscreen),
                    tokens
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );

        let header = container(header).padding([10, 14]).style(theme::panel);
        if TITLE_BAR_INSET > 0.0 {
            mouse_area(header)
                .on_press(Message::DragWindow)
                .on_double_click(Message::ToggleMaximize)
                .into()
        } else {
            header.into()
        }
    }

    /// Opens the recent files menu; nothing when no file was opened yet.
    fn recent_files_button(&self, tokens: Tokens) -> Element<'_, Message> {
        if self.recent_files.is_empty() {
            return iced::widget::Space::new().into();
        }
        let is_open = self.menu.is_some_and(|(menu, _)| menu == Menu::RecentFiles);
        container(icon_button(
            Icon::Recent,
            "Recent files",
            Some(if is_open {
                Message::CloseMenu
            } else {
                Message::OpenMenu(Menu::RecentFiles)
            }),
            tokens,
        ))
        .id(Id::new(Menu::RecentFiles.anchor_id()))
        .into()
    }
}
