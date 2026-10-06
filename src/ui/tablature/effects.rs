//! Effects written over the staff: articulations, and the palm mute, let
//! ring and vibrato drawn across the beats they last.

use super::colors::TablatureColors;
use super::layout::{ROW_SPAN, STRING_LINE_HEIGHT};
use crate::parser::model::{
    Beat, BeatStrokeDirection, HarmonicType, KeySignature, MeasureHeader, NoteEffect, SlapEffect,
    TremoloPickingEffect, TripletFeel,
};
use crate::ui::widgets::UI_FONT;
use iced::advanced::text::Shaping::Auto;
use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{Color, Point, Renderer};

/// Whether a measure states its key: when it changes, or when a song opens
/// in something other than the default key everything starts in.
pub(super) fn shows_key_signature(
    header: &MeasureHeader,
    previous: Option<&MeasureHeader>,
) -> bool {
    previous.map_or(
        header.key_signature != KeySignature::new(0, false),
        |previous| header.key_signature != previous.key_signature,
    )
}

/// How a change of feel reads: TuxGuitar draws note pictograms, which the
/// tab spells out instead.
pub(super) const fn triplet_feel_label(feel: TripletFeel) -> &'static str {
    match feel {
        TripletFeel::None => "straight",
        TripletFeel::Eighth => "swing 8th",
        TripletFeel::Sixteenth => "swing 16th",
    }
}

/// An effect drawn across the beats it lasts rather than on each of them,
/// as scores write it: a label where it starts, then a dashed line (a wave
/// for a vibrato) up to where it stops, carried over bar lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpanEffect {
    LetRing,
    Vibrato,
    PalmMute,
}

impl SpanEffect {
    pub(super) const ALL: [Self; 3] = [Self::LetRing, Self::Vibrato, Self::PalmMute];

    pub(super) fn is_on(self, beat: &Beat) -> bool {
        beat.notes.iter().any(|note| match self {
            Self::LetRing => note.effect.let_ring,
            Self::Vibrato => note.effect.vibrato,
            Self::PalmMute => note.effect.palm_mute,
        })
    }

    /// Written where the effect starts; the vibrato wave speaks for itself.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::LetRing => "let ring",
            Self::Vibrato => "",
            Self::PalmMute => "P.M.",
        }
    }
}

/// Whether `effect` runs on from the last sounding beat of `before` into
/// the first one of `after`.
pub(super) fn span_crosses(effect: SpanEffect, before: &[Beat], after: &[Beat]) -> bool {
    let last = before.iter().rev().find(|beat| !beat.empty);
    let first = after.iter().find(|beat| !beat.empty);
    last.zip(first)
        .is_some_and(|(last, first)| effect.is_on(last) && effect.is_on(first))
}

/// Draw the runs of `effect` over the measure's beats, in its lane.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_effect_spans(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    effect: SpanEffect,
    beats: &[Beat],
    beat_positions: &[f32],
    from_previous: bool,
    into_next: bool,
    measure_end_x: f32,
    lane_y: f32,
) {
    // the label's font advance, to start the line after it
    const LABEL_ADVANCE: f32 = 4.4;
    const END_TICK: f32 = 3.5;
    let y = lane_y + ROW_SPAN / 2.0;
    let mut index = 0;
    while index < beats.len() {
        if !effect.is_on(&beats[index]) {
            index += 1;
            continue;
        }
        let first = index;
        while index + 1 < beats.len() && effect.is_on(&beats[index + 1]) {
            index += 1;
        }
        let last = index;
        index += 1;

        let continued = first == 0 && from_previous;
        let continues = last + 1 == beats.len() && into_next;
        let start_x = if continued {
            0.0
        } else {
            beat_positions[first] - 1.0
        };
        // the effect lasts until the next beat starts
        let end_x = if continues {
            measure_end_x
        } else {
            beat_positions
                .get(last + 1)
                .map_or(beat_positions[last] + 14.0, |next| next - 4.0)
        };

        let mut line_x = start_x;
        let label = effect.label();
        if !continued && !label.is_empty() {
            frame.fill_text(Text {
                shaping: Auto,
                content: label.to_string(),
                color: colors.muted,
                size: 8.5.into(),
                position: Point::new(start_x, y),
                align_y: iced::alignment::Vertical::Center,
                font: UI_FONT,
                ..Text::default()
            });
            line_x += label.chars().count() as f32 * LABEL_ADVANCE + 4.0;
        }
        // too short a run for a line to read as one: the label says it
        if end_x - line_x < 8.0 {
            continue;
        }

        if effect == SpanEffect::Vibrato {
            draw_wave(frame, colors.foreground, line_x, end_x, y);
            continue;
        }
        let dashed = Stroke {
            line_dash: LineDash {
                segments: &[3.0, 2.5],
                offset: 0,
            },
            ..Stroke::default().with_width(0.9).with_color(colors.muted)
        };
        frame.stroke(
            &Path::line(Point::new(line_x, y), Point::new(end_x, y)),
            dashed,
        );
        if !continues {
            frame.stroke(
                &Path::line(Point::new(end_x, y), Point::new(end_x, y + END_TICK)),
                Stroke::default().with_width(0.9).with_color(colors.muted),
            );
        }
    }
}

