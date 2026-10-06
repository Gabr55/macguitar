//! The tablature: measures laid out in lines across the sheet, and turned
//! like pages as playback moves on.

mod bars;
mod bends;
mod chords;
mod colors;
#[cfg(test)]
mod drawing_tests;
mod effects;
mod highlight;
mod layout;
mod measure;
mod notes;
mod rhythm;

use crate::audio::midi_event::FIRST_TICK;
use crate::audio::playback_order::playback_tick;
use crate::parser::model::Song;
use crate::ui::app::Message;
use iced::widget::{Id, Row, column, scrollable};
use iced::{Element, Length};
use layout::RowSpacing;
use measure::CanvasMeasure;
pub use measure::set_touch_looping;
use std::collections::BTreeMap;
use std::rc::Rc;

const INNER_PADDING: f32 = 10.0;
const SCROLLBAR_WIDTH: f32 = 10.0; // iced default scrollbar width (iced_widget/src/scrollable.rs)

pub struct Tablature {
    pub song: Rc<Song>,
    pub track_id: usize,
    pub canvas_measures: Vec<CanvasMeasure>,
    line_heights: Vec<f32>, // rendered height of each line, in order
    viewport_height: f32,   // visible height of the tablature container
    page_top_line: u32,     // line currently shown at the top of the view
    focused_measure: usize,
    line_tracker: LineTracker,
    pub scroll_id: Id,
    measure_per_tick: BTreeMap<u32, u32>, // tick to measure index as u32
    /// Measures played in a loop, first and last.
    loop_range: Option<(usize, usize)>,
    /// Where the highlight is within the focused measure, in score ticks:
    /// the start of the beat clicked, or the position playback reached.
    /// Changing tracks keeps the measure and finds the beat of the new
    /// track sounding at that moment, whatever moved the highlight there.
    focus_tick: u32,
    /// How large the measures are drawn, 1.0 as laid out.
    zoom: f32,
    /// Width the rows have, in pixels on screen.
    available_width: f32,
}

impl Tablature {
    pub fn new(
        song: Rc<Song>,
        track_id: usize,
        scroll_id: Id,
        playback_order: &[(usize, i64)],
    ) -> Self {
        let measure_count = song.measure_headers.len();
        // build tick-to-measure map including expanded repeat ticks
        let mut measure_per_tick = BTreeMap::new();
        for (measure_index, tick_offset) in playback_order {
            let header = &song.measure_headers[*measure_index];
            let tick = playback_tick(header.start, *tick_offset);
            measure_per_tick.insert(tick, *measure_index as u32);
        }
        let mut tab = Self {
            song,
            track_id,
            canvas_measures: Vec::with_capacity(measure_count),
            line_heights: Vec::new(),
            viewport_height: 0.0,
            page_top_line: 1,
            focused_measure: 0,
            line_tracker: LineTracker::default(),
            scroll_id,
            loop_range: None,
            focus_tick: song_start(&measure_per_tick),
            measure_per_tick,
            zoom: 1.0,
            available_width: 0.0,
        };
        tab.load_measures();
        tab
    }

    pub fn load_measures(&mut self) {
        self.canvas_measures.clear();

        let syllables = lyric_syllables(&self.song, self.track_id);
        let measures = self.song.tracks[self.track_id].measures.len();
        for i in 0..measures {
            // the first measure always shows its time signature, the others
            // only when it changed
            let has_time_signature = i.checked_sub(1).is_none_or(|previous| {
                self.song.measure_headers[i].time_signature
                    != self.song.measure_headers[previous].time_signature
            });
            let mut measure = CanvasMeasure::new(
                i,
                self.track_id,
                self.song.clone(),
                self.focused_measure == i,
                has_time_signature,
                syllables.get(i).cloned().unwrap_or_default(),
            );
            measure.zoom = self.zoom;
            self.canvas_measures.push(measure);
        }
        // a new track keeps the loop drawn
        self.set_loop(self.loop_range);
        self.lay_out();
    }

