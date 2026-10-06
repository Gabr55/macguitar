//! Durations and intervals, as the interface writes them.

use crate::parser::model::{MeasureHeader, QUARTER_TIME};

/// Seconds elapsed from the song's start up to (but not including) `measure_idx`.
/// Tempo changes across measures are honored. Repeats are ignored — we compute
/// the song's linear duration, not expanded playback time.
pub(super) fn song_time_up_to_measure(headers: &[MeasureHeader], measure_idx: usize) -> f32 {
    headers
        .iter()
        .take(measure_idx)
        .enumerate()
        .map(|(i, h)| {
            let next_start = headers
                .get(i + 1)
                .map_or(h.start + h.length(), |next| next.start);
            let duration_ticks = next_start.saturating_sub(h.start) as f32;
            duration_ticks / QUARTER_TIME as f32 * 60.0 / h.tempo.value as f32
        })
        .sum()
}

/// Semitones with their sign: "+2", "−1".
pub(super) fn format_semitones(semitones: i32) -> String {
    if semitones < 0 {
        format!("\u{2212}{}", semitones.unsigned_abs())
    } else {
        format!("+{semitones}")
    }
}

pub(super) fn format_mmss(seconds: f32) -> String {
    let total = seconds.max(0.0) as u32;
    format!("{}:{:02}", total / 60, total % 60)
}
