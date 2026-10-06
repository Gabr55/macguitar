//! A measure of tablature, drawn on its own canvas: what it holds, how it
//! answers the pointer, and how it is drawn.

use super::bars::{
    EndingSpan, draw_alternative_ending, draw_bar_line, draw_close_repeat, draw_end_section,
    draw_open_repeat, draw_open_section, draw_time_signature,
};
use super::colors::TablatureColors;
use super::effects::{
    SpanEffect, draw_effect_spans, shows_key_signature, span_crosses, triplet_feel_label,
};
use super::highlight::{draw_cursor, draw_focused_box, draw_loop_band};
use super::layout::{
    BEAT_LENGTH, HALF_BEAT_LENGTH, MEASURE_NOTES_PADDING, MIN_BEAT_WIDTH, MIN_MEASURE_WIDTH,
    RowSpacing, STRING_LINE_HEIGHT, Staff, beat_base_width, beat_natural_width, grace_gap_width,
    logical_width, lyric_extra_width, measure_height, paint_at_zoom, spacing_for_quarter,
};
use super::notes::draw_beat;
use super::rhythm::{draw_rhythm, draw_tuplet_bracket, tuplet_runs};
use crate::parser::model::{Beat, MeasureHeader, Song, Tempo, TempoUnit, TripletFeel};
use crate::ui::app::Message;
use crate::ui::widgets::UI_FONT;
use iced::advanced::mouse;
use iced::advanced::text::Shaping::Auto;
use iced::mouse::Cursor;
use iced::touch;
use iced::widget::canvas::{Cache, Event, Geometry, Path, Stroke, Text};
use std::sync::atomic::{AtomicBool, Ordering};

type Frame = iced::widget::canvas::Frame<Renderer>;
use iced::widget::text::Alignment;
use iced::widget::{Action, Canvas, canvas};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug)]
pub struct CanvasMeasure {
    pub measure_id: usize,
    pub(super) track_id: usize,
    pub(super) song: Rc<Song>,
    pub(super) is_focused: bool,
    /// Part of the stretch played in a loop.
    pub(super) in_loop: bool,
    pub(super) focused_beat: usize,
    pub(super) canvas_cache: Cache,
    pub(super) measure_len: f32,
    // natural width of each beat and their sum (immutable song data,
    // computed once instead of on every redraw)
    pub(super) beat_widths: Vec<f32>,
    pub(super) natural_beats_len: f32,
    pub total_measure_len: f32,
    pub vertical_measure_height: f32,
    pub(super) has_time_signature: bool,
    pub is_first_on_line: bool,
    // rows this measure needs, and the rows granted to its line
    pub(super) row_needs: RowSpacing,
    pub(super) row_spacing: RowSpacing,
    // colors the cached geometry was painted with
    pub(super) painted_colors: Cell<Option<TablatureColors>>,
    pub(super) has_tempo_label: bool,
    pub(super) has_key_signature: bool,
    pub(super) has_triplet_feel: bool,
    /// One syllable per beat, empty where the beat carries none.
    pub(super) lyrics: Vec<String>,
    /// How large the measure is drawn, 1.0 as laid out.
    pub(super) zoom: f32,
}

