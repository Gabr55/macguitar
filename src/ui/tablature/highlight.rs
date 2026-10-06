//! What marks playback: the measure played, the beat played, the loop.

use super::colors::TablatureColors;
use super::layout::{CANVAS_MARGIN, Staff, focus_box_top};
use iced::widget::canvas::{Frame, Path};
use iced::{Point, Renderer, Size};

/// The band behind a looped measure: edge to edge, so the measures of a
/// loop join up, with an accent rule along its top.
pub(super) fn draw_loop_band(frame: &mut Frame<Renderer>, colors: TablatureColors, staff: Staff) {
    const OVERHANG: f32 = 7.0;
    let top = staff.top - OVERHANG;
    frame.fill_rectangle(
        Point::new(0.0, top),
        Size::new(staff.width, staff.height + OVERHANG * 2.0),
        colors.loop_band,
    );
    frame.fill_rectangle(
        Point::new(0.0, top),
        Size::new(staff.width, 2.0),
        colors.accent,
    );
}

/// The bar marking the beat being played, across the staff.
pub(super) fn draw_cursor(
    frame: &mut Frame<Renderer>,
    colors: TablatureColors,
    staff: Staff,
    beat_x: f32,
) {
    const WIDTH: f32 = 14.0;
    const OVERHANG: f32 = 6.0;
    let bar = Path::rounded_rectangle(
        Point::new(beat_x + 3.0 - WIDTH / 2.0, staff.top - OVERHANG),
        Size::new(WIDTH, staff.height + OVERHANG * 2.0),
        4.0.into(),
    );
    frame.fill(&bar, colors.cursor);
}

/// A soft tint behind the measure being played, under its notes, from the
/// measure number down to the rhythm.
pub(super) fn draw_focused_box(frame: &mut Frame<Renderer>, colors: TablatureColors, staff: Staff) {
    const SIDE: f32 = 8.0;
    let above = focus_box_top(staff.rows, staff.top);
    let below = staff.rows.staff_footer - CANVAS_MARGIN;
    let tint = Path::rounded_rectangle(
        Point::new(SIDE, staff.top - above),
        Size::new(staff.width - SIDE * 2.0, staff.height + above + below),
        6.0.into(),
    );
    frame.fill(&tint, colors.highlight);
}
