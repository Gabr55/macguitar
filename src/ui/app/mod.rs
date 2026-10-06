//! The application: its state, how it starts, and its window.

mod format;
mod message;
mod playback;
mod song_info;
mod subscription;
mod update;
mod view;

pub use message::Message;
use message::{Menu, TrackSelection};
use playback::PlaybackSettings;
use song_info::SongDisplayInfo;

use iced::theme::Mode;
use iced::widget::Id;
use iced::{Rectangle, Size, Task, Theme, system, window};

use crate::ApplicationArgs;
use crate::audio::midi_player::AudioPlayer;
use crate::cli::ThemeChoice;
use crate::config::Config;
use crate::ui::tablature::Tablature;
use crate::ui::theme;
use crate::ui::widgets::UI_FONT;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::sync::Notify;

const INTER_REGULAR: &[u8] = include_bytes!("../../../resources/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../../resources/fonts/Inter-Bold.ttf");
const NOTO_SANS: &[u8] = include_bytes!("../../../resources/fonts/NotoSans-Regular.ttf");
const WINDOW_ICON: &[u8] = include_bytes!("../../../resources/icon/icon-256.png");

/// On macOS the content runs under a transparent title bar; this strip at
/// the top leaves room for the window buttons.
const TITLE_BAR_INSET: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };

pub struct App {
    // the song
    song_info: Option<SongDisplayInfo>,
    all_tracks: Vec<TrackSelection>,
    track_selection: TrackSelection,
    tablature: Option<Tablature>,
    tablature_id: Id,
    audio_player: Option<AudioPlayer>,
    /// Measures played over and over, first and last, when looping.
    loop_range: Option<(usize, usize)>,
    /// Where the loop being drawn with the right button started, and
    /// whether the pointer has left that measure since.
    loop_anchor: Option<(usize, bool)>,
    /// A finger on the tablature, until it is lifted.
    touch: Option<TouchGesture>,
    is_loading: bool,

    // playback, kept from one song to the next
    playback: PlaybackSettings,
    sound_font_file: Option<PathBuf>,
    /// Latest tick published by the audio callback, with its wake-up signal.
    current_tick: Arc<AtomicU32>,
    beat_notify: Arc<Notify>,

    // settings and what they offer
    config: Config,
    /// SoundFonts found in the configuration folder, for the sound menu.
    installed_sound_fonts: Vec<PathBuf>,
    /// Tablatures opened lately, still on disk, the last one first.
    recent_files: Vec<PathBuf>,

    // the window
    /// Theme forced on the command line, if any.
    theme_choice: Option<ThemeChoice>,
    /// The theme in use, rebuilt when the desktop switches tone.
    theme: Theme,
    is_fullscreen: bool,
    /// The open menu, with the bounds of the button it hangs under.
    menu: Option<(Menu, Rectangle)>,
    /// Size of the window, which a phone's screen makes compact.
    window_size: Size,
    /// How large the tablature is drawn, kept between songs and launches.
    zoom: f32,
    error_message: Option<String>,
}

/// A finger down on a beat: a tap, a hold or a scroll, as it turns out.
#[derive(Debug, Clone, Copy)]
struct TouchGesture {
    /// Tells this touch from the next, for the timer of the hold.
    id: u64,
    measure: usize,
    beat: usize,
    /// The finger moved away: it scrolls the sheet.
    scrolled: bool,
    /// The finger stayed down long enough: it draws a loop.
    held: bool,
}

/// The sizes the tablature is drawn at, smallest to largest.
const ZOOM_STEPS: [f32; 11] = [0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.15, 1.3, 1.5, 1.75, 2.0];

/// The step after `zoom` in `direction`, or the last one; a zoom between
/// steps goes to the nearest one that way.
fn zoom_step(zoom: f32, direction: i32) -> f32 {
    if direction > 0 {
        ZOOM_STEPS
            .iter()
            .copied()
            .find(|step| *step > zoom + 0.01)
            .unwrap_or(ZOOM_STEPS[ZOOM_STEPS.len() - 1])
    } else {
        ZOOM_STEPS
            .iter()
            .rev()
            .copied()
            .find(|step| *step < zoom - 0.01)
            .unwrap_or(ZOOM_STEPS[0])
    }
}

/// Size of the window when it opens on a desktop.
const WINDOW_WIDTH: f32 = 1240.0;
const WINDOW_HEIGHT: f32 = 820.0;

/// Below this width or height the layout is compact: a phone's screen.
const COMPACT_WIDTH: f32 = 760.0;
const COMPACT_HEIGHT: f32 = 520.0;
/// Below this width, a phone held upright, the transport takes two lines.
const NARROW_WIDTH: f32 = 600.0;

