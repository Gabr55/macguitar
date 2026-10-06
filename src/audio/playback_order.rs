//! The order measures play in, repeats and alternative endings unrolled.

use crate::parser::model::{DirectionJump, DirectionTarget, MeasureHeader, QUARTER_TIME};

/// Compute the playback order of measures, expanding repeats and alternative endings.
///
/// Used by the MIDI builder to generate events at the correct ticks,
/// and by the tablature to map playback ticks back to visual measures.
/// Returns a Vec of (measure_index, tick_offset) pairs.
/// The tick_offset is the difference between the playback tick and the original measure tick.
///
/// Port of TuxGuitar's `MidiRepeatController` semantics:
/// - the first measure implicitly opens a repeat section
/// - a repeat_open marker restarts the section (Guitar Pro has no nested repeats)
///   and resets the counters only on its first pass
/// - the alternative ending bitmask latches from the first marked measure until
///   a repeat_close, so unmarked measures under a volta bracket inherit it
/// - a repeat_close on a skipped measure only clears the latch, it never jumps
/// - a repeat_close inside an alternative ending always jumps back; the section
///   ends by falling through an ending without a repeat_close
///
/// Then the jumps of the score, as Guitar Pro plays them:
/// - a D.C. or D.S. is taken at the end of its measure, its repeats done
/// - on the way back the repeats are played through once, a volta bracket
///   by its last ending
/// - a To Coda counts only on the way back from a jump "al Coda"; the coda
///   then plays with its repeats, and a jump "al Fine" stops at the fine
/// - every jump is taken once
pub fn compute_playback_order(headers: &[MeasureHeader]) -> Vec<(usize, i64)> {
    let mut order: Vec<(usize, i64)> = Vec::new();
    // i64: keeps the accumulator itself from overflowing on absurd repeat
    // counts; downstream event ticks remain u32 (the practical timeline limit)
    let mut running_tick: i64 = i64::from(QUARTER_TIME); // same starting tick as parser

    let mut index: usize = 0;
    let mut last_played: i64 = -1; // highest measure index played so far
    let mut repeat_start_index: usize = 0;
    let mut repeat_open = true; // first measure implicitly opens a repeat
    let mut repeat_number: i8 = 0; // 0-based repetition counter
    let mut repeat_alternative: u8 = 0; // latched alternative ending bitmask

    // the D.C. or D.S. taken, which says where the way back ends
    let mut navigation: Option<DirectionJump> = None;
    // on the way back from a D.C. or D.S. the repeats are played through
    // once, a volta bracket by its last ending, until the coda
    let mut returning = false;
    let mut final_ending: u8 = 0;
    // each jump is taken once, so the playback always ends
    let mut jumps_taken: Vec<usize> = Vec::new();

    while index < headers.len() {
        let header = &headers[index];
        let mut should_play = true;

        if header.repeat_open {
            repeat_start_index = index;
            repeat_open = true;
            // reset counters only on the first pass over this measure
            if index as i64 > last_played {
                repeat_number = 0;
                repeat_alternative = 0;
            } else if returning {
                repeat_alternative = 0;
            }
        } else {
            // latch the alternative ending bitmask from the first marked measure
            if repeat_alternative == 0 {
                repeat_alternative = header.repeat_alternative;
                if returning && repeat_alternative > 0 {
                    final_ending = last_ending(headers, index);
                }
            }
            // inside an alternative ending, the measure only plays if the
            // latched mask matches the current repetition
            let pass = if returning {
                final_ending
            } else {
                repetition_bit(repeat_number)
            };
            if (repeat_open || returning)
                && repeat_alternative > 0
                && repeat_alternative & pass == 0
            {
                should_play = false;
                // the close of a skipped ending terminates the latch but never jumps
                if header.repeat_close > 0 {
                    repeat_alternative = 0;
                }
            }
        }

        if should_play {
            last_played = last_played.max(index as i64);
            let tick_offset = running_tick - i64::from(header.start);
            order.push((index, tick_offset));
            running_tick += i64::from(header.length());

            if returning {
                // the bracket ends here, without going back
                if header.repeat_close > 0 {
                    repeat_alternative = 0;
                }
            } else if repeat_open && header.repeat_close > 0 {
                if repeat_number < header.repeat_close || repeat_alternative > 0 {
                    repeat_number += 1;
                    repeat_alternative = 0;
                    index = repeat_start_index;
                    continue;
                }
                // done repeating
                repeat_open = false;
                repeat_number = 0;
                repeat_alternative = 0;
            }

            // the way back stops at the fine
            if returning
                && navigation.and_then(DirectionJump::until) == Some(DirectionTarget::Fine)
                && header.targets.contains(&DirectionTarget::Fine)
            {
                break;
            }

            if !jumps_taken.contains(&index)
                && let Some((jump, destination)) = jump_from(headers, header, navigation, returning)
            {
                jumps_taken.push(index);
                repeat_alternative = 0;
                repeat_number = 0;
                if jump.is_to_coda() {
                    // the coda plays like the start of a song, repeats included
                    returning = false;
                    repeat_open = true;
                    repeat_start_index = destination;
                } else {
                    navigation = Some(jump);
                    returning = true;
                    repeat_open = false;
                }
                index = destination;
                continue;
            }
        }
        index += 1;
    }

    order
}