    /// Break the measures into lines for the width at the zoom, and give
    /// each line its rows.
    fn lay_out(&mut self) {
        // the measures are laid out in their own units, the zoom taken out
        self.line_tracker =
            LineTracker::make(&self.canvas_measures, self.available_width / self.zoom);
        self.update_first_on_line();
        self.update_line_spacing();
    }

    /// Draw the measures larger or smaller: fewer or more fit a line.
    pub fn set_zoom(&mut self, zoom: f32) {
        if (self.zoom - zoom).abs() < f32::EPSILON {
            return;
        }
        self.zoom = zoom;
        for measure in &mut self.canvas_measures {
            measure.zoom = zoom;
            measure.canvas_cache.clear();
        }
        self.lay_out();
    }

    pub fn update_container_size(&mut self, width: f32, height: f32) {
        self.viewport_height = height;
        // the padding and the scrollbar take their share
        self.available_width = width - (INNER_PADDING * 2.0) - SCROLLBAR_WIDTH;
        self.lay_out();
    }

    /// Give every measure of a line the same annotation rows, sized for the
    /// most demanding measure on it, so their staves stay aligned.
    fn update_line_spacing(&mut self) {
        let Some(line_count) = self.line_tracker.line_count() else {
            self.line_heights.clear();
            return;
        };
        let mut per_line = vec![RowSpacing::default(); line_count];
        for cm in &self.canvas_measures {
            let line = self.line_tracker.get_line(cm.measure_id) as usize - 1;
            per_line[line].merge(cm.row_needs());
        }
        for cm in &mut self.canvas_measures {
            let line = self.line_tracker.get_line(cm.measure_id) as usize - 1;
            cm.set_row_spacing(per_line[line]);
        }
        // line heights drive the playback scroll offset
        self.line_heights = vec![0.0; line_count];
        for cm in &self.canvas_measures {
            let line = self.line_tracker.get_line(cm.measure_id) as usize - 1;
            self.line_heights[line] =
                self.line_heights[line].max(cm.vertical_measure_height * self.zoom);
        }
    }

    /// Update the `is_first_on_line` flag on each measure based on the line tracker
    /// and clear caches for measures that changed line assignment.
    fn update_first_on_line(&mut self) {
        let mut prev_line = 0_u32;
        for cm in &mut self.canvas_measures {
            let line = self.line_tracker.get_line(cm.measure_id);
            let is_first = line != prev_line;
            if cm.is_first_on_line != is_first {
                cm.set_first_on_line(is_first);
                cm.clear_canvas_cache();
            }
            prev_line = line;
        }
    }

    /// The measure containing `tick`, with the tick mapped back onto the
    /// measure's own timeline (playback repeats shift it forward).
    fn measure_and_original_tick(&self, tick: u32) -> (usize, u32) {
        // range scan on `measure_per_tick` to find measure index and playback start tick
        let (playback_start, measure_index) = self
            .measure_per_tick
            .range(0..=tick)
            .next_back()
            // a tick before the first measure means playback has not reached
            // it yet: the cursor belongs on that first measure
            .or_else(|| self.measure_per_tick.iter().next())
            .map(|(&event_tick, &m_id)| (event_tick, m_id as usize))
            .unwrap_or_else(|| {
                log::warn!("No measure index found for tick:{tick}");
                (0, 0)
            });

        // compute tick offset between playback position and original measure position
        let original_start = self.song.measure_headers[measure_index].start;
        let tick_offset = i64::from(playback_start) - i64::from(original_start);
        let original_tick = (i64::from(tick) - tick_offset).max(i64::from(original_start)) as u32;
        (measure_index, original_tick)
    }

    /// The beat of `measure` sounding at `score_tick` on `track_id`: the
    /// last one starting at or before it.
    fn beat_at(&self, track_id: usize, measure: usize, score_tick: u32) -> usize {
        self.song.tracks[track_id].measures[measure].voices[0]
            .beats
            .partition_point(|beat| beat.start <= score_tick)
            .saturating_sub(1)
    }

