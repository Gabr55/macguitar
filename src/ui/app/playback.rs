//! Playback settings that outlive any one song.

use crate::audio::midi_player::AudioPlayer;
use crate::ui::app::message::TempoSelection;

/// How far the song can be retuned, in semitones either way.
pub(super) const MAX_TRANSPOSE: i32 = 12;

/// What the player is set to, carried over to the player of the next song.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PlaybackSettings {
    pub(super) tempo: TempoSelection,
    pub(super) metronome: bool,
    pub(super) count_in: bool,
    /// Semitones the whole song is retuned by, for practice in another
    /// tuning.
    pub(super) transpose: i32,
}

impl PlaybackSettings {
    /// Set a new player as the previous one was.
    pub(super) fn apply_to(&self, player: &AudioPlayer) {
        player.set_tempo_percentage(self.tempo.percentage);
        player.set_metronome(self.metronome);
        player.set_count_in(self.count_in);
        player.set_transpose(self.transpose);
    }

    /// Retune by `step` semitones, within `MAX_TRANSPOSE` either way.
    pub(super) fn retune(&mut self, step: i32) {
        self.transpose = (self.transpose + step).clamp(-MAX_TRANSPOSE, MAX_TRANSPOSE);
    }
}
