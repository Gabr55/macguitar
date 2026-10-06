//! Playing a song: its MIDI events, the order repeats play in, and the
//! player on the audio output.

pub mod midi_builder;
pub mod midi_event;
pub mod midi_player;
mod midi_player_params;
pub mod midi_sequencer;
pub mod playback_order;
