//! Playback settings shared without locks between the interface and the
//! audio thread: tempo, solo and mute, volumes, transposition, loops.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

const SOLO_NONE: i32 = -1;

/// Tracks with their own mute bit and volume.
const MAX_TRACKS: usize = 64;

/// Playback parameters shared lock-free between UI and audio callback.
pub struct MidiPlayerParams {
    tempo: AtomicU32,
    tempo_percentage: AtomicU32,
    solo_track_id: AtomicI32,               // -1 == None
    mute_mask: AtomicU64,                   // bit per muted track id
    track_volumes: [AtomicU32; MAX_TRACKS], // f32 bits per track, 1.0 as written
    metronome: AtomicBool,
    count_in: AtomicBool,
    finished: AtomicBool, // the sequence ran past its last event
    // pending count-in request: total ticks (high) | beat ticks (low), 0 = none
    count_in_request: AtomicU64,
    master_volume: AtomicU32,       // f32 bits
    transpose: AtomicI32,           // semitones added to pitched notes
    percussion_channels: AtomicU32, // bit per MIDI channel playing drums
    // playback ticks looped over: start (high) | end (low), 0 = no loop
    loop_range: AtomicU64,
}

impl MidiPlayerParams {
    pub fn new(tempo: u32, tempo_percentage: u32, solo_track_id: Option<usize>) -> Self {
        Self {
            tempo: AtomicU32::new(tempo),
            tempo_percentage: AtomicU32::new(tempo_percentage),
            solo_track_id: AtomicI32::new(solo_track_id.map_or(SOLO_NONE, |id| id as i32)),
            mute_mask: AtomicU64::new(0),
            track_volumes: std::array::from_fn(|_| AtomicU32::new(1.0_f32.to_bits())),
            metronome: AtomicBool::new(false),
            count_in: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            count_in_request: AtomicU64::new(0),
            master_volume: AtomicU32::new(1.0_f32.to_bits()),
            transpose: AtomicI32::new(0),
            // channel 10 (9 from zero) is the General MIDI drum kit
            percussion_channels: AtomicU32::new(1 << 9),
            loop_range: AtomicU64::new(0),
        }
    }

    /// Volume of a track, from `0.0` (silent) to `1.0` (as written).
    pub fn track_volume(&self, track_id: usize) -> f32 {
        self.track_volumes
            .get(track_id)
            .map_or(1.0, |volume| f32::from_bits(volume.load(Ordering::Relaxed)))
    }

    pub fn set_track_volume(&self, track_id: usize, volume: f32) {
        if let Some(slot) = self.track_volumes.get(track_id) {
            slot.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        }
    }

    /// The velocity a note of `track_id` sounds with, or `None` when its
    /// track is turned all the way down.
    pub fn track_velocity(&self, track_id: Option<u8>, velocity: i16) -> Option<i32> {
        let volume = track_id.map_or(1.0, |track| self.track_volume(usize::from(track)));
        let scaled = (f32::from(velocity) * volume).round() as i32;
        (scaled > 0).then_some(scaled.min(127))
    }

    /// Semitones the pitched notes are shifted by, as if the instrument
    /// were tuned down (negative) or up (positive).
    pub fn transpose(&self) -> i32 {
        self.transpose.load(Ordering::Relaxed)
    }

    pub fn set_transpose(&self, semitones: i32) {
        self.transpose.store(semitones, Ordering::Relaxed);
    }

    /// The playback ticks `start..end` played over and over, or `None`.
    pub fn loop_range(&self) -> Option<(u32, u32)> {
        let packed = self.loop_range.load(Ordering::Relaxed);
        (packed != 0).then_some(((packed >> 32) as u32, packed as u32))
    }

    pub fn set_loop_range(&self, range: Option<(u32, u32)>) {
        let packed = range
            .filter(|(start, end)| start < end)
            .map_or(0, |(start, end)| (u64::from(start) << 32) | u64::from(end));
        self.loop_range.store(packed, Ordering::Relaxed);
    }

    /// Mark a MIDI channel as playing drums, which are never transposed.
    pub fn add_percussion_channel(&self, channel: u8) {
        if channel < 16 {
            self.percussion_channels
                .fetch_or(1 << channel, Ordering::Relaxed);
        }
    }

    /// The key to play for `key` on `channel`, with the transposition.
    pub fn transposed_key(&self, channel: i32, key: i32) -> i32 {
        let is_percussion = (0..16).contains(&channel)
            && self.percussion_channels.load(Ordering::Relaxed) & (1 << channel) != 0;
        if is_percussion {
            key
        } else {
            (key + self.transpose()).clamp(0, 127)
        }
    }

    pub fn metronome_enabled(&self) -> bool {
        self.metronome.load(Ordering::Relaxed)
    }