impl CanvasMeasure {
    pub fn new(
        measure_id: usize,
        track_id: usize,
        song: Rc<Song>,
        focused: bool,
        has_time_signature: bool,
        lyrics: Vec<String>,
    ) -> Self {
        let track = &song.tracks[track_id];
        let measure = &track.measures[measure_id];
        let measure_header = &song.measure_headers[measure_id];
        let beats = &measure.voices[0].beats;
        // the measure is spaced by its shortest note. A beat flagged empty
        // carries no time, so it has no say, as TuxGuitar's own skips it.
        // The fallback is for a measure with nothing to space by: a floor
        // here would widen the long notes it is meant to protect
        let quarter_spacing = beats
            .iter()
            .filter(|beat| !beat.empty)
            .map(|beat| spacing_for_quarter(&beat.duration))
            .fold(0.0_f32, f32::max);
        let quarter_spacing = if quarter_spacing > 0.0 {
            quarter_spacing
        } else {
            MIN_BEAT_WIDTH
        };
        let beat_widths: Vec<f32> = beats
            .iter()
            .enumerate()
            .map(|(i, beat)| {
                beat_natural_width(beat, beats.get(i + 1), quarter_spacing)
                    + lyric_extra_width(&lyrics, i, beat_base_width(beat, quarter_spacing))
            })
            .collect();
        let natural_beats_len: f32 = beat_widths.iter().sum();
        let measure_len = MIN_MEASURE_WIDTH.max(natural_beats_len);
        // total length of measure (padding on both sides)
        let mut total_measure_len = measure_len + MEASURE_NOTES_PADDING * 2.0;
        // extra space for time signature
        if has_time_signature {
            total_measure_len += BEAT_LENGTH;
        }
        // extra space for repeat open bar with dots
        if measure_header.repeat_open {
            total_measure_len += BEAT_LENGTH + HALF_BEAT_LENGTH;
        }
        // extra space for repeat close bar with dots
        if measure_header.repeat_close > 0 {
            total_measure_len += BEAT_LENGTH + HALF_BEAT_LENGTH;
        }
        let string_count = track.strings.len();
        // the tempo, key and feel are shown on the first measure and again
        // wherever they change
        let previous_header = measure_id.checked_sub(1).map(|p| &song.measure_headers[p]);
        let has_tempo_label =
            previous_header.is_none_or(|previous| measure_header.tempo != previous.tempo);
        let has_key_signature = shows_key_signature(measure_header, previous_header);
        let has_triplet_feel = previous_header.map_or(
            measure_header.triplet_feel != TripletFeel::None,
            |previous| measure_header.triplet_feel != previous.triplet_feel,
        );
        let has_lyrics = lyrics.iter().any(|syllable| !syllable.is_empty());
        let row_needs = RowSpacing::for_measure(
            measure,
            measure_header,
            has_tempo_label,
            has_key_signature || has_triplet_feel,
            has_lyrics,
        );
        let vertical_measure_height = measure_height(row_needs, string_count);
        Self {
            measure_id,
            track_id,
            song,
            is_focused: focused,
            in_loop: false,
            focused_beat: 0,
            canvas_cache: Cache::default(),
            measure_len,
            beat_widths,
            natural_beats_len,
            total_measure_len,
            vertical_measure_height,
            has_time_signature,
            is_first_on_line: false,
            row_needs,
            row_spacing: row_needs,
            painted_colors: Cell::new(None),
            has_tempo_label,
            has_key_signature,
            has_triplet_feel,
            lyrics,
            zoom: 1.0,
        }
    }

    /// The annotation rows this measure needs, for the line to merge.
    pub const fn row_needs(&self) -> RowSpacing {
        self.row_needs
    }

    /// Apply the rows granted to this measure's line.
    pub fn set_row_spacing(&mut self, row_spacing: RowSpacing) {
        if self.row_spacing != row_spacing {
            self.row_spacing = row_spacing;
            let string_count = self.song.tracks[self.track_id].strings.len();
            self.vertical_measure_height = measure_height(row_spacing, string_count);
            self.canvas_cache.clear();
        }
    }

    pub const fn set_first_on_line(&mut self, value: bool) {
        self.is_first_on_line = value;
    }

