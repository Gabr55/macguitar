//! The rhythm under the staff: stems, beams, flags, dots, tuplets, and the
//! rests.

use super::colors::TablatureColors;
use super::layout::{
    BEAM_SPACING, BEAM_THICKNESS, FLAG_WIDTH, MAX_BEAMS, REST_WIDTH, STEM_LENGTH, STEM_TOP,
    STRING_LINE_HEIGHT,
};
use crate::parser::model::{Beat, Duration, MeasureHeader, QUARTER_TIME};
use crate::ui::widgets::UI_FONT;
use iced::advanced::text::Shaping::Auto;
use iced::widget::canvas::{Frame, LineCap, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{Color, Point, Renderer, Size};

/// How a beat's rhythm is drawn under the tab: `None` when it carries no
/// stem, otherwise the number of beams the stem needs.
///
/// A whole note has no stem; a half note gets a short one; a quarter a full
/// one; and every halving below that adds a beam, as TuxGuitar does.
pub(super) fn stem_beams(beat: &Beat) -> Option<usize> {
    if beat.notes.is_empty() || beat.duration.value <= 1 {
        // a silence is drawn as a rest on the staff instead
        return None;
    }
    let halvings = f32::from(beat.duration.value).log2().round() as usize;
    Some(halvings.saturating_sub(2).min(MAX_BEAMS))
}

/// Whether a beat is a silence: it takes its time without sounding.
///
/// A beat flagged empty carries no time of its own, so it is not a rest.
pub(super) const fn is_rest(beat: &Beat) -> bool {
    beat.notes.is_empty() && !beat.empty
}

/// The rest of a silent beat, centred on the staff, in the shapes engraved
/// scores use: a block hung under a line for a whole, sitting on the next
/// one for a half, a bold zigzag with a curl for a quarter, and below that a
/// slanted stem carrying one flag per halving, each flag ending in a drop.
///
/// The strings are cut behind it, as behind the frets, so it reads at a
/// glance; the rest being played takes the accent like a fret would.
pub(super) fn draw_rest(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    color: Color,
    beat: &Beat,
    x: f32,
    staff_top_y: f32,
    staff_height: f32,
) {
    let center_y = staff_top_y + staff_height / 2.0;
    let paper = if color == colors.accent {
        colors.cursor
    } else {
        colors.paper
    };
    let thick = Stroke::default()
        .with_width(2.1)
        .with_color(color)
        .with_line_cap(iced::widget::canvas::LineCap::Round)
        .with_line_join(iced::widget::canvas::LineJoin::Round);
    let thin = Stroke::default()
        .with_width(1.3)
        .with_color(color)
        .with_line_cap(iced::widget::canvas::LineCap::Round);
    let mut knockout = |top: f32, height: f32| {
        frame.fill_rectangle(
            Point::new(x - 1.5, top - 1.5),
            Size::new(REST_WIDTH + 3.0, height + 3.0),
            paper,
        );
    };
    match beat.duration.value {
        1 | 2 => {
            // the string at or above the middle of the staff, and the next
            let line = staff_top_y
                + ((staff_height / 2.0) / STRING_LINE_HEIGHT).floor() * STRING_LINE_HEIGHT;
            let (top, height) = if beat.duration.value == 1 {
                (line, 4.5)
            } else {
                (line + STRING_LINE_HEIGHT - 4.5, 4.5)
            };
            knockout(top + 1.0, height - 2.0);
            frame.fill(
                &Path::rounded_rectangle(
                    Point::new(x, top),
                    Size::new(REST_WIDTH, height),
                    0.8.into(),
                ),
                color,
            );
        }
        4 => {
            let height = 18.0;
            let top = center_y - height / 2.0;
            knockout(top, height);
            let zigzag = Path::new(|p| {
                p.move_to(Point::new(x + 2.0, top));
                p.line_to(Point::new(x + 6.5, top + 5.5));
                p.line_to(Point::new(x + 2.5, top + 9.5));
                p.line_to(Point::new(x + 6.5, top + 13.5));
                // the curl at the foot
                p.quadratic_curve_to(
                    Point::new(x + 0.5, top + 11.0),
                    Point::new(x + 3.5, top + height),
                );
            });
            frame.stroke(&zigzag, thick);
        }
        value => {
            const FLAG_STEP: f32 = 5.0;
            // the stem leans like an italic stroke
            const SLANT: f32 = 0.27;
            let flags = (f32::from(value.max(8)).log2().round() as usize - 2).min(MAX_BEAMS);
            let stem_length = 13.0 + (flags - 1) as f32 * FLAG_STEP;
            let top = center_y - (stem_length + 1.0) / 2.0;
            knockout(top, stem_length + 1.0);
            let stem_top = Point::new(x + REST_WIDTH, top + 1.0);
            frame.stroke(
                &Path::line(
                    stem_top,
                    Point::new(stem_top.x - stem_length * SLANT, stem_top.y + stem_length),
                ),
                thin,
            );
            for flag in 0..flags {
                let dy = flag as f32 * FLAG_STEP;
                let joint = Point::new(stem_top.x - dy * SLANT, stem_top.y + dy);
                let drop = Point::new(joint.x - 5.8, joint.y + 1.6);
                frame.fill(&Path::circle(drop, 1.9), color);
                frame.stroke(
                    &Path::new(|p| {
                        p.move_to(drop);
                        p.quadratic_curve_to(Point::new(joint.x - 2.5, joint.y + 3.5), joint);
                    }),
                    thin,
                );
            }
        }
    }
    draw_duration_dot(frame, colors, beat, x + REST_WIDTH + 3.0, center_y);
}

/// How long a run of short notes may be before it breaks, from TuxGuitar's
/// `getDivisionLength`: a quarter note, or a dotted one in compound time.
pub(super) const fn division_length(header: &MeasureHeader) -> u32 {
    let signature = &header.time_signature;
    if signature.denominator.value == 8 && signature.numerator.is_multiple_of(3) {
        QUARTER_TIME + QUARTER_TIME / 2
    } else {
        QUARTER_TIME
    }
}

/// Runs of beats beamed together: consecutive sounding notes of an eighth or
/// shorter falling in one division of the measure, like TuxGuitar's `canJoin`.
/// Each tuplet is beamed on its own, never with the plain notes or the
/// other tuplets around it, as Guitar Pro writes it.
///
/// Each run is the inclusive index range of the beats it joins.
pub(super) fn beam_runs(beats: &[Beat], measure_start: u32, division: u32) -> Vec<(usize, usize)> {
    let joinable = |beat: &Beat| stem_beams(beat).is_some_and(|beams| beams > 0);
    let division_of = |beat: &Beat| beat.start.saturating_sub(measure_start) / division.max(1);
    let tuplets = tuplet_groups(beats);

    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (index, beat) in beats.iter().enumerate() {
        if !joinable(beat) {
            continue;
        }
        match runs.last_mut() {
            // a run carries on while the beats stay adjacent, in the same
            // division of the measure, and in the same tuplet or in none
            Some(run)
                if run.1 + 1 == index
                    && division_of(&beats[run.1]) == division_of(beat)
                    && tuplets[run.1] == tuplets[index] =>
            {
                run.1 = index;
            }
            _ => runs.push((index, index)),
        }
    }
    runs
}

/// The rhythm under the staff: a stem per note, beams over the runs that
/// join, and a flag where a note stands alone.
pub(super) fn draw_rhythm(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    beats: &[Beat],
    beat_positions: &[f32],
    tab_bottom_y: f32,
    header: &MeasureHeader,
) {
    let stem_x = |index: usize| beat_positions.get(index).copied().unwrap_or(0.0) + 3.0;
    let top = tab_bottom_y + STEM_TOP;
    let bottom = top + STEM_LENGTH;
    let stroke = Stroke::default()
        .with_width(1.0)
        .with_color(colors.foreground);

    for (index, beat) in beats.iter().enumerate() {
        // rests carry their own dot; empty beats carry nothing at all
        if beat.notes.is_empty() {
            continue;
        }
        let x = stem_x(index);
        if let Some(beams) = stem_beams(beat) {
            // a half note is stemmed only half way, so it reads as the
            // longer note
            let stem_top = if beat.duration.value == 2 {
                top + STEM_LENGTH / 2.0
            } else {
                top
            };
            frame.stroke(
                &Path::line(Point::new(x, stem_top), Point::new(x, bottom)),
                stroke,
            );
            // the dot clears the flag and the beams stacked up the stem,
            // but never leaves the stem itself
            let dot_y = (bottom - beams as f32 * BEAM_SPACING).max(stem_top);
            draw_duration_dot(frame, colors, beat, x + FLAG_WIDTH + 2.0, dot_y);
        } else {
            // a dotted whole note has no stem, yet keeps its dot, as
            // TuxGuitar draws it
            draw_duration_dot(frame, colors, beat, x, bottom);
        }
    }

    let beam_stroke = Stroke::default()
        .with_width(BEAM_THICKNESS)
        .with_color(colors.foreground);
    for (first, last) in beam_runs(beats, header.start, division_length(header)) {
        // a note standing alone carries flags, not a stub of beam
        if first == last {
            let flags = stem_beams(&beats[first]).unwrap_or(0);
            draw_flags(frame, colors, stem_x(first), bottom, flags);
            continue;
        }
        let deepest = (first..=last)
            .filter_map(|i| stem_beams(&beats[i]))
            .max()
            .unwrap_or(0);
        // one bar per level, broken wherever the notes under it are longer
        for level in 1..=deepest {
            let y = bottom - (level - 1) as f32 * BEAM_SPACING;
            let mut bar: Option<(usize, usize)> = None;
            for (index, beat) in beats.iter().enumerate().take(last + 1).skip(first) {
                let deep_enough = stem_beams(beat).is_some_and(|beams| beams >= level);
                match (&mut bar, deep_enough) {
                    (Some(open), true) => open.1 = index,
                    (None, true) => bar = Some((index, index)),
                    (Some(open), false) => {
                        draw_beam(frame, beam_stroke, *open, first, last, &stem_x, y);
                        bar = None;
                    }
                    (None, false) => {}
                }
            }
            if let Some(open) = bar {
                draw_beam(frame, beam_stroke, open, first, last, &stem_x, y);
            }
        }
    }
}

/// The flags of a lone short note, one per beam it would have: each curls
/// up from the foot of the stem and away to the right, as in print.
pub(super) fn draw_flags(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    stem_x: f32,
    bottom: f32,
    count: usize,
) {
    let stroke = Stroke::default()
        .with_width(1.6)
        .with_color(colors.foreground)
        .with_line_cap(LineCap::Round);
    for level in 0..count {
        let y = bottom - level as f32 * BEAM_SPACING;
        let flag = Path::new(|path| {
            path.move_to(Point::new(stem_x, y));
            path.bezier_curve_to(
                Point::new(stem_x + 1.0, y - 5.0),
                Point::new(stem_x + FLAG_WIDTH + 2.0, y - 5.0),
                Point::new(stem_x + FLAG_WIDTH, y - 13.0),
            );
        });
        frame.stroke(&flag, stroke);
    }
}

/// Which way a stub over a single note points: forward for a note standing
/// alone or opening its run, back towards the note it follows otherwise.
pub(super) const fn beam_leans_right(from: usize, run_first: usize, run_last: usize) -> bool {
    run_first == run_last || from == run_first
}

/// One bar of a beam. A bar over a single note is a stub, leaning towards
/// the run it belongs to.
pub(super) fn draw_beam(
    frame: &mut Frame<Renderer>,
    stroke: Stroke,
    bar: (usize, usize),
    run_first: usize,
    run_last: usize,
    stem_x: &impl Fn(usize) -> f32,
    y: f32,
) {
    let (from, to) = bar;
    let (x1, x2) = if from == to {
        let x = stem_x(from);
        if beam_leans_right(from, run_first, run_last) {
            (x, x + FLAG_WIDTH)
        } else {
            (x - FLAG_WIDTH, x)
        }
    } else {
        (stem_x(from), stem_x(to))
    };
    frame.stroke(&Path::line(Point::new(x1, y), Point::new(x2, y)), stroke);
}

/// The dot of a dotted duration, at the position its caller sets aside.
pub(super) fn draw_duration_dot(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    beat: &Beat,
    dot_x: f32,
    y: f32,
) {
    if !beat.duration.dotted && !beat.duration.double_dotted {
        return;
    }
    frame.fill(&Path::circle(Point::new(dot_x, y), 1.2), colors.foreground);
    if beat.duration.double_dotted {
        frame.fill(
            &Path::circle(Point::new(dot_x + 3.5, y), 1.2),
            colors.foreground,
        );
    }
}

/// The tuplet group of each beat, `None` outside one: consecutive beats
/// sharing a tuplet division, like TuxGuitar's `paintDivisionTypes`. A group
/// ends when the division changes or when its accumulated duration
/// completes a whole tuplet.
pub(super) fn tuplet_groups(beats: &[Beat]) -> Vec<Option<usize>> {
    let mut groups = Vec::with_capacity(beats.len());
    let mut next_group = 0;
    let mut run: Option<TupletRun> = None;
    for beat in beats {
        let duration = &beat.duration;
        if let Some(current) = &run {
            let whole_group = current.shortest > 0
                && current
                    .accumulated
                    .is_multiple_of(u32::from(current.enters) * current.shortest);
            if !duration.same_tuplet_division(current.division) || whole_group {
                run = None;
                next_group += 1;
            }
        }
        if duration.is_tuplet() && run.is_none() {
            run = Some(TupletRun::new(duration));
        }
        match &mut run {
            Some(current) => {
                current.extend(duration.time());
                groups.push(Some(next_group));
            }
            None => groups.push(None),
        }
    }
    groups
}

/// The brackets of the tuplets: `(group size, first beat x, last beat x)`
/// per group.
pub(super) fn tuplet_runs(beats: &[Beat], beat_positions: &[f32]) -> Vec<(u8, f32, f32)> {
    let groups = tuplet_groups(beats);
    let mut runs: Vec<(u8, f32, f32)> = Vec::new();
    let mut previous = None;
    for ((beat, &x), group) in beats.iter().zip(beat_positions).zip(groups) {
        match (group, runs.last_mut()) {
            (Some(g), Some(run)) if previous == Some(g) => run.2 = x,
            (Some(_), _) => runs.push((beat.duration.tuplet_enters, x, x)),
            (None, _) => {}
        }
        previous = group;
    }
    runs
}

/// A run of consecutive beats sharing one tuplet division.
pub(super) struct TupletRun<'a> {
    pub(super) division: &'a Duration,
    pub(super) enters: u8,
    pub(super) accumulated: u32,
    pub(super) shortest: u32,
}