    pub fn set_metronome(&self, enabled: bool) {
        self.metronome.store(enabled, Ordering::Relaxed);
    }

    /// Whether playback has run past the last event of the sequence.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    pub fn set_finished(&self, finished: bool) {
        self.finished.store(finished, Ordering::Relaxed);
    }

    pub fn count_in_enabled(&self) -> bool {
        self.count_in.load(Ordering::Relaxed)
    }

    pub fn set_count_in(&self, enabled: bool) {
        self.count_in.store(enabled, Ordering::Relaxed);
    }

    /// Ask the audio callback to click through a measure before playing.
    pub fn request_count_in(&self, total_ticks: u32, beat_ticks: u32) {
        let packed = (u64::from(total_ticks) << 32) | u64::from(beat_ticks);
        self.count_in_request.store(packed, Ordering::Relaxed);
    }

    /// Take the pending count-in request, if any: `(total ticks, beat ticks)`.
    pub fn take_count_in_request(&self) -> Option<(u32, u32)> {
        match self.count_in_request.swap(0, Ordering::Relaxed) {
            0 => None,
            packed => Some(((packed >> 32) as u32, packed as u32)),
        }
    }

    pub fn toggle_track_mute(&self, track_id: usize) {
        if track_id < 64 {
            self.mute_mask.fetch_xor(1 << track_id, Ordering::Relaxed);
        }
    }

    pub fn is_track_muted(&self, track_id: usize) -> bool {
        track_id < 64 && self.mute_mask.load(Ordering::Relaxed) & (1 << track_id) != 0
    }

    /// Like TuxGuitar's `shouldSend`: mute wins, then solo excludes the rest.
    pub fn is_track_audible(&self, track_id: usize) -> bool {
        if self.is_track_muted(track_id) {
            return false;
        }
        match self.solo_track_id() {
            Some(solo_track_id) => solo_track_id == track_id,
            None => true,
        }
    }

    pub fn master_volume(&self) -> f32 {
        f32::from_bits(self.master_volume.load(Ordering::Relaxed))
    }

    pub fn set_master_volume(&self, volume: f32) {
        self.master_volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn solo_track_id(&self) -> Option<usize> {
        match self.solo_track_id.load(Ordering::Relaxed) {
            SOLO_NONE => None,
            id => Some(id as usize),
        }
    }

    pub fn set_solo_track_id(&self, solo_track_id: Option<usize>) {
        self.solo_track_id.store(
            solo_track_id.map_or(SOLO_NONE, |id| id as i32),
            Ordering::Relaxed,
        );
    }

    pub fn adjusted_tempo(&self) -> u32 {
        let tempo = self.tempo.load(Ordering::Relaxed);
        let pct = self.tempo_percentage.load(Ordering::Relaxed);
        // clamp to 1 BPM: at tempo 0 the sequencer would never advance again,
        // freezing playback with no way to reach the next tempo change event
        ((tempo as f32 * pct as f32 / 100.0) as u32).max(1)
    }

    pub fn set_tempo(&self, tempo: u32) {
        self.tempo.store(tempo, Ordering::Relaxed);
    }

    pub fn set_tempo_percentage(&self, tempo_percentage: u32) {
        self.tempo_percentage
            .store(tempo_percentage, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod transpose_tests {
    use super::MidiPlayerParams;

    #[test]
    fn transposes_pitched_channels_only() {
        let params = MidiPlayerParams::new(120, 100, None);
        params.add_percussion_channel(3);
        params.set_transpose(-2);
        assert_eq!(params.transposed_key(0, 64), 62);
        // the General MIDI kit and the song's own drum channels keep their keys
        assert_eq!(params.transposed_key(9, 38), 38);
        assert_eq!(params.transposed_key(3, 38), 38);
    }

    #[test]
    fn transposed_keys_stay_in_the_midi_range() {
        let params = MidiPlayerParams::new(120, 100, None);
        params.set_transpose(12);
        assert_eq!(params.transposed_key(0, 120), 127);
        params.set_transpose(-12);
        assert_eq!(params.transposed_key(0, 5), 0);
    }
}

#[cfg(test)]
mod track_volume_tests {
    use super::MidiPlayerParams;

    #[test]
    fn a_track_volume_scales_its_notes() {
        let params = MidiPlayerParams::new(120, 100, None);
        assert_eq!(params.track_velocity(Some(2), 100), Some(100));
        params.set_track_volume(2, 0.5);
        assert_eq!(params.track_velocity(Some(2), 100), Some(50));
        // other tracks, and events of no track, keep theirs
        assert_eq!(params.track_velocity(Some(1), 100), Some(100));
        assert_eq!(params.track_velocity(None, 100), Some(100));
        params.set_track_volume(2, 0.0);
        assert_eq!(params.track_velocity(Some(2), 100), None);
    }
}