    pub fn view(&self) -> Element<'_, Message> {
        Canvas::new(self)
            .height(self.vertical_measure_height * self.zoom)
            .width(Length::Fixed(self.total_measure_len * self.zoom))
            .into()
    }

    /// View stretched to fill its row, weighted by the natural measure width.
    pub fn view_fill(&self) -> Element<'_, Message> {
        let portion = (self.total_measure_len.round() as u16).max(1);
        Canvas::new(self)
            .height(self.vertical_measure_height * self.zoom)
            .width(Length::FillPortion(portion))
            .into()
    }

    /// The fixed overhead width (padding, time signature, repeats) that doesn't scale with beats.
    pub(super) fn overhead_width(&self) -> f32 {
        self.total_measure_len - self.measure_len
    }

    /// Mark the measure as the one being played, or not.
    pub(super) fn set_focused(&mut self, focused: bool) {
        if self.is_focused != focused {
            self.is_focused = focused;
            self.focused_beat = 0;
            self.canvas_cache.clear();
        }
    }

    pub fn set_in_loop(&mut self, in_loop: bool) {
        if self.in_loop != in_loop {
            self.in_loop = in_loop;
            self.canvas_cache.clear();
        }
    }

    pub fn focus_beat(&mut self, beat_id: usize) {
        if self.focused_beat != beat_id {
            self.focused_beat = beat_id;
            self.canvas_cache.clear();
        }
    }

    pub fn clear_canvas_cache(&self) {
        self.canvas_cache.clear();
    }

    /// The beat under the given x position, mirroring the layout used by
    /// `draw` (clicks in the leading padding select the first beat, clicks
    /// past the last beat the last one).
    pub(super) fn beat_at_x(&self, x: f32, actual_width: f32) -> usize {
        let measure_header = &self.song.measure_headers[self.measure_id];
        let actual_measure_len = actual_width - self.overhead_width();
        let width_scale = if self.natural_beats_len > 0.0 {
            actual_measure_len / self.natural_beats_len
        } else {
            1.0
        };
        let mut beat_x = 0.0;
        if self.has_time_signature {
            beat_x += BEAT_LENGTH;
        }
        if measure_header.repeat_open {
            beat_x += BEAT_LENGTH;
        }
        beat_x += MEASURE_NOTES_PADDING;
        let beats = &self.song.tracks[self.track_id].measures[self.measure_id].voices[0].beats;
        for (beat_id, width) in self.beat_widths.iter().enumerate() {
            beat_x += width * width_scale;
            if x < beat_x {
                return beat_id;
            }
        }
        // past the last beat: the nearest visible one, never the zero-width
        // empty beat that may close the voice
        beats
            .iter()
            .rposition(|beat| !beat.empty)
            .unwrap_or_else(|| self.beat_widths.len().saturating_sub(1))
    }
}

/// What a measure remembers of the pointer.
#[derive(Debug, Default)]
pub struct MeasureInteraction {
    /// The pointer is over the measure: entering it is reported once, so a
    /// loop being drawn with the right button can grow over it.
    pub(super) hovered: bool,
    /// Where a finger touched the measure, until it moves away: a tap or a
    /// hold, rather than a scroll.
    pub(super) touched_at: Option<Point>,
}

/// How far a finger moves before a touch is a scroll, in logical pixels.
const TOUCH_SLOP: f32 = 10.0;

/// A loop is being drawn with a finger: its moves draw, they do not scroll.
static TOUCH_LOOPING: AtomicBool = AtomicBool::new(false);

/// Start or stop drawing a loop with a finger.
pub fn set_touch_looping(looping: bool) {
    TOUCH_LOOPING.store(looping, Ordering::Relaxed);
}