/// The jump `header` makes at its end, and the measure it lands on.
///
/// A D.C. or D.S. is taken on the first pass; a To Coda only on the way
/// back from a jump "al Coda". A jump whose mark is missing is ignored.
fn jump_from(
    headers: &[MeasureHeader],
    header: &MeasureHeader,
    navigation: Option<DirectionJump>,
    returning: bool,
) -> Option<(DirectionJump, usize)> {
    header.jumps.iter().find_map(|&jump| {
        let follow = if jump.is_to_coda() {
            returning && navigation.and_then(DirectionJump::until) == jump.destination()
        } else {
            !returning
        };
        if !follow {
            return None;
        }
        let destination = match jump.destination() {
            None => 0,
            Some(target) => headers.iter().position(|h| h.targets.contains(&target))?,
        };
        Some((jump, destination))
    })
}

/// The bit of the last ending of the volta bracket starting at `index`.
fn last_ending(headers: &[MeasureHeader], index: usize) -> u8 {
    let endings = headers[index..]
        .iter()
        .map(|h| h.repeat_alternative)
        .take_while(|&mask| mask > 0)
        .fold(0, |all, mask| all | mask);
    if endings == 0 {
        0
    } else {
        1 << (7 - endings.leading_zeros())
    }
}

/// Translate a tick from the original timeline into the expanded playback timeline.
pub fn playback_tick(original_tick: u32, tick_offset: i64) -> u32 {
    (i64::from(original_tick) + tick_offset) as u32
}

/// First playback tick of each measure, used for seeking.
///
/// Measures the playback never reaches (e.g. an alternative ending whose
/// repetition never occurs) fall back to the first playback tick of the
/// closest preceding played measure.
pub fn first_playback_ticks(headers: &[MeasureHeader], order: &[(usize, i64)]) -> Vec<u32> {
    let mut first_ticks: Vec<Option<u32>> = vec![None; headers.len()];
    for &(measure_index, tick_offset) in order {
        let slot = &mut first_ticks[measure_index];
        if slot.is_none() {
            *slot = Some(playback_tick(headers[measure_index].start, tick_offset));
        }
    }
    // carry forward over never-played measures
    let mut carried = 0;
    first_ticks
        .into_iter()
        .map(|tick| {
            if let Some(tick) = tick {
                carried = tick;
            }
            carried
        })
        .collect()
}

