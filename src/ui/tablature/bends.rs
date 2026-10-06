//! Bends and their release, drawn as arrows over the staff.

use super::colors::TablatureColors;
use super::layout::BEND_ARROW_WIDTH;
use crate::ui::widgets::UI_FONT;
use iced::advanced::text::Shaping::Auto;
use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{Point, Renderer};

// Amplitude labels in tones, indexed by bend value (half-semitone units)
pub(super) const BEND_AMPLITUDES: [&str; 13] = [
    "", "¼", "½", "¾", "1", "1¼", "1½", "1¾", "2", "2¼", "2½", "2¾", "3",
];

/// Draw a bend like TuxGuitar's `paintBend`: one curved arrow per movement
/// rising to (or releasing from) a band above the staff, with the reached
/// amplitude in tones next to the arrow tip. A bend without movements is a
/// held bend, drawn as a dashed line at band height across the beat.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_bend(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    movements: &[i32],
    note_position_x: f32,
    note_position_y: f32,
    beat_position_x: f32,
    width_per_beat: f32,
    width_scale: f32,
    measure_start_y: f32,
    show_amplitude: bool,
) {
    let stroke = Stroke::default()
        .with_width(0.8)
        .with_color(colors.foreground);
    let arrow_size = 2.5;
    // shrink the arrows with the beat when the measure is compressed, so
    // they stay within the width reserved by beat_natural_width
    let arrow_width = BEND_ARROW_WIDTH * width_scale.min(1.0);
    // vertical extent: from the note's fret number up to a band above the staff
    let y_low = note_position_y + 4.0;
    let y_high = measure_start_y - 8.0;
    let amplitude_y = measure_start_y - 18.0;

    if movements.is_empty() {
        // held bend: dashed line at band height until the end of the beat
        let x_start = note_position_x - 4.0;
        let x_end = (beat_position_x + width_per_beat - 6.0).max(x_start + 5.0);
        let dashed = Stroke {
            line_dash: LineDash {
                segments: &[2.5, 2.5],
                offset: 0,
            },
            ..stroke
        };
        frame.stroke(
            &Path::line(Point::new(x_start, y_high), Point::new(x_end, y_high)),
            dashed,
        );
        return;
    }

    let mut x0 = note_position_x + 7.0;
    let mut first_movement = true;
    for &movement in movements {
        let release = movement < 0;
        let (y0, y1, direction) = if release {
            (y_high, y_low, -1.0)
        } else {
            (y_low, y_high, 1.0)
        };
        let x1 = x0 + arrow_width;

        // pre-bend: the note starts already bent, marked by a vertical
        // line with an upward arrowhead before the release
        if first_movement && release {
            frame.stroke(&Path::line(Point::new(x0, y0), Point::new(x0, y1)), stroke);
            frame.stroke(
                &Path::line(Point::new(x0, y0), Point::new(x0 - 2.0, y0 + 2.0)),
                stroke,
            );
            frame.stroke(
                &Path::line(Point::new(x0, y0), Point::new(x0 + 2.0, y0 + 2.0)),
                stroke,
            );
        }

        // curved arrow body
        let body = Path::new(|p| {
            p.move_to(Point::new(x0, y0));
            p.line_to(Point::new(x0 + 1.0, y0));
            p.bezier_curve_to(
                Point::new(x0 + 1.0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
            );
        });
        frame.stroke(&body, stroke);

        // arrowhead
        let tip = Point::new(x1, y1);
        frame.stroke(
            &Path::line(
                tip,
                Point::new(x1 - arrow_size, y1 + arrow_size * direction),
            ),
            stroke,
        );
        frame.stroke(
            &Path::line(
                tip,
                Point::new(x1 + arrow_size, y1 + arrow_size * direction),
            ),
            stroke,
        );

        // amplitude reached by the movement (releases only label the pre-bend)
        if show_amplitude && (!release || first_movement) {
            let mut x_amplitude = if release { x0 } else { x1 };
            let amplitude = movement.unsigned_abs() as usize;
            if !amplitude.is_multiple_of(4) {
                // longer label (fraction), shift left to stay near the tip
                x_amplitude -= 4.0;
            }
            if let Some(label) = BEND_AMPLITUDES.get(amplitude) {
                let amplitude_text = Text {
                    shaping: Auto,
                    content: (*label).to_string(),
                    color: colors.foreground,
                    size: 8.0.into(),
                    position: Point::new(x_amplitude, amplitude_y),
                    font: UI_FONT,
                    ..Text::default()
                };
                frame.fill_text(amplitude_text);
            }
        }

        first_movement = false;
        x0 = x1;
    }
}
