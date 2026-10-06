//! Tests of the MIDI events built from the files in `test-files/`.

use super::MidiBuilder;
use super::effects::{apply_triplet_feel, compute_stroke_offsets};
use crate::audio::midi_event::{MidiEvent, MidiEventType};
use crate::audio::playback_order::compute_playback_order;
use crate::parser::model::{
    Beat, BeatStrokeDirection, DURATION_EIGHTH, DURATION_SIXTEENTH, Note, NoteEffect, NoteType,
    TripletFeel,
};
use crate::parser::test_support::parse_gp_file;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[test]
fn test_midi_events_for_all_files() {
    let test_dir = Path::new("test-files");
    let gold_dir = Path::new("test-files/gold-generated-midi");
    std::fs::create_dir_all(gold_dir).unwrap();
    for entry in std::fs::read_dir(test_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let extension = path.extension().unwrap();
        if extension != "gp5"
            && extension != "gp4"
            && extension != "gp3"
            && extension != "gpx"
            && extension != "gp"
        {
            continue;
        }
        let file_name = path.file_name().unwrap().to_str().unwrap();
        eprintln!("Parsing file: {file_name}");
        let file_path = path.to_str().unwrap();
        let song = parse_gp_file(file_path)
            .unwrap_or_else(|err| panic!("Failed to parse file: {file_name}\n{err}"));
        let song = Rc::new(song);
        let builder = MidiBuilder::new();
        let events = builder.build_for_song(&song);
        assert!(!events.is_empty(), "No events found for {file_name}");

        // assert sorted by tick
        assert!(events.windows(2).all(|w| w[0].tick <= w[1].tick));
        assert_eq!(events[0].tick, 1);

        // check against golden file
        let gold_file_path = gold_dir.join(format!("{file_name}.txt"));
        if !gold_file_path.exists() {
            // create gold file
            let mut file = std::fs::File::create(&gold_file_path).unwrap();
            for event in &events {
                writeln!(file, "{}", print_event(event)).unwrap();
            }
        }

        // verify against gold file
        validate_gold_rendered_result(&events, gold_file_path);
    }
}

fn print_event(event: &MidiEvent) -> String {
    format!("{:?} {:?} {:?}", event.tick, event.event, event.track)
}

fn validate_gold_rendered_result(events: &[MidiEvent], gold_path: PathBuf) {
    let gold = std::fs::read_to_string(&gold_path).expect("gold file not found!");
    let mut actual_lines = events.iter().map(print_event);
    for (line_number, expected) in gold.lines().enumerate() {
        let actual = actual_lines.next().unwrap();
        assert_eq!(
            expected.trim_end(),
            actual.trim_end(),
            "line {} failed for {gold_path:?}",
            line_number + 1
        );
    }
}

#[test]
fn triplet_feel_none_no_change() {
    let beat = Beat {
        start: 960,
        ..Beat::default()
    };
    let adj = apply_triplet_feel(&beat, None, None, TripletFeel::None);
    assert_eq!(adj.start, 960);
    assert_eq!(adj.duration, beat.duration.time());
}

#[test]
fn triplet_feel_eighth_first_beat() {
    // first eighth note on quarter boundary → extended to 2/3 triplet * 2
    let mut beat = Beat {
        start: 960,
        ..Beat::default()
    };
    beat.duration.value = u16::from(DURATION_EIGHTH);
    let adj = apply_triplet_feel(&beat, None, None, TripletFeel::Eighth);
    // triplet_duration = 480 * 2 / 3 = 320, long note = 640
    assert_eq!(adj.start, 960);
    assert_eq!(adj.duration, 640);
}

#[test]
fn triplet_feel_eighth_second_beat() {
    // second eighth note on half-quarter boundary → shortened to 1/3 triplet
    let mut beat = Beat {
        start: 960 + 480, // half-quarter boundary
        ..Beat::default()
    };
    beat.duration.value = u16::from(DURATION_EIGHTH);
    let adj = apply_triplet_feel(&beat, None, None, TripletFeel::Eighth);
    // triplet_duration = 320, short note, start shifts to 960 + 640 = 1600
    assert_eq!(adj.start, 1600);
    assert_eq!(adj.duration, 320);
}

#[test]
fn triplet_feel_preserves_total_time() {
    // first + second beat durations should sum to the original pair
    let mut first = Beat {
        start: 960,
        ..Beat::default()
    };
    first.duration.value = u16::from(DURATION_EIGHTH);
    let mut second = Beat {
        start: 960 + 480,
        ..Beat::default()
    };
    second.duration.value = u16::from(DURATION_EIGHTH);
    let adj1 = apply_triplet_feel(&first, None, Some(&second), TripletFeel::Eighth);
    let adj2 = apply_triplet_feel(&second, Some(&first), None, TripletFeel::Eighth);
    // total should be 960 (one quarter note)
    assert_eq!(adj1.duration + adj2.duration, 960);
    // second starts where first ends
    assert_eq!(adj2.start, adj1.start + adj1.duration);
}