/// A vibrato wave from `start_x` to `end_x`, in whole periods.
pub(super) fn draw_wave(
    frame: &mut Frame<Renderer>,
    color: Color,
    start_x: f32,
    end_x: f32,
    y: f32,
) {
    const PERIOD: f32 = 5.0;
    const AMPLITUDE: f32 = 1.8;
    let periods = ((end_x - start_x) / PERIOD).floor().max(1.0) as usize;
    let wave = Path::new(|builder| {
        builder.move_to(Point::new(start_x, y));
        for period in 0..periods {
            let x = start_x + period as f32 * PERIOD;
            builder.quadratic_curve_to(
                Point::new(x + PERIOD / 4.0, y - AMPLITUDE * 2.0),
                Point::new(x + PERIOD / 2.0, y),
            );
            builder.quadratic_curve_to(
                Point::new(x + PERIOD * 3.0 / 4.0, y + AMPLITUDE * 2.0),
                Point::new(x + PERIOD, y),
            );
        }
    });
    frame.stroke(
        &wave,
        Stroke::default()
            .with_width(1.1)
            .with_color(color)
            .with_line_cap(iced::widget::canvas::LineCap::Round),
    );
}

/// Effect annotations stacked above a beat, deduplicated across its notes.
/// Shared by the row sizing and the drawing so both agree on the height.
pub(super) fn beat_annotations(beat: &Beat) -> Vec<&'static str> {
    let mut annotations: Vec<&'static str> = beat
        .notes
        .iter()
        .flat_map(|note| above_note_effect_annotation(&note.effect))
        .collect();
    annotations.sort_unstable();
    annotations.dedup();
    annotations
}

/// Picking direction above the staff, like TuxGuitar's `paintPickStroke`:
/// an up stroke is a `∨`, a down stroke the bracket-shaped `∏`.
pub(super) fn draw_pick_stroke(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    direction: &BeatStrokeDirection,
    beat_position_x: f32,
    y: f32,
) {
    let stroke = Stroke::default()
        .with_width(0.8)
        .with_color(colors.foreground);
    let x = beat_position_x + 3.5;
    match direction {
        BeatStrokeDirection::Up => {
            let tip = Point::new(x, y + 8.0);
            frame.stroke(&Path::line(Point::new(x - 3.0, y), tip), stroke);
            frame.stroke(&Path::line(Point::new(x + 3.0, y), tip), stroke);
        }
        BeatStrokeDirection::Down => {
            frame.stroke(
                &Path::line(Point::new(x - 3.0, y), Point::new(x - 3.0, y + 6.0)),
                stroke,
            );
            frame.stroke(
                &Path::line(Point::new(x + 3.0, y), Point::new(x + 3.0, y + 6.0)),
                stroke,
            );
            let top_bar = Path::line(Point::new(x - 3.0, y), Point::new(x + 3.0, y));
            frame.stroke(
                &top_bar,
                Stroke::default()
                    .with_width(2.0)
                    .with_color(colors.foreground),
            );
        }
        BeatStrokeDirection::None => {}
    }
}

/// Staccato dot above the top note of the beat (TuxGuitar only draws it in
/// score mode; the dot above the fret number is the tab equivalent).
pub(super) fn draw_staccato_dot(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    beat: &Beat,
    beat_position_x: f32,
    measure_start_y: f32,
) {
    let min_string = beat.notes.iter().map(|n| n.string).min().unwrap_or(1);
    let top_note_y = measure_start_y + (f32::from(min_string) - 1.0) * STRING_LINE_HEIGHT - 5.0;
    let center = Point::new(beat_position_x + 3.5, top_note_y - 3.0);
    frame.fill(&Path::circle(center, 1.3), colors.foreground);
}