impl App {
    /// A phone's screen: smaller margins, fewer and denser controls.
    fn is_compact(&self) -> bool {
        self.window_size.width < COMPACT_WIDTH || self.window_size.height < COMPACT_HEIGHT
    }

    /// A phone held upright.
    fn is_narrow(&self) -> bool {
        self.window_size.width < NARROW_WIDTH
    }

    fn new(
        sound_font_file: Option<PathBuf>,
        config: Config,
        theme_choice: Option<ThemeChoice>,
    ) -> Self {
        let zoom = config.zoom();
        Self {
            song_info: None,
            all_tracks: Vec::new(),
            track_selection: TrackSelection::default(),
            tablature: None,
            tablature_id: Id::new("tablature-outer-container"),
            audio_player: None,
            loop_range: None,
            loop_anchor: None,
            touch: None,
            is_loading: false,
            playback: PlaybackSettings::default(),
            sound_font_file,
            current_tick: Arc::new(AtomicU32::new(0)),
            beat_notify: Arc::new(Notify::new()),
            installed_sound_fonts: Config::installed_sound_fonts(),
            recent_files: config.recent_files(),
            config,
            theme_choice,
            theme: theme::build(theme::is_dark(theme_choice, Mode::None)),
            is_fullscreen: false,
            menu: None,
            window_size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            zoom,
            error_message: None,
        }
    }

    fn boot(args: &ApplicationArgs) -> (Self, Task<Message>) {
        // a SoundFont given on the command line wins over the saved choice
        let sound_font = args.sound_font_bank.clone().or_else(|| {
            args.local_config
                .get_sound_font()
                .filter(|path| path.exists())
        });
        let mut app = Self::new(sound_font, args.local_config.clone(), args.theme);
        app.error_message = args
            .crash_report
            .as_ref()
            .map(|report| format!("MacGuitar stopped unexpectedly the last time:\n{report}"));

        let open_task = args
            .tab_file_path
            .clone()
            .map_or_else(Task::none, |path| Task::done(Message::OpenFile(path)));
        let theme_task = system::theme().map(Message::SystemThemeChanged);
        (app, Task::batch([theme_task, open_task]))
    }

    pub fn start(args: ApplicationArgs) -> iced::Result {
        let antialiasing = !args.no_antialiasing;
        iced::application(move || Self::boot(&args), Self::update, Self::view)
            .title(Self::title)
            .subscription(Self::subscription)
            .font(INTER_REGULAR)
            .font(INTER_BOLD)
            .font(NOTO_SANS)
            .default_font(UI_FONT)
            .theme(|state: &Self| state.theme.clone())
            .window(window_settings())
            .antialiasing(antialiasing)
            .run()
    }

    fn title(&self) -> String {
        match &self.song_info {
            Some(song_info) => format!("{} \u{2014} MacGuitar", song_info.file_name),
            None => String::from("MacGuitar"),
        }
    }

    /// Run `action` on the player, when a song is loaded.
    fn with_player(&self, action: impl FnOnce(&AudioPlayer)) {
        if let Some(player) = &self.audio_player {
            action(player);
        }
    }
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        min_size: Some(Size::new(820.0, 520.0)),
        position: window::Position::Centered,
        icon: window_icon(),
        #[cfg(target_os = "macos")]
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..window::Settings::default()
    }
}

/// The app icon for the title bar and task bar. macOS ignores it: the dock
/// takes the icon of the application bundle.
fn window_icon() -> Option<window::Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(WINDOW_ICON));
    let mut reader = decoder.read_info().ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut rgba).ok()?;
    rgba.truncate(info.buffer_size());
    window::icon::from_rgba(rgba, info.width, info.height).ok()
}

#[cfg(test)]
mod zoom_tests {
    use super::{ZOOM_STEPS, zoom_step};

    #[test]
    fn the_zoom_goes_by_steps_and_stops_at_the_ends() {
        assert!((zoom_step(1.0, 1) - 1.15).abs() < f32::EPSILON);
        assert!((zoom_step(1.0, -1) - 0.9).abs() < f32::EPSILON);
        let last = ZOOM_STEPS[ZOOM_STEPS.len() - 1];
        assert!((zoom_step(last, 1) - last).abs() < f32::EPSILON);
        assert!((zoom_step(ZOOM_STEPS[0], -1) - ZOOM_STEPS[0]).abs() < f32::EPSILON);
        // a zoom saved between steps goes to the nearest one that way
        assert!((zoom_step(1.07, 1) - 1.15).abs() < f32::EPSILON);
        assert!((zoom_step(1.07, -1) - 1.0).abs() < f32::EPSILON);
    }
}