impl<'a> TupletRun<'a> {
    pub(super) const fn new(division: &'a Duration) -> Self {
        Self {
            division,
            enters: division.tuplet_enters,
            accumulated: 0,
            shortest: 0,
        }
    }

    pub(super) const fn extend(&mut self, time: u32) {
        self.accumulated += time;
        if self.shortest == 0 || time < self.shortest {
            self.shortest = time;
        }
    }
}

/// A tuplet bracket: a horizontal line broken by the group size, with a
/// tick at each end pointing down towards the notes. A run covering a
/// single beat has no span to bracket, so only its label is drawn.
pub(super) fn draw_tuplet_bracket(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    enters: u8,
    x1: f32,
    x2: f32,
    y: f32,
) {
    const TICK: f32 = 4.0;
    const LABEL_SIZE: f32 = 8.0;
    let has_span = x2 > x1;
    // the notes are centred a few pixels right of their beat position
    let left = x1 + 1.0;
    let right = x2 + 6.0;
    let center = left + (right - left) / 2.0;
    let label = enters.to_string();
    let label_half = label.chars().count() as f32 * LABEL_SIZE / 4.0;

    if has_span {
        let stroke = Stroke::default()
            .with_width(0.8)
            .with_color(colors.foreground);
        frame.stroke(
            &Path::line(Point::new(left, y - TICK), Point::new(left, y)),
            stroke,
        );
        frame.stroke(
            &Path::line(Point::new(right, y - TICK), Point::new(right, y)),
            stroke,
        );
        // the arms stop short of the label, and are skipped when the label
        // already fills the span
        let arm_left_end = center - label_half - 1.0;
        let arm_right_start = center + label_half + 1.0;
        if arm_left_end > left {
            frame.stroke(
                &Path::line(Point::new(left, y), Point::new(arm_left_end, y)),
                stroke,
            );
        }
        if right > arm_right_start {
            frame.stroke(
                &Path::line(Point::new(arm_right_start, y), Point::new(right, y)),
                stroke,
            );
        }
    }

    let label_text = Text {
        shaping: Auto,
        content: label,
        color: colors.foreground,
        size: LABEL_SIZE.into(),
        position: Point::new(center, y - LABEL_SIZE / 2.0),
        align_x: Alignment::Center,
        font: UI_FONT,
        ..Text::default()
    };
    frame.fill_text(label_text);
}
