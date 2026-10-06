//! Guitar Pro 6 (`.gpx`) and 7 (`.gp`): a container holding the score as
//! XML (GPIF), read into a document, then built into a song.

mod archive;
mod bit_reader;
// Faithful in-memory model of the GPIF schema; not every field is consumed by
// the player (e.g. clef, automation flags), so some remain read-only.
#[allow(dead_code)]
pub mod document;
pub mod document_reader;
pub mod file_system;
pub mod song_builder;
