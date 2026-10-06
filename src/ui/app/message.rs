//! What the interface reports, and the choices it offers.

use iced::Size;
use iced::theme::Mode;
use std::fmt::Display;

use crate::parser::model::Song;
use crate::ui::tuning::tuning_label;

use crate::ui::files::{FileError, LoadedFile};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TempoSelection {
    pub(super) percentage: u32,
}

impl Default for TempoSelection {
    fn default() -> Self {
        Self::new(100)
    }
}

impl TempoSelection {
    pub(super) const fn new(percentage: u32) -> Self {
        Self { percentage }
    }

    /// The next preset up, if any.
    pub(super) fn faster(self) -> Option<Self> {
        let index = Self::PRESET.iter().position(|preset| *preset == self)?;
        Self::PRESET.get(index + 1).copied()
    }

    /// The next preset down, if any.
    pub(super) fn slower(self) -> Option<Self> {
        let index = Self::PRESET.iter().position(|preset| *preset == self)?;
        index.checked_sub(1).map(|previous| Self::PRESET[previous])
    }

    pub(super) const PRESET: [Self; 9] = [
        Self::new(25),
        Self::new(50),
        Self::new(60),
        Self::new(70),
        Self::new(80),
        Self::new(90),
        Self::new(100),
        Self::new(150),
        Self::new(200),
    ];
}

impl Display for TempoSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.percentage)
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct TrackSelection {
    pub(super) index: usize,
    pub(super) name: String,
    pub(super) tuning: Option<String>,
    /// Open string pitches as (string, MIDI pitch); empty for tracks that
    /// are not stringed instruments.
    pub(super) strings: Vec<(i32, i32)>,
}

impl TrackSelection {
    /// The tracks of `song`, as the track menu lists them. Drum tracks
    /// carry no tuning.
    pub(super) fn all_of(song: &Song) -> Vec<Self> {
        song.tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let is_stringed = song
                    .midi_channels
                    .iter()
                    .find(|channel| channel.channel_id == track.channel_id)
                    .is_some_and(|channel| !channel.is_percussion());
                let strings: &[(i32, i32)] = if is_stringed { &track.strings } else { &[] };
                Self {
                    index,
                    name: track.name.clone(),
                    tuning: tuning_label(strings),
                    strings: strings.to_vec(),
                }
            })
            .collect()
    }
}

impl Display for TrackSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.index + 1, self.name)?;
        if let Some(tuning) = &self.tuning {
            write!(f, " ({tuning})")?;
        }
        Ok(())
    }
}

/// The sound the song plays with, as offered in the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundChoice {
    BuiltIn,
    File(PathBuf),
    /// Pick another SoundFont file.
    Browse,
}

impl Display for SoundChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuiltIn => write!(f, "Built-in sound"),
            Self::File(path) => write!(
                f,
                "{}",
                path.file_stem().map_or_else(
                    || path.display().to_string(),
                    |s| s.to_string_lossy().into()
                )
            ),
            Self::Browse => write!(f, "Choose a SoundFont\u{2026}"),
        }
    }
}

/// The menus floating over the window, each hung under its button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Tracks,
    RecentFiles,
}

impl Menu {
    /// The id of the button the menu hangs under.
    pub(super) const fn anchor_id(self) -> &'static str {
        match self {
            Self::Tracks => "track-menu",
            Self::RecentFiles => "recent-files-menu",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // files
    OpenFileDialog,
    TablaturePicked(Option<PathBuf>),
    OpenFile(PathBuf),
    FileLoaded(Result<LoadedFile, FileError>),

    // playback
    PlayPause,
    StopPlayer,
    /// The audio reached a new beat, at this playback tick.
    FocusTick(u32),
    /// Measure and beat clicked in the tablature.
    FocusMeasure(usize, usize),
    NextMeasure,
    PreviousMeasure,
    /// Master volume slider, in `0.0..=1.0`.
    MasterVolumeChanged(f32),

    // tracks and sound
    TrackSelected(TrackSelection),
    /// Toggle solo or mute of the selected track.
    ToggleSolo,
    ToggleMute,
    /// Toggle solo or mute of a track, from the track menu.
    SoloTrack(usize),
    MuteTrack(usize),
    /// Volume of a track, in `0.0..=1.0`, from the track menu.
    TrackVolumeChanged(usize, f32),
    SoundSelected(SoundChoice),
    SoundFontPicked(Option<PathBuf>),

    // practice
    TempoSelected(TempoSelection),
    IncreaseTempo,
    DecreaseTempo,
    ToggleMetronome,
    ToggleCountIn,
    /// Retune the whole song by this many semitones from where it is.
    Transpose(i32),
    /// The right button went down on a measure: a loop starts there.
    LoopFrom(usize),
    /// The pointer entered a measure: a loop being drawn grows to it.
    LoopOver(usize),
    /// The right button was let go: the loop is drawn.
    LoopDrawn,
    ToggleLoop,

    // window
    OpenMenu(Menu),
    MenuAnchored(Menu, iced::Rectangle),
    CloseMenu,
    WindowResized,
    TablatureResized(Size),
    /// Toggle fullscreen, which shows the tablature alone.
    ToggleFullscreen,
    /// Leave or enter fullscreen.
    SetFullscreen(bool),
    /// The mode the window is in, read back after it changed: the system
    /// (the green button, a gesture) can change it too.
    WindowModeChanged(iced::window::Mode),
    /// Escape: closes what is open, the menu, then fullscreen.
    Escape,
    /// Move the window by its header (the title bar is hidden on macOS).
    DragWindow,
    ToggleMaximize,
    /// The desktop light/dark setting, at start and when it changes.
    SystemThemeChanged(Mode),
    ReportError(String),
    ClearError,
}
