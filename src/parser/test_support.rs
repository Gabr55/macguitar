//! Helpers shared by the tests of every module.

use crate::error::AppError;
use crate::parser::model::Song;
use crate::parser::parse_gp_data;

/// Parse the Guitar Pro file at `file_path`.
pub fn parse_gp_file(file_path: &str) -> Result<Song, AppError> {
    let file_data = std::fs::read(file_path)?;
    parse_gp_data(&file_data)
}