    /// Move the highlight to the beat at the given playback tick. Scrolling
    /// is driven separately by [`Self::page_scroll_offset`].
    pub fn focus_on_tick(&mut self, tick: u32) {
        // the first tick of the song, set on stop: its first beat
        let (measure, score_tick) = if tick == FIRST_TICK {
            (0, self.song.measure_headers[0].start)
        } else {
            self.measure_and_original_tick(tick)
        };
        // beat notifications coalesce, so the first tick seen in a measure
        // may already be past its first beat
        let beat = self.beat_at(self.track_id, measure, score_tick);
        self.focus_tick = score_tick;
        self.move_focus(measure, beat);
    }

    pub fn focus_on_measure(&mut self, measure: usize) {
        self.focus_on_measure_beat(measure, 0);
    }

    /// Move the highlight to `beat` of `measure`, as a click does; a measure
    /// past the end of the track is ignored.
    pub fn focus_on_measure_beat(&mut self, measure: usize, beat: usize) {
        let Some(header) = self.song.measure_headers.get(measure) else {
            return;
        };
        self.focus_tick = header.start + self.beat_tick_offset(measure, beat);
        self.move_focus(measure, beat);
    }

    /// Highlight `beat` of `measure`, and only it.
    fn move_focus(&mut self, measure: usize, beat: usize) {
        if measure >= self.canvas_measures.len() {
            return;
        }
        if measure != self.focused_measure {
            if let Some(previous) = self.canvas_measures.get_mut(self.focused_measure) {
                previous.set_focused(false);
            }
            self.focused_measure = measure;
            self.canvas_measures[measure].set_focused(true);
        }
        self.canvas_measures[measure].focus_beat(beat);
    }

    /// Tick offset of a beat from the start of its measure.
    pub fn beat_tick_offset(&self, measure_id: usize, beat_id: usize) -> u32 {
        let measure_start = self.song.measure_headers[measure_id].start;
        self.song.tracks[self.track_id].measures[measure_id].voices[0]
            .beats
            .get(beat_id)
            .map_or(0, |beat| beat.start.saturating_sub(measure_start))
    }

    pub const fn focused_measure(&self) -> usize {
        self.focused_measure
    }

    pub const fn measure_count(&self) -> usize {
        self.canvas_measures.len()
    }

    /// Scroll needed to bring `measure_id` into view, a page at a time: the
    /// view holds still while the measure is on the page being read, then
    /// turns to the page starting on its line.
    ///
    /// The turn comes one line early, as playback reaches the last line of
    /// the page rather than once it has left: that line moves to the top
    /// with a fresh page below it, so the eye arrives before the sound. It
    /// lands on a line change, which is a natural break in the reading.
    pub fn page_scroll_offset(&mut self, measure_id: usize) -> Option<f32> {
        let line = self.line_tracker.get_line(measure_id);
        // already reading from the top of the view
        if line == self.page_top_line {
            return None;
        }
        let last_line = self.page_top_line + self.visible_lines(self.page_top_line) - 1;
        if line > self.page_top_line && line < last_line {
            return None;
        }
        self.page_top_line = line;
        Some(self.offset_for_line_top(line))
    }

    /// Scroll offset putting `line` at the top of the view.
    fn offset_for_line_top(&self, line: u32) -> f32 {
        if line <= 1 {
            return 0.0;
        }
        let lines_above = (line - 1) as usize;
        INNER_PADDING + self.line_heights.iter().take(lines_above).sum::<f32>()
    }