/// Bit for the given repetition in an alternative ending bitmask.
/// Repetitions beyond the 8th never match.
const fn repetition_bit(repetition: i8) -> u8 {
    if repetition >= 0 && repetition < 8 {
        1 << repetition
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(start: u32, repeat_open: bool, repeat_close: i8) -> MeasureHeader {
        MeasureHeader {
            start,
            repeat_open,
            repeat_close,
            ..MeasureHeader::default()
        }
    }

    #[test]
    fn no_repeats() {
        let headers = vec![
            make_header(960, false, 0),
            make_header(4800, false, 0),
            make_header(8640, false, 0),
        ];
        let order = compute_playback_order(&headers);
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], (0, 0));
        assert_eq!(order[1], (1, 0));
        assert_eq!(order[2], (2, 0));
    }

    #[test]
    fn simple_repeat() {
        // |: M0 | M1 :|  M2
        // Plays: M0 M1 M0 M1 M2
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            make_header(960 + measure_len, false, 1),
            make_header(960 + measure_len * 2, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 1, 2]);

        // tick offsets: first pass is 0, second pass shifts by 2 measures
        assert_eq!(order[0].1, 0);
        assert_eq!(order[1].1, 0);
        assert_eq!(order[2].1, i64::from(measure_len) * 2);
        assert_eq!(order[3].1, i64::from(measure_len) * 2);
        assert_eq!(order[4].1, i64::from(measure_len) * 2);
    }

    #[test]
    fn repeat_three_times() {
        // |: M0 :| x3  M1
        // Plays: M0 M0 M0 M1
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 2),
            make_header(960 + measure_len, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 0, 0, 1]);
    }

    #[test]
    fn two_repeat_sections() {
        // |: M0 :|  |: M1 :|  M2
        // Plays: M0 M0 M1 M1 M2
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 1),
            make_header(960 + measure_len, true, 1),
            make_header(960 + measure_len * 2, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 0, 1, 1, 2]);
    }

    #[test]
    fn alternative_endings() {
        // |: M0 | M1[1.] :| M2[2.] | M3
        // Each ending carries its own close (as in real GP files); the last
        // ending has none and falls through.
        // Plays: M0 M1 M0 M2 M3
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            MeasureHeader {
                start: 960 + measure_len,
                repeat_alternative: 1,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            MeasureHeader {
                start: 960 + measure_len * 2,
                repeat_alternative: 2,
                ..MeasureHeader::default()
            },
            make_header(960 + measure_len * 3, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 2, 3]);
    }

    #[test]
    fn first_playback_ticks_carry_over_never_played_measures() {
        // |: M0 | M1[1.] :| M2[3.] :| M3
        // One repeat only: ending "3." never plays.
        // Plays: M0 M1 M0 M3
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            MeasureHeader {
                start: 960 + measure_len,
                repeat_alternative: 0b001,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            MeasureHeader {
                start: 960 + measure_len * 2,
                repeat_alternative: 0b100,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            make_header(960 + measure_len * 3, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 3]);

        // seeking to the never-played M2 falls back to M1's first playback tick
        let first_ticks = first_playback_ticks(&headers, &order);
        assert_eq!(first_ticks, vec![960, 4800, 4800, 12480]);
    }

    #[test]
    fn three_alternatives() {
        // |: M0 | M1[1.] :| M2[2.] :| M3[3.] | M4
        // Plays: M0 M1 M0 M2 M0 M3 M4
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            MeasureHeader {
                start: 960 + measure_len,
                repeat_alternative: 1,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            MeasureHeader {
                start: 960 + measure_len * 2,
                repeat_alternative: 2,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            MeasureHeader {
                start: 960 + measure_len * 3,
                repeat_alternative: 4,
                ..MeasureHeader::default()
            },
            make_header(960 + measure_len * 4, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 2, 0, 3, 4]);
    }

    #[test]
    fn nested_repeats() {
        // |: M0 |: M1 :| M2 :|  M3
        // Guitar Pro has no nested repeats: the second open restarts the
        // section, and the outer close is ignored once the inner section
        // completed (TuxGuitar semantics).
        // Plays: M0 M1 M1 M2 M3
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            make_header(960 + measure_len, true, 1),
            make_header(960 + measure_len * 2, false, 1),
            make_header(960 + measure_len * 3, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 1, 2, 3]);
    }

    #[test]
    fn repeat_close_without_open() {
        // M0 | M1 :|  M2
        // Plays: M0 M1 M0 M1 M2
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, false, 0),
            make_header(960 + measure_len, false, 1),
            make_header(960 + measure_len * 2, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 1, 2]);
    }

    #[test]
    fn single_measure_repeat() {
        // |: M0 :|
        // Plays: M0 M0
        let headers = vec![make_header(960, true, 1)];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 0]);
    }

    #[test]
    fn tick_offsets_are_consistent() {
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            make_header(960 + measure_len, false, 1),
            make_header(960 + measure_len * 2, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let playback_ticks: Vec<i64> = order
            .iter()
            .map(|(idx, offset)| i64::from(headers[*idx].start) + offset)
            .collect();
        assert!(playback_ticks.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn alternative_on_last_pass_with_close() {
        // |: M0 | M1[1.+2.] :|
        // A close inside an alternative ending always jumps back, so M0 plays
        // once more before M1 (matching no further ending) falls through
        // (TuxGuitar semantics).
        // Plays: M0 M1 M0 M1 M0
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            MeasureHeader {
                start: 960 + measure_len,
                repeat_alternative: 3,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 1, 0]);
    }

    #[test]
    fn empty() {
        let headers: Vec<MeasureHeader> = vec![];
        let order = compute_playback_order(&headers);
        assert!(order.is_empty());
    }

    #[test]
    fn trailing_alternative_after_close() {
        // |: M0 | M1[1.] :| M2[2.] | M3
        // The second ending lives after the closing measure; the repetition
        // counter must survive the repeat completing for M2 to match.
        // Plays: M0 M1 M0 M2 M3
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 0),
            MeasureHeader {
                start: 960 + measure_len,
                repeat_alternative: 1,
                repeat_close: 1,
                ..MeasureHeader::default()
            },
            MeasureHeader {
                start: 960 + measure_len * 2,
                repeat_alternative: 2,
                ..MeasureHeader::default()
            },
            make_header(960 + measure_len * 3, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 2, 3]);
    }

    #[test]
    fn completed_repeat_then_bare_close() {
        // |: M0 :|  M1 :|
        // A close without a new open after a completed section is ignored
        // (TuxGuitar semantics); it must not replay the song from measure 0.
        // Plays: M0 M0 M1
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, true, 1),
            make_header(960 + measure_len, false, 1),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 0, 1]);
    }

    #[test]
    fn bare_close_does_not_leak_into_next_repeat() {
        // M0 | M1 :|  |: M2 :|  M3
        // The jump back to measure 0 (which has no repeat_open) must not
        // corrupt the state of the later explicit repeat on M2.
        // Plays: M0 M1 M0 M1 M2 M2 M3
        let measure_len = 3840_u32;
        let headers = vec![
            make_header(960, false, 0),
            make_header(960 + measure_len, false, 1),
            make_header(960 + measure_len * 2, true, 1),
            make_header(960 + measure_len * 3, false, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 1, 2, 2, 3]);
    }

    #[test]
    fn sequential_sections_with_alternatives() {
        // |: M0 | M1[1.] :| M2[2.]  |: M3 | M4[1.] :| M5[2.]
        // The second section starts right after the first one; the repetition
        // counter must reset on its repeat_open for M4's first ending to match.
        // Plays: M0 M1 M0 M2 M3 M4 M3 M5
        let measure_len = 3840_u32;
        let alt = |idx: u32, repeat_alternative: u8, repeat_close: i8| MeasureHeader {
            start: 960 + measure_len * idx,
            repeat_alternative,
            repeat_close,
            ..MeasureHeader::default()
        };
        let headers = vec![
            make_header(960, true, 0),
            alt(1, 1, 1),
            alt(2, 2, 0),
            make_header(960 + measure_len * 3, true, 0),
            alt(4, 1, 1),
            alt(5, 2, 0),
        ];
        let order = compute_playback_order(&headers);
        let indices: Vec<usize> = order.iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 0, 2, 3, 4, 3, 5]);
    }

    /// Measures of one 4/4 bar each, with their marks.
    fn score(marks: &[(&[DirectionTarget], &[DirectionJump])]) -> Vec<MeasureHeader> {
        marks
            .iter()
            .enumerate()
            .map(|(i, (targets, jumps))| MeasureHeader {
                start: 960 + 3840 * i as u32,
                targets: targets.to_vec(),
                jumps: jumps.to_vec(),
                ..MeasureHeader::default()
            })
            .collect()
    }

    use DirectionJump::{DaCapoAlFine, DaCoda, DaSegno, DaSegnoAlCoda};
    use DirectionTarget::{Coda, Fine, Segno};

    fn indices(headers: &[MeasureHeader]) -> Vec<usize> {
        compute_playback_order(headers)
            .iter()
            .map(|(i, _)| *i)
            .collect()
    }

    #[test]
    fn da_segno_al_coda_goes_back_then_skips_to_the_coda() {
        let headers = score(&[
            (&[], &[]),
            (&[Segno], &[]),
            (&[], &[DaCoda]),
            (&[], &[]),
            (&[], &[DaSegnoAlCoda]),
            (&[Coda], &[]),
        ]);
        // To Coda is passed by the first time, followed on the way back
        assert_eq!(indices(&headers), vec![0, 1, 2, 3, 4, 1, 2, 5]);
    }

    #[test]
    fn da_capo_al_fine_stops_at_the_fine() {
        let headers = score(&[(&[], &[]), (&[Fine], &[]), (&[], &[DaCapoAlFine])]);
        assert_eq!(indices(&headers), vec![0, 1, 2, 0, 1]);
    }

    #[test]
    fn a_jump_without_its_mark_is_ignored() {
        let headers = score(&[(&[], &[]), (&[], &[DaSegno])]);
        assert_eq!(indices(&headers), vec![0, 1]);
    }

    #[test]
    fn the_way_back_skips_repeats_and_takes_the_last_ending() {
        let mut headers = score(&[
            (&[Segno], &[]),
            (&[], &[]),
            (&[], &[DaCoda]),
            (&[], &[DaSegnoAlCoda]),
            (&[Coda], &[]),
        ]);
        headers[0].repeat_open = true;
        // ending 1 closes the repeat, ending 2 carries the To Coda
        headers[1].repeat_alternative = 0b01;
        headers[1].repeat_close = 1;
        headers[2].repeat_alternative = 0b10;
        assert_eq!(indices(&headers), vec![0, 1, 0, 2, 3, 0, 2, 4]);
    }

    #[test]
    fn a_jump_waits_for_the_repeats_of_its_measure() {
        let mut headers = score(&[(&[Segno], &[]), (&[], &[DaSegno])]);
        headers[1].repeat_open = true;
        headers[1].repeat_close = 1;
        // the repeat of the D.S. measure plays first, then the jump; the
        // way back plays it once and ends the song
        assert_eq!(indices(&headers), vec![0, 1, 1, 0, 1]);
    }
}
