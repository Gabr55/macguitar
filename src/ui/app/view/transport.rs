//! The transport: playback, position, tempo, tuning and practice aids.

use iced::widget::space::Space;
use iced::widget::{column, container, pick_list, progress_bar, row, slider, text, tooltip};
use iced::{Alignment, Element, Length};

use crate::ui::app::App;
use crate::ui::app::format::{format_mmss, format_semitones, song_time_up_to_measure};
use crate::ui::app::message::{Message, TempoSelection};
use crate::ui::app::playback::MAX_TRANSPOSE;
use crate::ui::icons::{Icon, icon};
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::{
    UI_FONT_BOLD, divider, icon_button, muted, play_button, toggle_button, with_tooltip,
};

impl App {
    /// Playback controls, once a song is loaded.
    pub(super) fn transport(&self, tokens: Tokens) -> Option<Element<'_, Message>> {
        let audio_player = self.audio_player.as_ref()?;
        let tablature = self.tablature.as_ref()?;

        let headers = &tablature.song.measure_headers;
        let focused = tablature.focused_measure();
        let total_measures = tablature.measure_count();
        let current_seconds = song_time_up_to_measure(headers, focused);
        let total_seconds = song_time_up_to_measure(headers, total_measures);

        let buttons = row![
            icon_button(
                Icon::Previous,
                "Previous measure (\u{2190})",
                (focused > 0).then_some(Message::PreviousMeasure),
                tokens
            ),
            play_button(audio_player.is_playing(), tokens),
            icon_button(
                Icon::Next,
                "Next measure (\u{2192})",
                (focused + 1 < total_measures).then_some(Message::NextMeasure),
                tokens
            ),
            icon_button(Icon::Stop, "Stop", Some(Message::StopPlayer), tokens),
            toggle_button(
                Icon::Loop,
                "Loop (L) \u{2014} drag over measures with the right button",
                Message::ToggleLoop,
                self.loop_range.is_some(),
                tokens
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let position = column![
            row![
                text(format!("Measure {}", focused + 1))
                    .size(12)
                    .font(UI_FONT_BOLD),
                muted(format!(" / {total_measures}")).size(12),
                Space::new().width(Length::Fill),
                text(format_mmss(current_seconds))
                    .size(12)
                    .font(UI_FONT_BOLD),
                muted(format!(" / {}", format_mmss(total_seconds))).size(12),
            ],
            progress_bar(0.0..=total_seconds.max(f32::EPSILON), current_seconds)
                .girth(4)
                .style(theme::progress),
        ]
        .spacing(7)
        .width(Length::Fill);

        let tempo = row![
            with_tooltip(
                icon(Icon::Tempo, 17.0, tokens.muted),
                "Tempo (Ctrl \u{2191}/\u{2193})",
                tooltip::Position::Top,
            ),
            pick_list(
                TempoSelection::PRESET,
                Some(&self.playback.tempo),
                Message::TempoSelected,
            )
            .text_size(13)
            .padding([6, 10])
            .width(86)
            .style(theme::pick_list_style)
            .menu_style(theme::menu),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // each arrow says what it does: no tooltip over the whole stepper,
        // it would stack on theirs
        let transpose = row![
            icon_button(
                Icon::Down,
                "Retune the song a semitone down",
                (self.playback.transpose > -MAX_TRANSPOSE).then_some(Message::Transpose(-1)),
                tokens
            ),
            container(
                text(if self.playback.transpose == 0 {
                    "0".to_string()
                } else {
                    format_semitones(self.playback.transpose)
                })
                .size(13)
                .font(UI_FONT_BOLD)
            )
            .width(30)
            .align_x(Alignment::Center),
            icon_button(
                Icon::Up,
                "Retune the song a semitone up",
                (self.playback.transpose < MAX_TRANSPOSE).then_some(Message::Transpose(1)),
                tokens
            ),
        ]
        .align_y(Alignment::Center);

        let practice = row![
            toggle_button(
                Icon::Metronome,
                "Metronome",
                Message::ToggleMetronome,
                self.playback.metronome,
                tokens
            ),
            toggle_button(
                Icon::CountIn,
                "Count-in",
                Message::ToggleCountIn,
                self.playback.count_in,
                tokens
            ),
        ]
        .spacing(4);

        let volume = row![
            icon(Icon::Volume, 18.0, tokens.muted),
            slider(
                0.0..=1.0,
                audio_player.master_volume(),
                Message::MasterVolumeChanged
            )
            .step(0.01_f32)
            .width(110)
            .style(theme::slider_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let transport = row![
            buttons,
            divider(),
            position,
            divider(),
            tempo,
            transpose,
            practice,
            divider(),
            volume,
        ]
        .spacing(14)
        .align_y(Alignment::Center);

        Some(
            container(transport)
                .padding([10, 16])
                .style(theme::panel)
                .into(),
        )
    }
}