/// Tremolo picking slashes below the tab, one per duration halving from an
/// eighth note, like TuxGuitar's tablature-only rendering.
pub(super) fn draw_tremolo_picking(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    tremolo_picking: &TremoloPickingEffect,
    beat_position_x: f32,
    tab_bottom_y: f32,
) {
    let slashes = match tremolo_picking.duration.value {
        v if v >= 32 => 3,
        v if v >= 16 => 2,
        _ => 1,
    };
    let stroke = Stroke::default()
        .with_width(1.2)
        .with_color(colors.foreground);
    let x = beat_position_x + 3.5;
    let mut y = tab_bottom_y + 5.0;
    for _ in 0..slashes {
        frame.stroke(
            &Path::line(Point::new(x - 3.5, y + 1.5), Point::new(x + 3.5, y - 1.5)),
            stroke,
        );
        y += 4.0;
    }
}

pub(super) fn draw_stroke_arrow(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    beat: &Beat,
    beat_position_x: f32,
    measure_start_y: f32,
) {
    let min_string = beat.notes.iter().map(|n| n.string).min().unwrap_or(1);
    let max_string = beat.notes.iter().map(|n| n.string).max().unwrap_or(1);
    let top_y = measure_start_y + (f32::from(min_string) - 1.0) * STRING_LINE_HEIGHT;
    let bottom_y = measure_start_y + (f32::from(max_string) - 1.0) * STRING_LINE_HEIGHT;
    let arrow_x = beat_position_x + 10.0;
    let arrow_size = 3.0;

    let stroke = Stroke::default()
        .with_width(0.8)
        .with_color(colors.foreground);

    // vertical line spanning the chord
    frame.stroke(
        &Path::line(Point::new(arrow_x, top_y), Point::new(arrow_x, bottom_y)),
        stroke,
    );

    // arrowhead: down stroke = pick goes low-to-high strings = arrowhead at top
    match beat.effect.stroke.direction {
        BeatStrokeDirection::Down => {
            let tip = Point::new(arrow_x, top_y - arrow_size);
            frame.stroke(
                &Path::line(Point::new(arrow_x - arrow_size, top_y), tip),
                stroke,
            );
            frame.stroke(
                &Path::line(Point::new(arrow_x + arrow_size, top_y), tip),
                stroke,
            );
        }
        BeatStrokeDirection::Up => {
            let tip = Point::new(arrow_x, bottom_y + arrow_size);
            frame.stroke(
                &Path::line(Point::new(arrow_x - arrow_size, bottom_y), tip),
                stroke,
            );
            frame.stroke(
                &Path::line(Point::new(arrow_x + arrow_size, bottom_y), tip),
                stroke,
            );
        }
        BeatStrokeDirection::None => {}
    }
}

// Similar to `https://www.tuxguitar.app/files/1.6.0/desktop/help/edit_effects.html`
pub(super) fn above_note_effect_annotation(note_effect: &NoteEffect) -> Vec<&'static str> {
    let mut annotations: Vec<&'static str> = vec![];
    if note_effect.accentuated_note {
        annotations.push(">");
    }
    if note_effect.heavy_accentuated_note {
        annotations.push("^");
    }
    if note_effect.fade_in {
        annotations.push("<");
    }
    if let Some(harmonic) = &note_effect.harmonic {
        match harmonic.kind {
            HarmonicType::Natural => annotations.push("N.H"),
            HarmonicType::Artificial => annotations.push("A.H"),
            HarmonicType::Tapped => annotations.push("T.H"),
            HarmonicType::Pinch => annotations.push("P.H"),
            HarmonicType::Semi => annotations.push("S.H"),
        }
    }
    if note_effect.trill.is_some() {
        annotations.push("Tr");
    }
    if note_effect.tremolo_bar.is_some() {
        annotations.push("T.B");
    }
    match note_effect.slap {
        SlapEffect::Tapping => annotations.push("T"),
        SlapEffect::Slapping => annotations.push("S"),
        SlapEffect::Popping => annotations.push("P"),
        SlapEffect::None => {}
    }
    annotations
}
