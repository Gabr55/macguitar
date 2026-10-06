//! The tuning of the track, shown above the tablature.

use iced::widget::{container, row, text};
use iced::{Alignment, Element};

use crate::ui::app::App;
use crate::ui::app::format::format_semitones;
use crate::ui::app::message::Message;
use crate::ui::theme::{self};
use crate::ui::tuning::{string_notes, tuning_name};
use crate::ui::widgets::{UI_FONT_BOLD, muted};

impl App {
    /// The tuning of the selected track, above its tablature, as it sounds
    /// with the song retuned.
    pub(super) fn tuning_line(&self) -> Option<Element<'_, Message>> {
        let track = &self.track_selection;
        if track.strings.is_empty() {
            return None;
        }
        let sounding: Vec<(i32, i32)> = track
            .strings
            .iter()
            .map(|&(string, pitch)| (string, pitch + self.playback.transpose))
            .collect();
        let mut line = row![muted("Tuning").size(12)]
            .spacing(6)
            .align_y(Alignment::Center);
        // a named tuning reads first; an unnamed one is just its notes
        if let Some(name) = tuning_name(&sounding) {
            line = line.push(text(name).size(12).font(UI_FONT_BOLD));
        }
        for note in string_notes(&sounding) {
            line = line.push(
                container(text(note).size(11).font(UI_FONT_BOLD))
                    .padding([1, 6])
                    .style(theme::kbd),
            );
        }
        if self.playback.transpose != 0 {
            let written = tuning_name(&track.strings).map_or_else(
                || string_notes(&track.strings).join(" "),
                ToString::to_string,
            );
            line = line.push(
                muted(format!(
                    "{} from {written}",
                    format_semitones(self.playback.transpose)
                ))
                .size(12),
            );
        }
        Some(container(line).padding([6, 10]).into())
    }
}