    /// How many lines fit in the view when `from_line` is at the top. Always
    /// at least one, so a line taller than the view still turns the page.
    fn visible_lines(&self, from_line: u32) -> u32 {
        let mut used = 0.0;
        let mut count = 0;
        for height in self.line_heights.iter().skip((from_line - 1) as usize) {
            if used + height > self.viewport_height {
                break;
            }
            used += height;
            count += 1;
        }
        count.max(1)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let has_layout = self.line_tracker.tablature_container_width > 0.0;

        let content: Element<Message> = if has_layout {
            // Build explicit rows using LineTracker line assignments.
            // Each measure uses FillPortion to stretch and fill the row width.
            let row_width = self.line_tracker.tablature_container_width * self.zoom;
            let mut rows: Vec<Element<Message>> = Vec::new();
            let mut current_row: Vec<Element<Message>> = Vec::new();
            let mut current_line = 0_u32;

            for cm in &self.canvas_measures {
                let line = self.line_tracker.get_line(cm.measure_id);
                if line != current_line && !current_row.is_empty() {
                    rows.push(
                        Row::with_children(std::mem::take(&mut current_row))
                            .width(row_width)
                            .into(),
                    );
                }
                current_line = line;
                current_row.push(cm.view_fill());
            }
            if !current_row.is_empty() {
                rows.push(Row::with_children(current_row).width(row_width).into());
            }

            column(rows).padding(INNER_PADDING).into()
        } else {
            // Before container size is known, use wrapping layout with natural widths
            let measure_elements = self
                .canvas_measures
                .iter()
                .map(|m| m.view())
                .collect::<Vec<Element<Message>>>();

            column![Row::with_children(measure_elements).wrap()]
                .padding(INNER_PADDING)
                .into()
        };

        scrollable(content)
            .id(self.scroll_id.clone())
            .height(Length::Fill)
            .width(Length::Fill)
            .direction(scrollable::Direction::default())
            .style(crate::ui::theme::scrollable_style)
            .into()
    }

    /// Mark the measures `first..=last` as looped, or none.
    pub fn set_loop(&mut self, range: Option<(usize, usize)>) {
        self.loop_range = range;
        for measure in &mut self.canvas_measures {
            let in_loop =
                range.is_some_and(|(first, last)| (first..=last).contains(&measure.measure_id));
            measure.set_in_loop(in_loop);
        }
    }

    /// Show another track, keeping the position: the same measure, and the
    /// beat of the new track sounding where the highlight was. Returns the
    /// scroll offset bringing that measure's line to the top, as its place
    /// changes with the new track's layout.
    pub fn update_track(&mut self, track: usize) -> Option<f32> {
        if track == self.track_id {
            return None;
        }
        self.track_id = track;
        self.load_measures();
        let measure = self.focused_measure;
        let beat = self.beat_at(track, measure, self.focus_tick);
        self.move_focus(measure, beat);
        self.page_top_line = self.line_tracker.get_line(measure);
        Some(self.offset_for_line_top(self.page_top_line))
    }
}

/// The score tick the song starts at: that of its first played measure.
fn song_start(measure_per_tick: &BTreeMap<u32, u32>) -> u32 {
    measure_per_tick
        .keys()
        .next()
        .copied()
        .unwrap_or(FIRST_TICK)
}

/// Lay the song's lyrics out over the track, one syllable per beat that
/// sounds, like TuxGuitar. Each line starts at its own measure.
///
/// The result is indexed by measure, then by beat.
fn lyric_syllables(song: &Song, track_id: usize) -> Vec<Vec<String>> {
    let measures = &song.tracks[track_id].measures;
    let mut per_measure = vec![Vec::new(); measures.len()];
    let Some(lyrics) = &song.lyrics else {
        return per_measure;
    };
    // lyrics belong to one track, numbered from one
    if lyrics.track_choice != song.tracks[track_id].number {
        return per_measure;
    }
    for (start_measure, text) in &lyrics.lines {
        let mut syllables = text.split_whitespace();
        let first_measure = (*start_measure).max(1) as usize - 1;
        'line: for (measure_index, measure) in measures.iter().enumerate().skip(first_measure) {
            let beats = &measure.voices[0].beats;
            let row = &mut per_measure[measure_index];
            if row.len() < beats.len() {
                row.resize(beats.len(), String::new());
            }
            for (beat_index, beat) in beats.iter().enumerate() {
                if beat.notes.is_empty() {
                    continue;
                }
                let Some(syllable) = syllables.next() else {
                    break 'line;
                };
                row[beat_index] = syllable.to_string();
            }
        }
    }
    per_measure
}

