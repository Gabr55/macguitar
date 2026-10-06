//! Parsing of the files in `test-files/`, in every format.

use crate::parser::model::Song;
use crate::parser::model::{GpVersion, NoteType};
use crate::parser::test_support::parse_gp_file;

fn init_logger() {
    env_logger::builder()
        .is_test(true)
        .try_init()
        .unwrap_or_default();
}

fn parse_all_files_successfully(with_extension: &str) {
    init_logger();
    let test_dir = std::path::Path::new("test-files");
    for entry in std::fs::read_dir(test_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().unwrap() != with_extension {
            continue;
        }
        let file_name = path.file_name().unwrap().to_str().unwrap();
        eprintln!("Parsing file: {file_name}");
        let file_path = path.to_str().unwrap();
        let song = parse_gp_file(file_path)
            .unwrap_or_else(|err| panic!("Failed to parse file: {file_name}\n{err}"));
        // no empty tracks
        assert!(!song.tracks.is_empty(), "File: {file_name}");
        // assert global invariant across all measures
        for (t_id, t) in song.tracks.iter().enumerate() {
            assert_eq!(
                t.measures.len(),
                song.measure_headers.len(),
                "Track:{t_id} File:{file_name}"
            );
            for (m_id, m) in t.measures.iter().enumerate() {
                assert_eq!(
                    m.track_index, t_id,
                    "Track:{t_id} Measure:{m_id} File:{file_name}"
                );
                assert_eq!(
                    m.header_index, m_id,
                    "Track:{t_id} Measure:{m_id} File:{file_name}"
                );
                // Voice count depends on the parsed format version, not the file
                // extension: many .gp3/.gp4 files actually contain newer versions.
                let voice_count = if song.version >= GpVersion::GP5 { 2 } else { 1 };
                assert_eq!(
                    m.voices.len(),
                    voice_count,
                    "Track:{t_id} Measure:{m_id} File:{file_name}"
                );
                let measure_header = &song.measure_headers[m_id];
                let measure_start = measure_header.start;
                for v in &m.voices {
                    v.beats.iter().enumerate().for_each(|(i, b)| {
                        assert!(
                            b.start >= measure_start,
                            "track:{t_id} measure:{m_id} beat:{i} file:{file_name}"
                        );
                    });
                }
            }
        }
    }
}

#[test]
fn parse_all_gp5_files_successfully() {
    parse_all_files_successfully("gp5");
}

#[test]
fn parse_all_gp4_files_successfully() {
    parse_all_files_successfully("gp4");
}

#[test]
fn parse_all_gp3_files_successfully() {
    parse_all_files_successfully("gp3");
}

#[test]
fn parse_all_gpx_files_successfully() {
    parse_all_files_successfully("gpx");
}

#[test]
fn parse_all_gp7_files_successfully() {
    parse_all_files_successfully("gp");
}

#[test]
fn gp_version_ordering() {
    // GpVersion derives PartialOrd from variant declaration order.
    // This test ensures the ordering is correct and catches accidental reordering.
    assert!(GpVersion::GP3 < GpVersion::GP4);
    assert!(GpVersion::GP4 < GpVersion::GP4_06);
    assert!(GpVersion::GP4_06 < GpVersion::GP5);
    assert!(GpVersion::GP5 < GpVersion::GP5_10);
}

#[test]
fn unknown_effect_values_fall_back() {
    use crate::parser::model::{GraceEffectTransition, Octave, TremoloPickingEffect, TrillEffect};
    assert_eq!(
        GraceEffectTransition::get_grace_effect_transition(9),
        GraceEffectTransition::None
    );
    assert_eq!(Octave::get_octave(9), Octave::None);
    assert_eq!(TrillEffect::from_trill_period(0), None);
    assert_eq!(TrillEffect::from_trill_period(4), None);
    assert_eq!(TremoloPickingEffect::from_tremolo_value(0), None);
    assert_eq!(TremoloPickingEffect::from_tremolo_value(4), None);
}