impl canvas::Program<Message> for CanvasMeasure {
    type State = MeasureInteraction;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        let mouse_event = match event {
            Event::Mouse(mouse_event) => mouse_event,
            Event::Touch(touch_event) => return self.on_touch(state, touch_event, bounds, cursor),
            _ => return None,
        };
        match mouse_event {
            // the left button places the playhead on a beat
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let cursor_position = cursor.position_in(bounds)?;
                let beat_id =
                    self.beat_at_x(cursor_position.x / self.zoom, bounds.width / self.zoom);
                log::debug!("Clicked on measure {} beat {beat_id}", self.measure_id);
                Some(Action::publish(Message::FocusMeasure(
                    self.measure_id,
                    beat_id,
                )))
            }
            // the right button draws the loop, from here to where it is let go
            mouse::Event::ButtonPressed(mouse::Button::Right) => {
                cursor.position_in(bounds)?;
                Some(Action::publish(Message::LoopFrom(self.measure_id)))
            }
            mouse::Event::CursorMoved { .. } => {
                let hovered = cursor.is_over(bounds);
                if hovered == state.hovered {
                    return None;
                }
                state.hovered = hovered;
                hovered.then(|| Action::publish(Message::LoopOver(self.measure_id)))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let colors = TablatureColors::of(theme);
        // the cached geometry holds the colors it was painted with, so a
        // change of theme has to repaint it
        if self.painted_colors.get() != Some(colors) {
            self.painted_colors.set(Some(colors));
            self.canvas_cache.clear();
        }
        // the cache redraws only when cleared or resized
        let tab = self.canvas_cache.draw(renderer, bounds.size(), |frame| {
            log::debug!("Re-drawing measure {}", self.measure_id);
            paint_at_zoom(frame, self.zoom, |frame| self.paint(frame, colors));
        });

        vec![tab]
    }
}

