//! MacGuitar: a Guitar Pro tablature player, based on ruxguitar by
//! Arnaud Gourlay.
//!
//! `parser` reads Guitar Pro files into a [`parser::model::Song`], `audio`
//! turns it into MIDI events played by a SoundFont synthesizer, and `ui`
//! draws the tablature and the controls with iced.

// release builds on Windows open as a window, without a console behind it
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::cli::{CliArgs, ThemeChoice};
use crate::config::Config;
use crate::error::AppError;
use crate::ui::app::App;
use clap::Parser;
use std::path::PathBuf;

mod audio;
mod cli;
mod config;
mod error;
mod parser;
mod ui;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("macguitar=info"))
        .init();
    if let Err(err) = run(CliArgs::parse()) {
        // Display rather than Debug: the message is for the user
        log::error!("{err}");
        std::process::exit(1);
    }
}

fn run(args: CliArgs) -> Result<(), AppError> {
    if let Some(sound_font_file) = &args.sound_font_file {
        if !sound_font_file.exists() {
            return Err(AppError::Config(format!(
                "Sound font file not found {sound_font_file:?}"
            )));
        }
        log::info!("Starting with custom sound font file {sound_font_file:?}");
    }
    if let Some(tab_file_path) = &args.tab_file_path {
        if !tab_file_path.exists() {
            return Err(AppError::Config(format!(
                "Tab file not found {tab_file_path:?}"
            )));
        }
        log::info!("Starting with tab file {tab_file_path:?}");
    }

    App::start(ApplicationArgs {
        sound_font_bank: args.sound_font_file,
        tab_file_path: args.tab_file_path,
        no_antialiasing: args.no_antialiasing,
        theme: args.theme,
        local_config: Config::read_config()?,
    })?;
    Ok(())
}

/// What the application starts with.
#[derive(Debug, Clone)]
pub struct ApplicationArgs {
    sound_font_bank: Option<PathBuf>,
    tab_file_path: Option<PathBuf>,
    no_antialiasing: bool,
    theme: Option<ThemeChoice>,
    local_config: Config,
}
