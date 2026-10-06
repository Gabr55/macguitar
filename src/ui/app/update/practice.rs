//! Practice aids: tempo, metronome, count-in, retuning and loops.

use iced::Task;

use crate::ui::app::App;
use crate::ui::app::message::{Message, TempoSelection};

impl App {
    pub(super) fn set_tempo(&mut self, tempo: TempoSelection) -> Task<Message> {
        if self.is_loading {
            return Task::none();
        }
        self.playback.tempo = tempo;
        self.with_player(|player| player.set_tempo_percentage(tempo.percentage));
        Task::none()
    }

    pub(super) fn toggle_metronome(&mut self) -> Task<Message> {
        self.playback.metronome = !self.playback.metronome;
        let enabled = self.playback.metronome;
        self.with_player(|player| player.set_metronome(enabled));
        Task::none()
    }

    pub(super) fn toggle_count_in(&mut self) -> Task<Message> {
        self.playback.count_in = !self.playback.count_in;
        let enabled = self.playback.count_in;
        self.with_player(|player| player.set_count_in(enabled));
        Task::none()
    }

    /// Retune the whole song by `step` semitones from where it is.
    pub(super) fn retune(&mut self, step: i32) -> Task<Message> {
        self.playback.retune(step);
        let semitones = self.playback.transpose;
        self.with_player(|player| player.set_transpose(semitones));
        Task::none()
    }

    /// The right button went down on `measure`. Nothing changes until the
    /// pointer moves or is let go: a click and a drag do different things.
    pub(super) fn start_loop(&mut self, measure: usize) -> Task<Message> {
        self.loop_anchor = Some((measure, false));
        Task::none()
    }

    /// The pointer entered `measure` while drawing a loop.
    pub(super) fn extend_loop(&mut self, measure: usize) -> Task<Message> {
        if let Some((anchor, dragged)) = &mut self.loop_anchor
            && (*dragged || measure != *anchor)
        {
            *dragged = true;
            let anchor = *anchor;
            self.set_loop(Some((anchor.min(measure), anchor.max(measure))));
        }
        Task::none()
    }

    /// The right button was let go. A click without a drag clears the loop
    /// it lands on, or loops its measure.
    pub(super) fn finish_loop(&mut self) -> Task<Message> {
        if let Some((measure, false)) = self.loop_anchor.take() {
            let in_loop = self
                .loop_range
                .is_some_and(|(first, last)| (first..=last).contains(&measure));
            self.set_loop((!in_loop).then_some((measure, measure)));
        }
        Task::none()
    }

    /// Stop looping, or loop the measure being played.
    pub(super) fn toggle_loop(&mut self) -> Task<Message> {
        let range = if self.loop_range.is_some() {
            None
        } else {
            self.tablature.as_ref().map(|tablature| {
                let measure = tablature.focused_measure();
                (measure, measure)
            })
        };
        self.set_loop(range);
        Task::none()
    }

    /// Loop the measures `first..=last`, or stop looping with `None`.
    fn set_loop(&mut self, range: Option<(usize, usize)>) {
        self.loop_range = range;
        if let Some(tablature) = &mut self.tablature {
            tablature.set_loop(range);
        }
        self.with_player(|player| player.set_loop(range));
    }
}
