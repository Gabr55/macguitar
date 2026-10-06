//! The notes on the strings: frets, ties, slurs and slides.

use super::bends::draw_bend;
use super::chords::{draw_chord_diagram, has_diagram};
use super::colors::{FRET_FONT, TablatureColors};
use super::effects::{
    beat_annotations, draw_pick_stroke, draw_staccato_dot, draw_stroke_arrow, draw_tremolo_picking,
};
use super::layout::{
    CHORD_DIAGRAM_HEIGHT, GRACE_DIGIT_WIDTH, GRACE_GAP, NOTE_DIGIT_WIDTH, ROW_EFFECT_LINE,
    RowSpacing, STRING_LINE_HEIGHT, logical_width,
};
use super::rhythm::{draw_rest, is_rest};
use crate::parser::model::{
    Beat, BeatStrokeDirection, BendEffect, GraceEffect, Note, NoteType, SlideType,
};
use crate::ui::widgets::UI_FONT;
use iced::advanced::text::Shaping::Auto;
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{Color, Point, Renderer, Size};

/// Position and fret of the next note on `string` after `beat_index`,
/// within the measure, like TuxGuitar's `getNextNote`. Beats without a note
/// on the string are skipped.
pub(super) fn next_note_on_string(
    beats: &[Beat],
    beat_positions: &[f32],
    beat_index: usize,
    string: i8,
) -> Option<(f32, i16)> {
    beats
        .iter()
        .zip(beat_positions)
        .skip(beat_index + 1)
        .find_map(|(beat, &x)| {
            beat.notes
                .iter()
                .find(|note| note.string == string)
                .map(|note| (x, note.value))
        })
}

/// Position of the note a tie hangs from: the closest earlier note on the
/// same string, like TuxGuitar's `getNoteForTie`. A rest before it breaks
/// the tie, leaving nothing to hang from.
pub(super) fn tied_from_note_x(
    beats: &[Beat],
    beat_positions: &[f32],
    beat_index: usize,
    string: i8,
) -> Option<f32> {
    for (beat, &x) in beats.iter().zip(beat_positions).take(beat_index).rev() {
        if beat.notes.is_empty() {
            return None;
        }
        if beat.notes.iter().any(|note| note.string == string) {
            return Some(x);
        }
    }
    None
}

/// Label of a grace note: its fret, or a cross when it is dead.
pub(super) fn grace_label(grace: &GraceEffect) -> String {
    if grace.is_dead {
        "x".to_string()
    } else {
        grace.fret.to_string()
    }
}

