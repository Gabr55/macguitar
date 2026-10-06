//! Builds the MIDI events of a song, effects included.

mod builder;
mod effects;
#[cfg(test)]
mod tests;

pub use builder::{METRONOME_KEYS, METRONOME_TRACK, METRONOME_VELOCITY, MidiBuilder};
