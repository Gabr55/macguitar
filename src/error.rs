//! The error type of the application.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The window or its renderer failed.
    #[error("interface error: {0}")]
    Ui(#[from] iced::Error),
    #[error("configuration error: {0}")]
    Config(String),
    /// A Guitar Pro file could not be read.
    #[error("parsing error: {0}")]
    Parsing(String),
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
}