impl CanvasMeasure {
    /// A finger on the tablature, as on a phone: a tap places the playhead
    /// (in the app, once let go without moving), a hold loops the measure
    /// and a hold then a drag loops the measures it goes over; a finger
    /// moving at once scrolls.
    fn on_touch(
        &self,
        state: &mut MeasureInteraction,
        event: &touch::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        match event {
            touch::Event::FingerPressed { position, .. } => {
                let at = cursor.position_in(bounds)?;
                state.touched_at = Some(*position);
                state.hovered = true;
                let beat = self.beat_at_x(at.x / self.zoom, bounds.width / self.zoom);
                Some(Action::publish(Message::TouchDown(self.measure_id, beat)))
            }
            touch::Event::FingerMoved { position, .. } => {
                if TOUCH_LOOPING.load(Ordering::Relaxed) {
                    // the loop grows over the measures the finger enters,
                    // and the sheet stays still under it
                    let hovered = cursor.is_over(bounds);
                    let entered = hovered && !state.hovered;
                    state.hovered = hovered;
                    if !hovered {
                        return None;
                    }
                    let action = if entered {
                        Action::publish(Message::LoopOver(self.measure_id))
                    } else {
                        Action::request_redraw()
                    };
                    return Some(action.and_capture());
                }
                let origin = state.touched_at?;
                if origin.distance(*position) <= TOUCH_SLOP {
                    return None;
                }
                state.touched_at = None;
                Some(Action::publish(Message::TouchScrolled))
            }
            touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. } => {
                state.touched_at = None;
                state.hovered = false;
                None
            }
        }
    }

    /// Draw the measure, back to front.
    fn paint(&self, frame: &mut Frame, colors: TablatureColors) {
        let track = &self.song.tracks[self.track_id];
        let header = &self.song.measure_headers[self.measure_id];
        let staff = Staff {
            width: logical_width(frame),
            top: self.row_spacing.first_string_y(),
            height: STRING_LINE_HEIGHT * (track.strings.len() - 1) as f32,
            rows: self.row_spacing,
        };

        self.paint_backdrop(frame, colors, staff, track.strings.len());
        self.paint_opening(frame, colors, staff, header, track.strings.len());
        self.paint_labels(frame, colors, staff, header);

        // what follows is drawn over the highlight
        let colors = if self.is_focused {
            colors.highlighted()
        } else if self.in_loop {
            colors.looped()
        } else {
            colors
        };
        let beats = &track.measures[self.measure_id].voices[0].beats;
        // the neighbouring measures, for what carries over the bar lines
        let previous_beats = self
            .measure_id
            .checked_sub(1)
            .map(|previous| track.measures[previous].voices[0].beats.as_slice());
        let next_beats = track
            .measures
            .get(self.measure_id + 1)
            .map(|next| next.voices[0].beats.as_slice());

        let beat_positions = self.beat_positions(staff, header);
        self.paint_beats(frame, colors, staff, beats, &beat_positions, next_beats);
        Self::paint_effect_spans(
            frame,
            colors,
            staff,
            beats,
            &beat_positions,
            previous_beats,
            next_beats,
        );
        Self::paint_rhythm(frame, colors, staff, header, beats, &beat_positions);
        self.paint_closing(frame, colors, staff, header);
    }

    /// The loop band, the highlight of the measure played, and the strings.
    fn paint_backdrop(
        &self,
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        string_count: usize,
    ) {
        if self.in_loop {
            draw_loop_band(frame, colors, staff);
        }
        if self.is_focused {
            draw_focused_box(frame, colors, staff);
        }
        let stroke = Stroke::default()
            .with_width(0.8)
            .with_color(colors.string_line);
        for string in 0..string_count {
            let y = staff.top + string as f32 * STRING_LINE_HEIGHT;
            // from just right of the opening bar line to the end
            frame.stroke(
                &Path::line(Point::new(1.0, y), Point::new(staff.width, y)),
                stroke,
            );
        }
    }

    /// The opening bar line, and the time signature when it changes.
    fn paint_opening(
        &self,
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        header: &MeasureHeader,
        string_count: usize,
    ) {
        if header.repeat_open {
            draw_open_repeat(frame, colors, staff);
        } else if self.measure_id == 0 {
            draw_open_section(frame, colors, staff);
        } else if self.is_first_on_line {
            // further on a line, the previous measure's end line opens it
            draw_bar_line(frame, colors, staff, 0.0);
        }
        if self.has_time_signature {
            draw_time_signature(
                frame,
                colors,
                staff,
                &header.time_signature,
                string_count,
                header.repeat_open,
            );
        }
    }

    /// Tempo, section marker, key and feel, measure number and alternative
    /// ending, over the staff.
    fn paint_labels(
        &self,
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        header: &MeasureHeader,
    ) {
        let label = |content: String, color: Color, size: f32, position: Point| Text {
            shaping: Auto,
            content,
            color,
            size: size.into(),
            position,
            font: UI_FONT,
            ..Text::default()
        };
        // the marker follows the tempo on the same row
        let mut marker_x = MEASURE_NOTES_PADDING;
        if self.has_tempo_label {
            // the note is drawn rather than written: not every font has it
            let (unit, count) = tempo_mark(&header.tempo);
            let y = staff.rows.marker_y();
            let note_width = draw_tempo_note(frame, colors, unit, Point::new(1.0, y));
            let value = format!("= {count}");
            marker_x += note_width + (value.chars().count() * 7) as f32 + 14.0;
            frame.fill_text(label(
                value,
                colors.foreground,
                11.0,
                Point::new(note_width + 4.0, y),
            ));
        }
        if let Some(marker) = &header.marker {
            frame.fill_text(label(
                marker.title.clone(),
                colors.accent,
                10.0,
                Point::new(marker_x, staff.rows.marker_y()),
            ));
            marker_x += (marker.title.chars().count() * 6) as f32 + 12.0;
        }
        // where a jump lands, after the marker; the jumps at the end of the
        // measure, where they are taken
        if !header.targets.is_empty() {
            let targets: Vec<&str> = header.targets.iter().map(|t| t.label()).collect();
            frame.fill_text(label(
                targets.join("  "),
                colors.accent,
                10.0,
                Point::new(marker_x, staff.rows.marker_y()),
            ));
        }
        if !header.jumps.is_empty() {
            let jumps: Vec<&str> = header.jumps.iter().map(|j| j.label()).collect();
            frame.fill_text(Text {
                align_x: Alignment::Right,
                ..label(
                    jumps.join("  "),
                    colors.foreground,
                    10.0,
                    Point::new(logical_width(frame) - 4.0, staff.rows.marker_y()),
                )
            });
        }
        if self.has_key_signature || self.has_triplet_feel {
            let mut labels = Vec::new();
            if self.has_key_signature {
                labels.push(header.key_signature.to_string());
            }
            if self.has_triplet_feel {
                labels.push(triplet_feel_label(header.triplet_feel).to_string());
            }
            frame.fill_text(label(
                labels.join("  "),
                colors.foreground,
                9.0,
                Point::new(2.0, staff.rows.signature_y()),
            ));
        }
        // the measure being played stands out, the others recede
        let number_color = if self.is_focused {
            colors.accent
        } else {
            colors.muted
        };
        frame.fill_text(label(
            (self.measure_id + 1).to_string(),
            number_color,
            10.0,
            Point::new(0.0, staff.top - 15.0),
        ));
        if header.repeat_alternative > 0 {
            let previous = self
                .measure_id
                .checked_sub(1)
                .and_then(|i| self.song.measure_headers.get(i));
            draw_alternative_ending(
                frame,
                colors,
                header.repeat_alternative,
                EndingSpan::of(header, previous),
                staff.width,
                RowSpacing::ALT_ENDING_Y,
            );
        }
    }

    /// Where each beat starts. Every beat is placed before any is drawn:
    /// slides and slurs reach for the notes that follow them.
    fn beat_positions(&self, staff: Staff, header: &MeasureHeader) -> Vec<f32> {
        let mut start = MEASURE_NOTES_PADDING;
        if self.has_time_signature {
            start += BEAT_LENGTH;
        }
        if header.repeat_open {
            start += BEAT_LENGTH;
        }
        let scale = self.width_scale(staff.width);
        self.beat_widths
            .iter()
            .scan(start, |x, width| {
                let position = *x;
                *x += width * scale;
                Some(position)
            })
            .collect()
    }

    /// How much the beats stretch to fill `width`.
    fn width_scale(&self, width: f32) -> f32 {
        if self.natural_beats_len > 0.0 {
            (width - self.overhead_width()) / self.natural_beats_len
        } else {
            1.0
        }
    }

    /// The beats: the cursor on the one played, the notes and their
    /// effects, and the syllable sung on each.
    fn paint_beats(
        &self,
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        beats: &[Beat],
        beat_positions: &[f32],
        next_beats: Option<&[Beat]>,
    ) {
        let scale = self.width_scale(staff.width);
        let lyrics_y = staff.bottom() + staff.rows.staff_footer + staff.rows.tuplet;
        for (index, (beat, &x)) in beats.iter().zip(beat_positions).enumerate() {
            let is_played = self.is_focused && index == self.focused_beat;
            if is_played {
                draw_cursor(frame, colors, staff, x);
            }
            let beat_color = if is_played {
                colors.accent
            } else {
                colors.foreground
            };
            // the glyphs after the note may not reach the next beat's grace
            let next_grace = beats.get(index + 1).map_or(0.0, grace_gap_width);
            draw_beat(
                frame,
                colors,
                x,
                self.beat_widths[index] * scale - next_grace,
                scale,
                staff.top,
                staff.height,
                staff.rows,
                beat,
                beat_color,
                beats,
                beat_positions,
                index,
                next_beats,
            );
            if let Some(syllable) = self.lyrics.get(index).filter(|s| !s.is_empty()) {
                frame.fill_text(Text {
                    shaping: Auto,
                    content: syllable.clone(),
                    color: colors.foreground,
                    size: 8.0.into(),
                    position: Point::new(x, lyrics_y),
                    font: UI_FONT,
                    ..Text::default()
                });
            }
        }
    }

    /// Palm mute, let ring and vibrato, over the beats they last.
    fn paint_effect_spans(
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        beats: &[Beat],
        beat_positions: &[f32],
        previous_beats: Option<&[Beat]>,
        next_beats: Option<&[Beat]>,
    ) {
        for effect in SpanEffect::ALL {
            if staff.rows.span_height(effect) == 0.0 {
                continue;
            }
            draw_effect_spans(
                frame,
                colors,
                effect,
                beats,
                beat_positions,
                previous_beats.is_some_and(|previous| span_crosses(effect, previous, beats)),
                next_beats.is_some_and(|next| span_crosses(effect, beats, next)),
                staff.width,
                staff.rows.span_y(effect),
            );
        }
    }

    /// The rhythm under the staff, and the tuplet brackets under it, hooks
    /// up, as scores write them.
    fn paint_rhythm(
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        header: &MeasureHeader,
        beats: &[Beat],
        beat_positions: &[f32],
    ) {
        draw_rhythm(
            frame,
            colors.for_rhythm(),
            beats,
            beat_positions,
            staff.bottom(),
            header,
        );
        if staff.rows.tuplet > 0.0 {
            let bracket_y = staff.bottom() + staff.rows.staff_footer + staff.rows.tuplet / 2.0;
            for (enters, x1, x2) in tuplet_runs(beats, beat_positions) {
                draw_tuplet_bracket(frame, colors, enters, x1, x2, bracket_y);
            }
        }
    }

    /// The closing bar line: a repeat, the end of the song, or a plain line.
    fn paint_closing(
        &self,
        frame: &mut Frame,
        colors: TablatureColors,
        staff: Staff,
        header: &MeasureHeader,
    ) {
        let is_last = self.measure_id + 1 == self.song.measure_headers.len();
        if header.repeat_close > 0 {
            draw_close_repeat(frame, colors, staff, header.repeat_close);
        } else if is_last {
            draw_end_section(frame, colors, staff);
        } else {
            draw_bar_line(frame, colors, staff, staff.width);
        }
    }
}

