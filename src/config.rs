//! The settings kept between launches, in `~/.config/macguitar/config.json`:
//! the folder of the last tablature, the SoundFont, the recent files.

use std::{
    env::home_dir,
    fs::{File, create_dir_all},
    io::{BufReader, Write},
    path::Path,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    tabs_folder: Option<PathBuf>,
    /// SoundFont picked in the app, instead of the built-in one.
    #[serde(default)]
    sound_font: Option<PathBuf>,
    /// Tablatures opened lately, the last one first.
    #[serde(default)]
    recent_files: Vec<PathBuf>,
    /// How large the tablature is drawn, 1.0 as laid out.
    #[serde(default)]
    zoom: Option<f32>,
}

/// How many recent files are remembered.
const MAX_RECENT_FILES: usize = 10;

/// Put `path` first in `recent`, once, keeping at most `MAX_RECENT_FILES`.
fn remember(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|known| *known != path);
    recent.insert(0, path);
    recent.truncate(MAX_RECENT_FILES);
}

impl Config {
    /// Folder of the settings, in the home directory.
    const FOLDER: &'static str = ".config/macguitar";
    /// Where ruxguitar kept them, taken over on the first launch.
    const RUXGUITAR_FOLDER: &'static str = ".config/ruxguitar";

    pub fn get_tabs_folder(&self) -> Option<PathBuf> {
        self.tabs_folder.clone()
    }

    pub fn set_tabs_folder(&mut self, new_tabs_folder: Option<PathBuf>) -> Result<(), AppError> {
        if self.tabs_folder == new_tabs_folder {
            Ok(())
        } else {
            self.tabs_folder = new_tabs_folder;
            self.save_config()
        }
    }

    /// SoundFonts dropped in the configuration's `soundfonts` folder, offered
    /// in the sound menu.
    pub fn installed_sound_fonts() -> Vec<PathBuf> {
        let Ok(folder) = Self::get_base_path().map(|base| base.join("soundfonts")) else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(folder) else {
            return Vec::new();
        };
        let mut sound_fonts: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sf2"))
            })
            .collect();
        sound_fonts.sort();
        sound_fonts
    }

    /// Tablatures opened lately and still on disk, the last one first.
    pub fn recent_files(&self) -> Vec<PathBuf> {
        self.recent_files
            .iter()
            .filter(|path| path.exists())
            .cloned()
            .collect()
    }

    pub fn add_recent_file(&mut self, path: PathBuf) -> Result<(), AppError> {
        if self.recent_files.first() == Some(&path) {
            return Ok(());
        }
        remember(&mut self.recent_files, path);
        self.save_config()
    }

    pub fn get_sound_font(&self) -> Option<PathBuf> {
        self.sound_font.clone()
    }

    /// How large the tablature is drawn, 1.0 unless changed.
    pub fn zoom(&self) -> f32 {
        self.zoom
            .filter(|zoom| zoom.is_finite() && *zoom > 0.0)
            .unwrap_or(1.0)
    }

    pub fn set_zoom(&mut self, zoom: f32) -> Result<(), AppError> {
        if self.zoom == Some(zoom) {
            Ok(())
        } else {
            self.zoom = Some(zoom);
            self.save_config()
        }
    }

    pub fn set_sound_font(&mut self, sound_font: Option<PathBuf>) -> Result<(), AppError> {
        if self.sound_font == sound_font {
            Ok(())
        } else {
            self.sound_font = sound_font;
            self.save_config()
        }
    }

    fn get_base_path() -> Result<PathBuf, AppError> {
        let home = home_dir()
            .ok_or_else(|| AppError::Config("Could not find home directory".to_string()))?;
        let path = home.join(Self::FOLDER);
        Ok(path)
    }

    fn get_path() -> Result<PathBuf, AppError> {
        let base = Self::get_base_path()?;
        Ok(base.join("config.json"))
    }

    /// Creates config if it does not exist
    pub fn read_config() -> Result<Self, AppError> {
        let base_path = Self::get_base_path()?;
        Self::take_over_ruxguitar_settings(&base_path);
        if !base_path.exists() {
            create_dir_all(base_path)?;
        }
        let config_path = Self::get_path()?;
        if !config_path.exists() {
            // create empty config
            Config::default().save_config()?;
        }
        let file = File::open(&config_path)?;
        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(config) => Ok(config),
            Err(err) => {
                log::warn!(
                    "Could not read local configuration {}: {err}, resetting to default",
                    config_path.display()
                );
                let default_config = Config::default();
                default_config.save_config()?;
                Ok(default_config)
            }
        }
    }

    /// Move the settings of ruxguitar, this player's former name, to the
    /// folder of MacGuitar, once, and point their paths at the new place.
    fn take_over_ruxguitar_settings(base_path: &Path) {
        let Some(old_path) = home_dir().map(|home| home.join(Self::RUXGUITAR_FOLDER)) else {
            return;
        };
        if base_path.exists() || !old_path.exists() {
            return;
        }
        if let Err(err) = std::fs::rename(&old_path, base_path) {
            log::warn!("Could not move the settings of ruxguitar: {err}");
            return;
        }
        log::info!("Moved the settings of ruxguitar to {}", base_path.display());
        let config_path = base_path.join("config.json");
        let Ok(json) = std::fs::read_to_string(&config_path) else {
            return;
        };
        let Ok(mut config) = serde_json::from_str::<Self>(&json) else {
            return;
        };
        let moved = |path: &PathBuf| {
            path.strip_prefix(&old_path)
                .map_or_else(|_| path.clone(), |rest| base_path.join(rest))
        };
        config.sound_font = config.sound_font.as_ref().map(moved);
        if let Err(err) = config.save_config() {
            log::warn!("Could not update the moved settings: {err}");
        }
    }

    /// Assumes the config folder exists
    pub fn save_config(&self) -> Result<(), AppError> {
        let config_path = Self::get_path()?;
        let json = serde_json::to_string_pretty(self).map_err(|err| {
            AppError::Config(format!("Could not save local configuration {err:}"))
        })?;
        let mut file = File::create(config_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_put_the_last_one_first_once() {
        let mut recent = vec![PathBuf::from("a.gp5"), PathBuf::from("b.gp5")];
        remember(&mut recent, PathBuf::from("b.gp5"));
        assert_eq!(recent, [PathBuf::from("b.gp5"), PathBuf::from("a.gp5")]);
    }

    #[test]
    fn recent_files_forget_the_oldest() {
        let mut recent = Vec::new();
        for i in 0..=MAX_RECENT_FILES {
            remember(&mut recent, PathBuf::from(format!("{i}.gp5")));
        }
        assert_eq!(recent.len(), MAX_RECENT_FILES);
        assert_eq!(recent[0], PathBuf::from(format!("{MAX_RECENT_FILES}.gp5")));
        assert!(!recent.contains(&PathBuf::from("0.gp5")));
    }
}
