//! Playing, stopping and moving through the song.

use iced::Task;
use iced::widget::operation::scroll_to;
use iced::widget::scrollable::AbsoluteOffset;

use crate::audio::midi_player::AudioPlayer;
use crate::ui::app::App;
use crate::ui::app::message::Message;

impl App {
    pub(super) fn play_pause(&mut self) -> Task<Message> {
        if self.is_loading {
            return Task::none();
        }
        if let Some(player) = &mut self.audio_player
            && let Some(err) = player.toggle_play()
        {
            return Task::done(Message::ReportError(err));
        }
        // the first playback can start before the tablature was ever laid
        // out (no resize happened yet): measure it now
        Task::done(Message::WindowResized)
    }

    /// Stop, and go back to the start of the song.
    pub(super) fn stop(&mut self) -> Task<Message> {
        let (Some(player), Some(tablature)) = (&mut self.audio_player, &mut self.tablature) else {
            return Task::none();
        };
        player.stop();
        tablature.focus_on_measure(0);
        scroll_to(
            tablature.scroll_id.clone(),
            AbsoluteOffset::<f32>::default(),
        )
    }

    /// Move the cursor with the audio, turning the page when needed.
    pub(super) fn follow_playback(&mut self, tick: u32) -> Task<Message> {
        // the song ran out: stop rather than play silence
        if self
            .audio_player
            .as_ref()
            .is_some_and(AudioPlayer::is_finished)
        {
            return Task::done(Message::StopPlayer);
        }
        let Some(tablature) = &mut self.tablature else {
            return Task::none();
        };
        tablature.focus_on_tick(tick);
        // the page holds still while it is read, and turns as playback
        // reaches its last line
        let focused = tablature.focused_measure();
        tablature
            .page_scroll_offset(focused)
            .map_or_else(Task::none, |y| {
                scroll_to(tablature.scroll_id.clone(), AbsoluteOffset { x: 0.0, y })
            })
    }

    /// Place the playhead on a beat clicked in the tablature.
    pub(super) fn seek_to_beat(&mut self, measure: usize, beat: usize) -> Task<Message> {
        if let Some(tablature) = &mut self.tablature {
            tablature.focus_on_measure_beat(measure, beat);
            let offset = tablature.beat_tick_offset(measure, beat);
            self.with_player(|player| player.focus_measure_at(measure, offset));
        }
        Task::none()
    }

    /// Move the playhead `step` measures forward (or back).
    pub(super) fn step_measure(&mut self, step: isize) -> Task<Message> {
        let target = self.tablature.as_ref().and_then(|tablature| {
            tablature
                .focused_measure()
                .checked_add_signed(step)
                .filter(|&measure| measure < tablature.measure_count())
        });
        target.map_or_else(Task::none, |measure| self.seek_to_measure(measure))
    }

    /// Place the playhead at the start of `measure`, scrolling to it.
    fn seek_to_measure(&mut self, measure: usize) -> Task<Message> {
        let Some(tablature) = &mut self.tablature else {
            return Task::none();
        };
        tablature.focus_on_measure(measure);
        let scroll = tablature
            .page_scroll_offset(measure)
            .map(|y| scroll_to(tablature.scroll_id.clone(), AbsoluteOffset { x: 0.0, y }));
        self.with_player(|player| player.focus_measure_at(measure, 0));
        scroll.unwrap_or_else(Task::none)
    }
}