/// Like TuxGuitar's `multipleBendConflicts`: with several bent notes in a
/// beat, only the lowest-string one keeps its amplitude label, and only
/// when all the bends start with the same movement.
pub(super) fn multiple_bend_conflicts(beat: &Beat, note: &Note, movements: &[i32]) -> bool {
    beat.notes.iter().any(|other| {
        other.effect.bend.as_ref().is_some_and(|other_bend| {
            if other.string < note.string {
                return true;
            }
            let other_movements = other_bend.movements();
            !other_movements.is_empty()
                && !movements.is_empty()
                && other_movements.first() != movements.first()
        })
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_beat(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    beat_position_x: f32,
    // span usable for glyphs drawn after the note, up to the next beat's grace
    beat_span: f32,
    width_scale: f32,
    measure_start_y: f32,
    vertical_measure_height: f32,
    rows: RowSpacing,
    beat: &Beat,
    beat_color: Color,
    beats: &[Beat],
    beat_positions: &[f32],
    beat_index: usize,
    next_measure: Option<&[Beat]>,
) {
    // Annotate chord effect
    if let Some(chord) = &beat.effect.chord {
        let mut name_y = rows.chord_y();
        if has_diagram(chord) {
            draw_chord_diagram(frame, colors, chord, beat_position_x + 3.0, name_y);
            name_y += CHORD_DIAGRAM_HEIGHT;
        }
        let note_effect_text = Text {
            shaping: Auto,
            content: chord.name.clone(),
            color: colors.foreground,
            size: 8.0.into(),
            position: Point::new(beat_position_x + 3.0, name_y),
            font: UI_FONT,
            ..Text::default()
        };
        frame.fill_text(note_effect_text);
    }
    if is_rest(beat) {
        draw_rest(
            frame,
            colors,
            beat_color,
            beat,
            beat_position_x + 2.0,
            measure_start_y,
            vertical_measure_height,
        );
    }
    if !beat.effect.stroke.is_empty() && !beat.notes.is_empty() {
        draw_stroke_arrow(frame, colors, beat, beat_position_x, measure_start_y);
    }
    if beat.effect.pick_stroke != BeatStrokeDirection::None {
        draw_pick_stroke(
            frame,
            colors,
            &beat.effect.pick_stroke,
            beat_position_x,
            rows.pick_stroke_y(),
        );
    }
    if beat.notes.iter().any(|n| n.effect.staccato) {
        draw_staccato_dot(frame, colors, beat, beat_position_x, measure_start_y);
    }
    if let Some(tremolo_picking) = beat
        .notes
        .iter()
        .find_map(|n| n.effect.tremolo_picking.as_ref())
    {
        draw_tremolo_picking(
            frame,
            colors,
            tremolo_picking,
            beat_position_x,
            measure_start_y + vertical_measure_height,
        );
    }

    // draw notes for beat
    for note in &beat.notes {
        let bend_movements = note.effect.bend.as_ref().map(BendEffect::movements);
        let show_bend_amplitude = bend_movements
            .as_deref()
            .is_some_and(|movements| !multiple_bend_conflicts(beat, note, movements));
        draw_note(
            frame,
            colors,
            measure_start_y,
            beat_position_x,
            beat_span,
            width_scale,
            note,
            beat_color,
            bend_movements.as_deref(),
            show_bend_amplitude,
            beats,
            beat_positions,
            beat_index,
            next_measure,
        );
    }

    // merge and display beat annotations (same position for all notes)
    // one annotation per row of the effect band, stacked downwards
    for (line, annotation) in beat_annotations(beat).into_iter().enumerate() {
        let y_position = rows.effects_y() + line as f32 * ROW_EFFECT_LINE;
        let note_effect_text = Text {
            shaping: Auto,
            content: annotation.to_string(),
            color: colors.muted,
            size: 8.5.into(),
            position: Point::new(beat_position_x - 3.0, y_position),
            font: UI_FONT,
            ..Text::default()
        };
        frame.fill_text(note_effect_text);
    }

    // user-authored text attached to the beat (e.g. "Verse", "fill")
    if !beat.text.is_empty() {
        let beat_text = Text {
            shaping: Auto,
            content: beat.text.clone(),
            color: colors.foreground,
            size: 8.0.into(),
            position: Point::new(beat_position_x + 3.0, rows.text_y()),
            font: UI_FONT,
            ..Text::default()
        };
        frame.fill_text(beat_text);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_note(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    measure_start_y: f32,
    beat_position_x: f32,
    width_per_beat: f32,
    width_scale: f32,
    note: &Note,
    beat_color: Color,
    bend_movements: Option<&[i32]>,
    show_bend_amplitude: bool,
    // the measure's beats and their positions, to reach neighbouring notes
    beats: &[Beat],
    beat_positions: &[f32],
    beat_index: usize,
    // the following measure, where a slide may land
    next_measure: Option<&[Beat]>,
) {
    // a tie continuing a note of this measure is an arc back to it; one
    // continuing a note before (the previous measure, past a rest) has no
    // note here to reach, and writes its fret in parentheses, as scores do
    let tied_from = (note.kind == NoteType::Tie)
        .then(|| tied_from_note_x(beats, beat_positions, beat_index, note.string))
        .flatten();
    let note_label = if note.kind == NoteType::Tie && tied_from.is_none() {
        format!("({})", note.value)
    } else {
        note_value(note)
    };
    let note_label_len = note_label.chars().count();
    let local_beat_position_y = (f32::from(note.string) - 1.0) * STRING_LINE_HEIGHT;
    // center the notes with more than one char
    let note_position_x = beat_position_x + 3.0 - note_label_len as f32 / 2.0;
    let note_position_y = measure_start_y + local_beat_position_y - 5.0;
    let string_y = measure_start_y + local_beat_position_y;

    if let Some(from_x) = tied_from {
        draw_tie_arc(frame, beat_color, from_x + 3.0, note_position_x, string_y);
    } else {
        // cut the string line around the fret, so it reads cleanly
        let paper = if beat_color == colors.accent {
            colors.cursor
        } else {
            colors.paper
        };
        let knockout_width = note_label_len as f32 * NOTE_DIGIT_WIDTH + 2.0;
        frame.fill_rectangle(
            Point::new(note_position_x - knockout_width / 2.0, string_y - 5.5),
            Size::new(knockout_width, 11.0),
            paper,
        );
        let note_text = Text {
            shaping: Auto,
            content: note_label,
            color: beat_color,
            size: 12.0.into(),
            position: Point::new(note_position_x, string_y),
            align_x: Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            font: FRET_FONT,
            ..Text::default()
        };
        frame.fill_text(note_text);
    }

    // small grace fret before the note, like TuxGuitar's paintEffects.
    // both glyphs are centred, so offset by their half widths to keep the
    // grace clear of the note whatever their digit counts
    if let Some(grace) = &note.effect.grace {
        let label = grace_label(grace);
        let note_half = note_label_len as f32 * NOTE_DIGIT_WIDTH / 2.0;
        let grace_half = label.chars().count() as f32 * GRACE_DIGIT_WIDTH / 2.0;
        let grace_text = Text {
            shaping: Auto,
            content: label,
            color: colors.foreground,
            size: 7.0.into(),
            position: Point::new(
                note_position_x - note_half - grace_half - GRACE_GAP,
                note_position_y + 2.0,
            ),
            align_x: Alignment::Center,
            font: UI_FONT,
            ..Text::default()
        };
        frame.fill_text(grace_text);
    }

    // like TuxGuitar's paintEffects, the bend arrows are exclusive with the
    // inline slide/hammer glyphs: they would collide in the same span
    if let Some(movements) = bend_movements {
        draw_bend(
            frame,
            colors,
            movements,
            note_position_x,
            note_position_y,
            beat_position_x,
            width_per_beat,
            width_scale,
            measure_start_y,
            show_bend_amplitude,
        );
    } else {
        // slides and hammers reach for the note they land on, like
        // TuxGuitar's paintSlide and paintHammer
        let next_note = next_note_on_string(beats, beat_positions, beat_index, note.string);
        let note_half = note_label_len as f32 * NOTE_DIGIT_WIDTH / 2.0;
        // a chord played legato takes one slur, over its highest note
        let slurred_above = beats[beat_index].notes.iter().any(|other| {
            other.string < note.string
                && (other.effect.hammer || other.effect.slide == Some(SlideType::LegatoSlideTo))
        });
        if let Some(slide) = note.effect.slide {
            // a slide into the next measure still slopes towards its fret
            // within the measure the slide reaches its note; into the next
            // one it only points the way
            let target = next_note
                .map(|(x, value)| (x + 3.0, value, false))
                .or_else(|| {
                    next_measure
                        .and_then(|next| first_fret_on_string(next, note.string))
                        .map(|value| (logical_width(frame) + NOTE_DIGIT_WIDTH, value, true))
                });
            draw_slide(
                frame,
                colors,
                slide,
                note_position_x,
                note_half,
                string_y,
                target,
                note.value,
            );
            if slide == SlideType::LegatoSlideTo && !slurred_above {
                draw_hammer_arc(
                    frame,
                    colors,
                    note_position_x,
                    note_position_y,
                    next_note.map(|(x, _)| x + 3.0),
                );
            }
        } else if note.effect.hammer && !slurred_above {
            draw_hammer_arc(
                frame,
                colors,
                note_position_x,
                note_position_y,
                next_note.map(|(x, _)| x + 3.0),
            );
        }
    }
}

/// The fret of the first note on `string` in a measure, if it has one.
pub(super) fn first_fret_on_string(beats: &[Beat], string: i8) -> Option<i16> {
    beats
        .iter()
        .flat_map(|beat| &beat.notes)
        .find(|note| note.string == string && note.kind != NoteType::Tie)
        .map(|note| note.value)
}

/// The sloped line of a slide. A shift or legato slide rises or falls
/// towards the note it lands on; the others are short strokes leading into
/// the note or away from it, in their direction.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_slide(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    slide: SlideType,
    note_position_x: f32,
    note_half: f32,
    string_y: f32,
    // the note landed on: position, fret, and whether it is in the next
    // measure
    target: Option<(f32, i16, bool)>,
    value: i16,
) {
    // vertical reach on each side of the string, and length of a stroke
    const RISE: f32 = STRING_LINE_HEIGHT / 2.8;
    const STROKE: f32 = 11.0;
    // a slide into the next measure keeps its slope and stops short,
    // pointing the way
    const MAX_LENGTH: f32 = 24.0;
    const GAP: f32 = 2.0;
    let after = note_position_x + note_half + GAP;
    let before = note_position_x - note_half - GAP;
    let (from, to) = match slide {
        SlideType::IntoFromBelow => (
            Point::new(before - STROKE, string_y + RISE),
            Point::new(before, string_y - RISE),
        ),
        SlideType::IntoFromAbove => (
            Point::new(before - STROKE, string_y - RISE),
            Point::new(before, string_y + RISE),
        ),
        SlideType::OutUpWards => (
            Point::new(after, string_y + RISE),
            Point::new(after + STROKE, string_y - RISE),
        ),
        SlideType::OutDownwards => (
            Point::new(after, string_y - RISE),
            Point::new(after + STROKE, string_y + RISE),
        ),
        SlideType::ShiftSlideTo | SlideType::LegatoSlideTo => {
            let Some((next_x, next_value, beyond)) = target else {
                // nowhere to land: leave the note downwards
                return draw_slide(
                    frame,
                    colors,
                    SlideType::OutDownwards,
                    note_position_x,
                    note_half,
                    string_y,
                    None,
                    value,
                );
            };
            let end_x = (next_x - note_half - GAP).max(after + 3.0);
            let end_x = if beyond {
                end_x.min(after + MAX_LENGTH)
            } else {
                end_x
            };
            let rise = match next_value.cmp(&value) {
                std::cmp::Ordering::Less => RISE,
                std::cmp::Ordering::Greater => -RISE,
                std::cmp::Ordering::Equal => 0.0,
            };
            (
                Point::new(after, string_y - rise),
                Point::new(end_x, string_y + rise),
            )
        }
    };
    frame.stroke(
        &Path::line(from, to),
        Stroke::default()
            .with_width(1.0)
            .with_color(colors.foreground),
    );
}

/// The slur of a hammer-on, pull-off or legato slide: a flat arc over the
/// frets, from the middle of this one to the middle of the next, as scores
/// write it. Kept low, it clears the frets of the string above in a chord,
/// and the line of a slide runs under it, between the frets.
pub(super) fn draw_hammer_arc(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    note_position_x: f32,
    note_position_y: f32,
    next_note_x: Option<f32>,
) {
    // just over the top of the digits, which start at note_position_y
    const LIFT: f32 = 2.0;
    const HEIGHT: f32 = 4.5;
    let start_x = note_position_x;
    let end_x = next_note_x.map_or(start_x + 10.0, |next| next.max(start_x + 6.0));
    let y = note_position_y - LIFT;
    let span = end_x - start_x;
    let arc = Path::new(|p| {
        p.move_to(Point::new(start_x + 1.0, y));
        p.bezier_curve_to(
            Point::new(start_x + span * 0.25, y - HEIGHT),
            Point::new(start_x + span * 0.75, y - HEIGHT),
            Point::new(end_x - 1.0, y),
        );
    });
    frame.stroke(
        &arc,
        Stroke::default()
            .with_width(0.9)
            .with_color(colors.foreground)
            .with_line_cap(iced::widget::canvas::LineCap::Round),
    );
}

/// The arc joining a tied note back to the one it continues.
pub(super) fn draw_tie_arc(
    frame: &mut Frame<Renderer>,
    color: Color,
    from_x: f32,
    to_x: f32,
    string_y: f32,
) {
    let y = string_y + STRING_LINE_HEIGHT / 3.0;
    let height = STRING_LINE_HEIGHT / 3.0;
    let from_x = from_x.min(to_x - 4.0);
    let arc = Path::new(|p| {
        p.move_to(Point::new(from_x, y));
        p.bezier_curve_to(
            Point::new(from_x, y + height),
            Point::new(to_x, y + height),
            Point::new(to_x, y),
        );
    });
    frame.stroke(&arc, Stroke::default().with_width(0.9).with_color(color));
}

pub(super) fn note_value(note: &Note) -> String {
    match note.kind {
        NoteType::Rest => {
            log::debug!("NoteType Rest");
            String::new()
        }
        NoteType::Normal => {
            if note.effect.ghost_note {
                format!("({})", note.value)
            } else {
                note.value.to_string()
            }
        }
        // a tie draws an arc back to its note instead of a label
        NoteType::Tie => String::new(),
        NoteType::Dead => "x".to_string(),
        NoteType::Unknown(i) => {
            log::warn!("NoteType Unknown({i})");
            String::new()
        }
    }
}