#[derive(Default)]
struct LineTracker {
    measure_to_line: Vec<u32>, // measure id to line number
    tablature_container_width: f32,
}

impl LineTracker {
    pub fn make(measures: &[CanvasMeasure], tablature_container_width: f32) -> Self {
        let widths: Vec<f32> = measures.iter().map(|m| m.total_measure_len).collect();
        Self::make_from_widths(&widths, tablature_container_width)
    }

    fn make_from_widths(widths: &[f32], tablature_container_width: f32) -> Self {
        let mut line_tracker = Self {
            measure_to_line: vec![0; widths.len()],
            tablature_container_width,
        };
        let mut current_line = 1;
        let mut horizontal_cursor = 0.0;
        for (i, &width) in widths.iter().enumerate() {
            horizontal_cursor += width;
            if horizontal_cursor > tablature_container_width {
                current_line += 1;
                horizontal_cursor = width;
            }
            line_tracker.measure_to_line[i] = current_line;
        }
        line_tracker
    }

    pub fn get_line(&self, measure_id: usize) -> u32 {
        self.measure_to_line[measure_id]
    }

    /// Number of lines, or `None` when there is no measure at all.
    fn line_count(&self) -> Option<usize> {
        self.measure_to_line.last().map(|&last| last as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::playback_order::compute_playback_order;
    use crate::parser::model::Lyrics;
    use crate::parser::test_support::parse_gp_file;

    fn load_tablature(width: f32, height: f32) -> Tablature {
        let song = Rc::new(parse_gp_file("test-files/effects.gp5").unwrap());
        let order = compute_playback_order(&song.measure_headers);
        let mut tab = Tablature::new(song, 0, Id::new("test-scroll"), &order);
        tab.update_container_size(width, height);
        tab
    }

    #[test]
    fn switching_tracks_preserves_position_between_different_rhythms() {
        let mut song = parse_gp_file("test-files/effects.gp5").unwrap();
        let mut other = parse_gp_file("test-files/effects.gp5")
            .unwrap()
            .tracks
            .remove(0);
        let measure = song.tracks[0]
            .measures
            .iter()
            .enumerate()
            .skip(3)
            .find(|(_, measure)| measure.voices[0].beats.len() >= 4)
            .map(|(index, _)| index)
            .unwrap();
        let tick = song.tracks[0].measures[measure].voices[0].beats[2].start + 1;
        other.measures[measure].voices[0].beats.remove(1);
        let other_id = song.tracks.len();
        song.tracks.push(other);
        let order = compute_playback_order(&song.measure_headers);
        let mut tab = Tablature::new(Rc::new(song), 0, Id::unique(), &order);
        tab.update_container_size(450.0, 300.0);
        tab.focus_on_tick(tick);
        assert_eq!(tab.focused_measure(), measure);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 2);
        tab.set_loop(Some((measure, measure + 1)));

        // no playback notification arrives while paused
        let scroll = tab.update_track(other_id).unwrap();
        assert_eq!(tab.focused_measure(), measure);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 1);
        assert_eq!(
            tab.canvas_measures.iter().filter(|m| m.is_focused).count(),
            1
        );
        assert_eq!(tab.loop_range, Some((measure, measure + 1)));
        assert!(scroll > 0.0);
        assert_eq!(
            scroll,
            tab.offset_for_line_top(tab.line_tracker.get_line(measure))
        );
        assert_eq!(tab.page_scroll_offset(measure), None);

        tab.update_track(0);
        assert_eq!(tab.focused_measure(), measure);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 2);
        assert_eq!(tab.update_track(0), None);
    }

    #[test]
    fn switching_tracks_preserves_position_in_a_repeat() {
        let path = "test-files/playback-repeat-close-without-start-at-beginning.gp5";
        let mut song = parse_gp_file(path).unwrap();
        let other_id = song.tracks.len();
        song.tracks
            .push(parse_gp_file(path).unwrap().tracks.remove(0));
        let order = compute_playback_order(&song.measure_headers);
        let &(measure, offset) = order.iter().find(|(_, offset)| *offset > 0).unwrap();
        let tick = playback_tick(song.measure_headers[measure].start, offset);
        let mut tab = Tablature::new(Rc::new(song), 0, Id::unique(), &order);
        tab.focus_on_tick(tick);
        tab.update_track(other_id);
        assert_eq!(tab.focused_measure(), measure);
        assert!(tab.canvas_measures[measure].is_focused);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 0);
    }

    #[test]
    fn lyrics_follow_the_beats_that_sound() {
        // the fixtures carry no lyrics: sing a few words over the effects,
        // whose opening measures are full of rests and dead notes
        let mut song = parse_gp_file("test-files/effects.gp5").unwrap();
        song.lyrics = Some(Lyrics {
            track_choice: 1,
            lines: vec![(1, "la la la la la la la la la la la la".to_string())],
        });
        let song = Rc::new(song);
        let lyric_track = song
            .tracks
            .iter()
            .position(|t| Some(t.number) == song.lyrics.as_ref().map(|l| l.track_choice))
            .expect("a track carrying the lyrics");

        let syllables = lyric_syllables(&song, lyric_track);
        let sung: Vec<&String> = syllables
            .iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect();
        assert!(!sung.is_empty(), "expected the lyrics to be laid out");

        // every syllable sits on a beat that actually sounds
        for (measure_index, row) in syllables.iter().enumerate() {
            let beats = &song.tracks[lyric_track].measures[measure_index].voices[0].beats;
            for (beat_index, syllable) in row.iter().enumerate() {
                if !syllable.is_empty() {
                    assert!(
                        !beats[beat_index].notes.is_empty(),
                        "syllable {syllable:?} landed on a silent beat"
                    );
                }
            }
        }

        // and a track without lyrics gets none, in a song of several tracks
        let mut song = parse_gp_file("test-files/bass-tuning.gp5").unwrap();
        song.lyrics = Some(Lyrics {
            track_choice: 1,
            lines: vec![(1, "la la la".to_string())],
        });
        let other = 1;
        assert!(
            lyric_syllables(&song, other)
                .iter()
                .flatten()
                .all(String::is_empty)
        );
    }

    #[test]
    fn lyrics_attached_to_no_track_stay_silent() {
        // a lyric track of zero names no track, as in TuxGuitar where only a
        // matching one-based track number receives the lyrics
        let song = Rc::new(parse_gp_file("test-files/beat-text-lyrics.gp5").unwrap());
        let lyrics = song.lyrics.as_ref().expect("the file carries lyrics");
        assert_eq!(lyrics.track_choice, 0);
        for track_id in 0..song.tracks.len() {
            assert!(
                lyric_syllables(&song, track_id)
                    .iter()
                    .flatten()
                    .all(String::is_empty),
                "track {track_id} should not receive the lyrics"
            );
        }
    }

    #[test]
    fn measures_of_a_line_share_their_height() {
        let tab = load_tablature(800.0, 600.0);
        // staves must align within a line, so every measure on it keeps the
        // same annotation rows
        let mut per_line: BTreeMap<u32, f32> = BTreeMap::new();
        for cm in &tab.canvas_measures {
            let line = tab.line_tracker.get_line(cm.measure_id);
            let height = *per_line.entry(line).or_insert(cm.vertical_measure_height);
            assert!(
                (height - cm.vertical_measure_height).abs() < f32::EPSILON,
                "measure {} breaks the height of line {line}",
                cm.measure_id
            );
        }
        // and the rows are allocated per line, not globally
        let distinct: Vec<f32> = {
            let mut heights: Vec<f32> = per_line.values().copied().collect();
            heights.sort_by(f32::total_cmp);
            heights.dedup();
            heights
        };
        assert!(
            distinct.len() > 1,
            "expected lines of different heights, got {distinct:?}"
        );
    }

    #[test]
    fn page_holds_still_while_being_read() {
        let mut tab = load_tablature(800.0, 600.0);
        let visible = tab.visible_lines(1);
        assert!(visible > 2, "the view should hold several lines");
        // the page only moves once playback reaches its last line
        for cm_id in 0..tab.measure_count() {
            if tab.line_tracker.get_line(cm_id) >= visible {
                break;
            }
            assert_eq!(
                tab.page_scroll_offset(cm_id),
                None,
                "measure {cm_id} should not move the page"
            );
        }
    }

    #[test]
    fn page_turns_a_line_early() {
        let mut tab = load_tablature(800.0, 600.0);
        let visible = tab.visible_lines(1);
        // reaching the last line of the page turns it, putting that line on
        // top so the whole next page is readable while it plays
        let last_line_measure = (0..tab.measure_count())
            .find(|&id| tab.line_tracker.get_line(id) == visible)
            .expect("a measure on the last visible line");
        let line = tab.line_tracker.get_line(last_line_measure);
        let offset = tab
            .page_scroll_offset(last_line_measure)
            .expect("the page should turn");
        assert!((offset - tab.offset_for_line_top(line)).abs() < f32::EPSILON);

        // the played line is now the top one, and the view settles again
        assert_eq!(tab.page_top_line, line);
        assert_eq!(tab.page_scroll_offset(last_line_measure), None);
    }

    #[test]
    fn page_turns_back_when_seeking_backwards() {
        let mut tab = load_tablature(800.0, 600.0);
        let last = tab.measure_count() - 1;
        tab.page_scroll_offset(last).expect("the page should turn");
        // seeking back to the start turns the page back to the top
        assert_eq!(tab.page_scroll_offset(0), Some(0.0));
    }

    #[test]
    fn a_line_taller_than_the_view_still_turns_the_page() {
        let mut tab = load_tablature(800.0, 1.0);
        // one line at a time, but never zero: playback must keep advancing
        assert_eq!(tab.visible_lines(1), 1);
        let next_line_measure = (0..tab.measure_count())
            .find(|&id| tab.line_tracker.get_line(id) == 2)
            .expect("a second line");
        assert!(tab.page_scroll_offset(next_line_measure).is_some());
    }

    #[test]
    fn line_tracker_single_line() {
        let widths = vec![100.0, 100.0, 100.0];
        let tracker = LineTracker::make_from_widths(&widths, 500.0);
        assert_eq!(tracker.get_line(0), 1);
        assert_eq!(tracker.get_line(1), 1);
        assert_eq!(tracker.get_line(2), 1);
    }

    #[test]
    fn line_tracker_wraps_to_multiple_lines() {
        let widths = vec![100.0, 100.0, 100.0, 100.0];
        let tracker = LineTracker::make_from_widths(&widths, 250.0);
        // first two fit (200 < 250), third overflows (300 >= 250)
        assert_eq!(tracker.get_line(0), 1);
        assert_eq!(tracker.get_line(1), 1);
        assert_eq!(tracker.get_line(2), 2);
        assert_eq!(tracker.get_line(3), 2);
    }

    #[test]
    fn line_tracker_exact_fit_stays() {
        // measures that exactly fill the width should stay on the same line
        let widths = vec![100.0, 100.0, 100.0];
        let tracker = LineTracker::make_from_widths(&widths, 200.0);
        assert_eq!(tracker.get_line(0), 1);
        assert_eq!(tracker.get_line(1), 1); // 200 == 200, fits exactly
        assert_eq!(tracker.get_line(2), 2); // 300 > 200, wraps
    }

    #[test]
    fn line_tracker_single_wide_measure() {
        // a measure wider than the container gets its own line
        let widths = vec![50.0, 300.0, 50.0];
        let tracker = LineTracker::make_from_widths(&widths, 200.0);
        assert_eq!(tracker.get_line(0), 1);
        assert_eq!(tracker.get_line(1), 2);
        assert_eq!(tracker.get_line(2), 3);
    }

    #[test]
    fn line_tracker_varying_widths() {
        let widths = vec![80.0, 60.0, 90.0, 70.0, 50.0];
        let tracker = LineTracker::make_from_widths(&widths, 200.0);
        // line 1: 80 + 60 = 140 < 200
        // line 1: 140 + 90 = 230 >= 200 → wrap
        // line 2: 90 + 70 = 160 < 200
        // line 2: 160 + 50 = 210 >= 200 → wrap
        assert_eq!(tracker.get_line(0), 1);
        assert_eq!(tracker.get_line(1), 1);
        assert_eq!(tracker.get_line(2), 2);
        assert_eq!(tracker.get_line(3), 2);
        assert_eq!(tracker.get_line(4), 3);
    }

    #[test]
    fn line_tracker_empty() {
        let widths: Vec<f32> = vec![];
        let tracker = LineTracker::make_from_widths(&widths, 500.0);
        assert_eq!(tracker.measure_to_line.len(), 0);
    }

    #[test]
    fn first_on_line_detection() {
        let widths = vec![100.0, 100.0, 100.0, 100.0];
        let tracker = LineTracker::make_from_widths(&widths, 250.0);
        // lines: [1, 1, 2, 2]
        let mut prev_line = 0_u32;
        let mut first_on_line = Vec::new();
        for i in 0..widths.len() {
            let line = tracker.get_line(i);
            first_on_line.push(line != prev_line);
            prev_line = line;
        }
        assert_eq!(first_on_line, vec![true, false, true, false]);
    }

    #[test]
    fn one_measure_is_focused_at_a_time() {
        let mut tab = load_tablature(800.0, 600.0);
        tab.focus_on_measure(3);
        tab.focus_on_measure_beat(5, 1);
        // past the end: ignored
        tab.focus_on_measure(tab.measure_count());
        let focused: Vec<usize> = tab
            .canvas_measures
            .iter()
            .filter(|measure| measure.is_focused)
            .map(|measure| measure.measure_id)
            .collect();
        assert_eq!(focused, [5]);
        assert_eq!(tab.focused_measure(), 5);
    }

    #[test]
    fn switching_tracks_keeps_a_clicked_position_without_audio() {
        // the highlight is placed by a click, no player behind it
        let mut song = parse_gp_file("test-files/effects.gp5").unwrap();
        let mut other = parse_gp_file("test-files/effects.gp5")
            .unwrap()
            .tracks
            .remove(0);
        let measure = song.tracks[0]
            .measures
            .iter()
            .position(|measure| measure.voices[0].beats.len() >= 4)
            .unwrap();
        // the other track holds the second beat longer: the third is gone
        other.measures[measure].voices[0].beats.remove(2);
        let other_id = song.tracks.len();
        song.tracks.push(other);
        let order = compute_playback_order(&song.measure_headers);
        let mut tab = Tablature::new(Rc::new(song), 0, Id::unique(), &order);
        tab.update_container_size(800.0, 600.0);

        tab.focus_on_measure_beat(measure, 2);
        tab.update_track(other_id);
        // the second beat of the other track sounds at the third's start
        assert_eq!(tab.focused_measure(), measure);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 1);
        // and coming back finds the beat clicked
        tab.update_track(0);
        assert_eq!(tab.canvas_measures[measure].focused_beat, 2);
    }

    #[test]
    fn stopping_puts_the_position_back_at_the_start() {
        let mut tab = load_tablature(800.0, 600.0);
        tab.focus_on_measure_beat(5, 1);
        // a stop resets the cursor tick to the first one
        tab.focus_on_tick(FIRST_TICK);
        assert_eq!(tab.focused_measure(), 0);
        assert_eq!(tab.canvas_measures[0].focused_beat, 0);
    }
}