#[test]
fn triplet_feel_wrong_duration_no_change() {
    // quarter note should not be affected by eighth triplet feel
    let beat = Beat {
        start: 960,
        ..Beat::default()
    };
    // default duration is quarter (960), not eighth
    let adj = apply_triplet_feel(&beat, None, None, TripletFeel::Eighth);
    assert_eq!(adj.start, 960);
    assert_eq!(adj.duration, 960);
}

#[test]
fn triplet_feel_sixteenth_pair() {
    // sixteenth pair on eighth-note boundary
    // target_duration = 240, boundary = 480
    // triplet_duration = 240 * 2 / 3 = 160
    let mut first = Beat {
        start: 960,
        ..Beat::default()
    };
    first.duration.value = u16::from(DURATION_SIXTEENTH);
    let mut second = Beat {
        start: 960 + 240,
        ..Beat::default()
    };
    second.duration.value = u16::from(DURATION_SIXTEENTH);
    let adj1 = apply_triplet_feel(&first, None, Some(&second), TripletFeel::Sixteenth);
    let adj2 = apply_triplet_feel(&second, Some(&first), None, TripletFeel::Sixteenth);
    assert_eq!(adj1.start, 960);
    assert_eq!(adj1.duration, 320); // long: 160 * 2
    assert_eq!(adj2.start, 1280); // 960 + 320
    assert_eq!(adj2.duration, 160); // short: 160
    assert_eq!(adj1.duration + adj2.duration, 480); // total = one eighth note
}

fn make_note(string: i8) -> Note {
    let mut note = Note::new(NoteEffect::default());
    note.string = string;
    note.kind = NoteType::Normal;
    note
}

#[test]
fn stroke_offsets_no_stroke() {
    let beat = Beat::default();
    let offsets = compute_stroke_offsets(&beat, 0, 6);
    assert_eq!(offsets, vec![0, 0, 0, 0, 0, 0]);
}

#[test]
fn stroke_offsets_down_stroke() {
    // down stroke: thickest string (6, index 5) plays first
    let mut beat = Beat::default();
    beat.effect.stroke.direction = BeatStrokeDirection::Down;
    beat.notes = vec![make_note(1), make_note(3), make_note(5)];
    let increment = 10;
    let offsets = compute_stroke_offsets(&beat, increment, 6);
    // string 5 (index 4) plays first (offset 0), string 3 (index 2) second, string 1 (index 0) third
    assert_eq!(offsets[4], 0); // string 5: first
    assert_eq!(offsets[2], 10); // string 3: second
    assert_eq!(offsets[0], 20); // string 1: third
    // strings without notes have 0 offset
    assert_eq!(offsets[1], 0);
    assert_eq!(offsets[3], 0);
    assert_eq!(offsets[5], 0);
}

#[test]
fn stroke_offsets_up_stroke() {
    // up stroke: thinnest string (1, index 0) plays first
    let mut beat = Beat::default();
    beat.effect.stroke.direction = BeatStrokeDirection::Up;
    beat.notes = vec![make_note(1), make_note(3), make_note(5)];
    let increment = 10;
    let offsets = compute_stroke_offsets(&beat, increment, 6);
    assert_eq!(offsets[0], 0); // string 1: first
    assert_eq!(offsets[2], 10); // string 3: second
    assert_eq!(offsets[4], 20); // string 5: third
}
#[test]
fn playback_follows_the_repeats() {
    // the same files and expected bar sequences as alphaTab's own playback
    // tests (MidiPlaybackController.test.ts)
    let cases: [(&str, &[usize]); 3] = [
        ("playback-repeat-close.gp5", &[0, 1, 0, 1, 2]),
        (
            "playback-repeat-close-multi.gp5",
            &[0, 1, 0, 1, 0, 1, 0, 1, 2],
        ),
        (
            "playback-repeat-close-without-start-at-beginning.gp5",
            &[0, 1, 0, 1],
        ),
    ];
    for (file, expected) in cases {
        let song = parse_gp_file(&format!("test-files/{file}")).unwrap();
        let order: Vec<usize> = compute_playback_order(&song.measure_headers)
            .iter()
            .map(|(measure, _)| *measure)
            .collect();
        assert_eq!(order, expected, "{file}");
    }
}

#[test]
fn percussion_plays_on_its_own_channel() {
    let song = Rc::new(parse_gp_file("test-files/percussion-all.gp5").unwrap());
    let drums = song
        .midi_channels
        .iter()
        .find(|channel| channel.is_percussion())
        .expect("a drum channel");
    let events = MidiBuilder::new().build_for_song(&song);
    assert!(events.iter().any(|event| matches!(
        event.event,
        MidiEventType::NoteOn(channel, _, _) if channel == i32::from(drums.channel_id)
    )));
}
