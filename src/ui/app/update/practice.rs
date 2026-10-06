//! Practice aids: tempo, metronome, count-in, retuning and loops.

use iced::Task;

use crate::ui::app::message::{Message, TempoSelection};
use crate::ui::app::{App, TouchGesture, zoom_step};
use crate::ui::tablature::set_touch_looping;

/// How long a finger stays down to hold rather than tap.
const TOUCH_HOLD: std::time::Duration = std::time::Duration::from_millis(450);

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

    /// A finger touched `beat` of `measure`: what it does is known when it
    /// moves, stays down or is lifted.
    pub(super) fn touch_down(&mut self, measure: usize, beat: usize) -> Task<Message> {
        let id = self.touch.map_or(0, |touch| touch.id.wrapping_add(1));
        self.touch = Some(TouchGesture {
            id,
            measure,
            beat,
            scrolled: false,
            held: false,
        });
        Task::perform(tokio::time::sleep(TOUCH_HOLD), move |()| {
            Message::TouchHeld(id)
        })
    }

    /// The finger moved away before holding: a scroll, which changes nothing.
    pub(super) fn touch_scrolled(&mut self) -> Task<Message> {
        if let Some(touch) = &mut self.touch
            && !touch.held
        {
            touch.scrolled = true;
        }
        Task::none()
    }

    /// The finger stayed on its measure: it loops it, and draws the loop
    /// on as it moves. On a loop already there, letting go clears it.
    pub(super) fn touch_held(&mut self, id: u64) -> Task<Message> {
        let Some(touch) = &mut self.touch else {
            return Task::none();
        };
        if touch.id != id || touch.scrolled || touch.held {
            return Task::none();
        }
        touch.held = true;
        let measure = touch.measure;
        set_touch_looping(true);
        let in_loop = self
            .loop_range
            .is_some_and(|(first, last)| (first..=last).contains(&measure));
        if in_loop {
            self.loop_anchor = Some((measure, false));
        } else {
            // shown at once, kept when let go
            self.set_loop(Some((measure, measure)));
            self.loop_anchor = Some((measure, true));
        }
        Task::none()
    }

    /// The finger was lifted: a tap places the playhead, a hold ends its loop.
    pub(super) fn touch_ended(&mut self) -> Task<Message> {
        let Some(touch) = self.touch.take() else {
            return Task::none();
        };
        if touch.held {
            set_touch_looping(false);
            self.finish_loop()
        } else if touch.scrolled {
            Task::none()
        } else {
            self.seek_to_beat(touch.measure, touch.beat)
        }
    }

    /// Draw the tablature a step larger or smaller, `direction` being 1 or
    /// -1; kept for the next songs and launches.
    pub(super) fn zoom(&mut self, direction: i32) -> Task<Message> {
        let zoom = zoom_step(self.zoom, direction);
        if (zoom - self.zoom).abs() < f32::EPSILON {
            return Task::none();
        }
        self.zoom = zoom;
        if let Some(tablature) = &mut self.tablature {
            tablature.set_zoom(zoom);
        }
        if let Err(err) = self.config.set_zoom(zoom) {
            log::warn!("Could not save the zoom: {err}");
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
