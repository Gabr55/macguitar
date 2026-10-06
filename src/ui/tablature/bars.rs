//! Bar lines, repeats, alternative endings and time signatures.

use super::colors::TablatureColors;
use super::layout::{BEAT_LENGTH, HALF_BEAT_LENGTH, STRING_LINE_HEIGHT, Staff};
use crate::parser::model::{MeasureHeader, TimeSignature};
use crate::ui::widgets::{UI_FONT, UI_FONT_BOLD};
use iced::advanced::text::Shaping::Auto;
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Point, Renderer};

const THIN_LINE: f32 = 1.5;
const THICK_LINE: f32 = 4.0;

/// A bar line across the staff at `x`.
fn bar_line(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
    x: f32,
    width: f32,
) {
    frame.stroke(
        &Path::line(Point::new(x, staff.top), Point::new(x, staff.bottom())),
        Stroke::default()
            .with_width(width)
            .with_color(colors.foreground),
    );
}

/// A plain bar line at `x`.
pub(super) fn draw_bar_line(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
    x: f32,
) {
    bar_line(frame, colors, staff, x, THIN_LINE);
}

/// The opening of the song: a thick line, then a thin one.
pub(super) fn draw_open_section(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
) {
    bar_line(frame, colors, staff, 0.0, THICK_LINE);
    bar_line(frame, colors, staff, 6.0, THIN_LINE);
}

/// The end of the song: a thin line, then a thick one.
pub(super) fn draw_end_section(frame: &mut Frame<Renderer>, colors: TablatureColors, staff: Staff) {
    bar_line(frame, colors, staff, staff.width - 8.0, THIN_LINE);
    bar_line(frame, colors, staff, staff.width - 2.0, THICK_LINE);
}

/// The start of a repeat: the opening lines, and the dots after them.
pub(super) fn draw_open_repeat(frame: &mut Frame<Renderer>, colors: TablatureColors, staff: Staff) {
    draw_open_section(frame, colors, staff);
    draw_repeat_dots(frame, colors, staff, HALF_BEAT_LENGTH);
}

/// The end of a repeat: the dots, the closing lines, and how many times
/// it is played when that is more than twice; `repeat_count` counts the
/// repeats, one less than the plays.
pub(super) fn draw_close_repeat(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
    repeat_count: i8,
) {
    draw_end_section(frame, colors, staff);
    draw_repeat_dots(frame, colors, staff, staff.width - HALF_BEAT_LENGTH);
    // played twice goes without saying; more is written as the total
    let plays = i32::from(repeat_count) + 1;
    if plays <= 2 {
        return;
    }
    frame.fill_text(Text {
        shaping: Auto,
        content: format!("x{plays}"),
        color: colors.foreground,
        size: 9.0.into(),
        position: Point::new(staff.width - 12.0, staff.top - 15.0),
        font: UI_FONT,
        ..Text::default()
    });
}

/// The two dots of a repeat sign, at `x`.
fn draw_repeat_dots(frame: &mut Frame<Renderer>, colors: TablatureColors, staff: Staff, x: f32) {
    let stroke = Stroke::default()
        .with_width(2.0)
        .with_color(colors.foreground);
    for third in [1.0, 2.0] {
        let center = Point::new(x, staff.top + staff.height * third / 3.0);
        frame.stroke(&Path::circle(center, 1.0), stroke);
    }
}

/// How a measure sits under the bracket of its alternative ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EndingSpan {
    /// The bracket starts here: its corner and its passes are drawn.
    pub(super) opens: bool,
    /// The bracket ends here, going back to repeat: its end hooks down.
    pub(super) closes: bool,
}

impl EndingSpan {
    /// The span of `header` in its bracket, `previous` being the measure
    /// before it: a bracket carries on over the measures of the same
    /// ending until one closes the repeat.
    pub(super) fn of(header: &MeasureHeader, previous: Option<&MeasureHeader>) -> Self {
        let continued = previous.is_some_and(|previous| {
            previous.repeat_alternative == header.repeat_alternative && previous.repeat_close == 0
        });
        Self {
            opens: !continued,
            closes: header.repeat_close > 0,
        }
    }
}

/// The bracket over an alternative ending, with the passes it is played
/// on: "1.", "2.", "1.3."... A bracket over several measures is labelled
/// once, at its start.
pub(super) fn draw_alternative_ending(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    endings: u8,
    span: EndingSpan,
    width: f32,
    bracket_y: f32,
) {
    const HEIGHT: f32 = 10.0;
    const START: f32 = 2.0;
    let stroke = Stroke::default()
        .with_width(1.0)
        .with_color(colors.foreground);
    let start = if span.opens { START } else { 0.0 };
    let end = if span.closes { width - START } else { width };
    frame.stroke(
        &Path::line(Point::new(start, bracket_y), Point::new(end, bracket_y)),
        stroke,
    );
    for (hooked, x) in [(span.opens, start), (span.closes, end)] {
        if hooked {
            frame.stroke(
                &Path::line(Point::new(x, bracket_y), Point::new(x, bracket_y + HEIGHT)),
                stroke,
            );
        }
    }
    if span.opens {
        frame.fill_text(Text {
            shaping: Auto,
            content: ending_label(endings),
            color: colors.foreground,
            size: 9.0.into(),
            position: Point::new(START + 3.0, bracket_y),
            font: UI_FONT,
            ..Text::default()
        });
    }
}

/// The passes of an alternative ending, from its bit mask: 1 → "1.",
/// 0b101 → "1.3.".
fn ending_label(endings: u8) -> String {
    (0..8_u8)
        .filter(|bit| endings & (1 << bit) != 0)
        .map(|bit| format!("{}.", bit + 1))
        .collect()
}

/// The time signature, after the opening lines (and the repeat dots).
pub(super) fn draw_time_signature(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
    time_signature: &TimeSignature,
    string_count: usize,
    after_repeat: bool,
) {
    let x = if after_repeat {
        BEAT_LENGTH
    } else {
        HALF_BEAT_LENGTH
    };
    // centred on the staff when it has more than four strings
    let y = STRING_LINE_HEIGHT * string_count.saturating_sub(4) as f32 / 2.0;
    frame.fill_text(Text {
        shaping: Auto,
        content: format!(
            "{}\n{}",
            time_signature.numerator, time_signature.denominator.value
        ),
        color: colors.foreground,
        size: 17.into(),
        position: Point::new(x, staff.top - 1.0 + y),
        font: UI_FONT_BOLD,
        ..Text::default()
    });
}

#[cfg(test)]
mod tests {
    use super::ending_label;

    #[test]
    fn endings_are_labelled_by_pass() {
        assert_eq!(ending_label(0b1), "1.");
        assert_eq!(ending_label(0b10), "2.");
        assert_eq!(ending_label(0b101), "1.3.");
    }
}
