//! Choosing the track shown, and how each track sounds.

use iced::Task;
use iced::widget::operation::scroll_to;
use iced::widget::scrollable::AbsoluteOffset;

use crate::ui::app::App;
use crate::ui::app::message::{Message, TrackSelection};

impl App {
    pub(super) fn select_track(&mut self, track: TrackSelection) -> Task<Message> {
        self.menu = None;
        let scroll = self.tablature.as_mut().and_then(|tablature| {
            tablature
                .update_track(track.index)
                .map(|y| scroll_to(tablature.scroll_id.clone(), AbsoluteOffset { x: 0.0, y }))
        });
        self.track_selection = track;
        scroll.unwrap_or_else(Task::none)
    }

    pub(super) fn toggle_solo(&self, track: usize) -> Task<Message> {
        self.with_player(|player| player.toggle_solo_mode(track));
        Task::none()
    }

    pub(super) fn toggle_mute(&self, track: usize) -> Task<Message> {
        self.with_player(|player| player.toggle_track_mute(track));
        Task::none()
    }
}
