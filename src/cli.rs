//! Command line arguments, all optional: the application opens without any.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Optional path to a sound font file.
    #[arg(long)]
    pub sound_font_file: Option<PathBuf>,
    /// Optional path to tab file to by-pass the file picker.
    #[arg(long)]
    pub tab_file_path: Option<PathBuf>,
    /// Disable antialiasing.
    #[arg(long, default_value_t = false)]
    pub no_antialiasing: bool,
    /// Force the color theme.
    #[arg(long, value_enum)]
    pub theme: Option<ThemeChoice>,
}

/// Color theme requested on the command line.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Light,
    Dark,
}