/// Notes of a song, all tracks, measures and voices.
fn notes(song: &Song) -> impl Iterator<Item = &crate::parser::model::Note> {
    song.tracks
        .iter()
        .flat_map(|track| &track.measures)
        .flat_map(|measure| &measure.voices)
        .flat_map(|voice| &voice.beats)
        .flat_map(|beat| &beat.notes)
}

#[test]
fn every_format_is_recognized() {
    for (file, version) in [
        ("notes.gp3", GpVersion::GP3),
        ("notes.gp4", GpVersion::GP4_06),
        ("notes.gp5", GpVersion::GP5_10),
        ("notes.gpx", GpVersion::GP6),
        ("notes.gp", GpVersion::GP7),
    ] {
        let song = parse_gp_file(&format!("test-files/{file}")).unwrap();
        assert_eq!(song.version, version, "{file}");
    }
}

#[test]
fn effects_read_alike_in_every_format() {
    use crate::parser::model::SlideType;
    // the same score saved by each version of Guitar Pro
    for extension in ["gp4", "gp5", "gpx", "gp"] {
        let song = parse_gp_file(&format!("test-files/slides.{extension}")).unwrap();
        let mut slides: Vec<SlideType> =
            notes(&song).filter_map(|note| note.effect.slide).collect();
        slides.sort_by_key(|slide| *slide as u8);
        slides.dedup();
        assert_eq!(slides.len(), 6, "every kind of slide in slides.{extension}");
    }
    for extension in ["gp3", "gp4", "gp5", "gpx", "gp"] {
        let count = |file: &str, has: fn(&crate::parser::model::Note) -> bool| {
            let song = parse_gp_file(&format!("test-files/{file}.{extension}")).unwrap();
            notes(&song).filter(|note| has(note)).count()
        };
        assert_eq!(
            count("hammer", |n| n.effect.hammer),
            10,
            "hammer.{extension}"
        );
        assert_eq!(
            count("bends", |n| n.effect.bend.is_some()),
            3,
            "bends.{extension}"
        );
        assert!(
            count("vibrato", |n| n.effect.vibrato) >= 2,
            "vibrato.{extension}"
        );

        let song = parse_gp_file(&format!("test-files/tuplets.{extension}")).unwrap();
        let triplets = song.tracks[0]
            .measures
            .iter()
            .flat_map(|measure| &measure.voices[0].beats)
            .filter(|beat| beat.duration.tuplet_enters > 1)
            .count();
        assert_eq!(triplets, 8, "tuplets.{extension}");
    }
}

#[test]
fn note_kinds_and_graces() {
    let dead = parse_gp_file("test-files/dead.gp5").unwrap();
    assert_eq!(
        notes(&dead)
            .filter(|note| note.kind == NoteType::Dead)
            .count(),
        4
    );
    let grace = parse_gp_file("test-files/grace.gp5").unwrap();
    assert_eq!(
        notes(&grace)
            .filter(|note| note.effect.grace.is_some())
            .count(),
        2
    );
}

#[test]
fn tracks_keep_their_strings() {
    // two guitars around a four string bass
    let song = parse_gp_file("test-files/bass-tuning.gp5").unwrap();
    let strings: Vec<usize> = song.tracks.iter().map(|t| t.strings.len()).collect();
    assert_eq!(strings, [6, 6, 4, 6]);
}

#[test]
fn time_signatures_change_by_measure() {
    let song = parse_gp_file("test-files/time-signatures.gp5").unwrap();
    let signatures: Vec<(i32, u16)> = song
        .measure_headers
        .iter()
        .map(|h| {
            (
                i32::from(h.time_signature.numerator),
                h.time_signature.denominator.value,
            )
        })
        .collect();
    let mut distinct = signatures.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), 5, "{signatures:?}");
}
