//! The track menu, with the solo and mute switches and the volume of
//! every track.

use iced::advanced::text::Shaping;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, slider, text};
use iced::{Alignment, Element, Length};

use crate::audio::midi_player::AudioPlayer;
use crate::ui::app::App;
use crate::ui::app::message::Message;
use crate::ui::icons::Icon;
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::toggle_button;

impl App {
    /// The tracks, each with its solo and mute switches.
    pub(super) fn track_menu(
        &self,
        anchor: iced::Rectangle,
        tokens: Tokens,
    ) -> Element<'_, Message> {
        let solo_track = self
            .audio_player
            .as_ref()
            .and_then(AudioPlayer::solo_track_id);
        let rows = self.all_tracks.iter().map(|track| {
            let selected = track.index == self.track_selection.index;
            let muted_track = self
                .audio_player
                .as_ref()
                .is_some_and(|p| p.is_track_muted(track.index));
            let volume = self
                .audio_player
                .as_ref()
                .map_or(1.0, |p| p.track_volume(track.index));
            let index = track.index;
            row![
                // a long name is cut at the entry's edge, clear of the switches
                button(
                    container(
                        text(track.to_string())
                            .size(13)
                            .shaping(Shaping::Advanced)
                            .wrapping(Wrapping::None)
                    )
                    .width(Length::Fill)
                    .clip(true)
                )
                .width(Length::Fill)
                .padding([7, 10])
                .style(theme::menu_item(selected))
                .on_press(Message::TrackSelected(track.clone())),
                toggle_button(
                    Icon::Solo,
                    "Solo",
                    Message::SoloTrack(track.index),
                    solo_track == Some(track.index),
                    tokens
                ),
                toggle_button(
                    Icon::Mute,
                    "Mute",
                    Message::MuteTrack(track.index),
                    muted_track,
                    tokens
                ),
                // the volume written in the file stays the reference: this
                // turns the track down from it
                slider(0.0..=1.0, volume, move |volume| {
                    Message::TrackVolumeChanged(index, volume)
                })
                .step(0.01_f32)
                .width(90)
                .style(theme::slider_style),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        });
        container(
            scrollable(column(rows).spacing(2).padding(iced::Padding {
                right: 8.0,
                ..iced::Padding::ZERO
            }))
            .style(theme::scrollable_style),
        )
        .padding(6)
        .width(anchor.width.max(300.0) + 180.0)
        .max_height(420)
        .style(theme::menu_panel)
        .into()
    }
}