/// The tempo as the score writes it, its note and count: 280 eighths, or
/// in quarters when written in a note drawn here as no other.
pub(super) fn tempo_mark(tempo: &Tempo) -> (TempoUnit, u32) {
    match tempo.written {
        Some((count, unit @ (TempoUnit::Eighth | TempoUnit::DottedQuarter))) => (unit, count),
        _ => (TempoUnit::Quarter, tempo.value),
    }
}

/// The note a tempo counts, its top-left at `at` in a line of 11 pixel
/// text: a head, a stem, a flag for an eighth and a dot when dotted.
/// Returns the width it takes.
fn draw_tempo_note(frame: &mut Frame, colors: TablatureColors, unit: TempoUnit, at: Point) -> f32 {
    let head = Point::new(at.x + 3.0, at.y + 11.0);
    let stem_x = head.x + 2.6;
    let stem_top = at.y + 1.5;
    frame.fill(&Path::circle(head, 2.8), colors.foreground);
    let stroke = Stroke::default()
        .with_width(1.1)
        .with_color(colors.foreground);
    frame.stroke(
        &Path::line(
            Point::new(stem_x, head.y - 0.5),
            Point::new(stem_x, stem_top),
        ),
        stroke,
    );
    let mut width = stem_x - at.x + 1.0;
    match unit {
        TempoUnit::Eighth => {
            let flag = Path::new(|path| {
                path.move_to(Point::new(stem_x, stem_top));
                path.bezier_curve_to(
                    Point::new(stem_x + 1.0, stem_top + 3.0),
                    Point::new(stem_x + 5.0, stem_top + 3.5),
                    Point::new(stem_x + 3.5, stem_top + 8.0),
                );
            });
            frame.stroke(&flag, stroke.with_width(1.3));
            width += 4.5;
        }
        TempoUnit::DottedQuarter => {
            frame.fill(
                &Path::circle(Point::new(stem_x + 3.0, head.y), 1.0),
                colors.foreground,
            );
            width += 4.0;
        }
        _ => {}
    }
    width
}
