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
    error_message: Option<String>,
}

impl App {
    fn new(
        sound_font_file: Option<PathBuf>,
        config: Config,
        theme_choice: Option<ThemeChoice>,
    ) -> Self {
        Self {
            song_info: None,
            all_tracks: Vec::new(),
            track_selection: TrackSelection::default(),
            tablature: None,
            tablature_id: Id::new("tablature-outer-container"),
            audio_player: None,
            loop_range: None,
            loop_anchor: None,
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
        let app = Self::new(sound_font, args.local_config.clone(), args.theme);

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
        size: Size::new(1240.0, 820.0),
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
