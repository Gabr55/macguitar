//! Opening tablatures and choosing the sound.

use iced::Task;
use iced::widget::Id;
use iced::widget::operation::scroll_to;
use iced::widget::scrollable::AbsoluteOffset;
use std::path::PathBuf;
use std::rc::Rc;

use crate::audio::midi_player::AudioPlayer;
use crate::audio::playback_order::compute_playback_order;
use crate::parser::model::Song;
use crate::parser::parse_gp_data;
use crate::ui::app::App;
use crate::ui::app::message::{Message, SoundChoice, TrackSelection};
use crate::ui::app::song_info::SongDisplayInfo;
use crate::ui::files::{FileError, LoadedFile, load_tablature, pick_sound_font, pick_tablature};
use crate::ui::tablature::Tablature;

impl App {
    pub(super) fn open_file_dialog(&mut self) -> Task<Message> {
        if self.is_loading {
            return Task::none();
        }
        self.is_loading = true;
        Task::perform(
            pick_tablature(self.config.get_tabs_folder()),
            Message::TablaturePicked,
        )
    }

    pub(super) fn open_file(&mut self, path: PathBuf) -> Task<Message> {
        self.menu = None;
        if self.is_loading {
            return Task::none();
        }
        self.is_loading = true;
        Task::perform(load_tablature(path), Message::FileLoaded)
    }

    pub(super) fn file_loaded(&mut self, result: Result<LoadedFile, FileError>) -> Task<Message> {
        self.is_loading = false;
        let opened = result
            .map_err(|err| format!("Could not open the file: {err}"))
            .and_then(|file| self.open_song(&file).map(|task| (file, task)));
        match opened {
            Ok((file, task)) => {
                self.remember_file(&file);
                task
            }
            Err(err) => Task::done(Message::ReportError(err)),
        }
    }

    /// Show and play the song in `file`, in place of the current one. On
    /// failure the current song stays as it was.
    fn open_song(&mut self, file: &LoadedFile) -> Result<Task<Message>, String> {
        let song = parse_gp_data(&file.contents)
            .map_err(|err| format!("Could not read {}: {err}", file.name()))?;
        let tracks = TrackSelection::all_of(&song);
        let first_track = tracks
            .first()
            .cloned()
            .ok_or_else(|| format!("{} has no tracks", file.name()))?;

        // the previous song stops before the new one takes its place
        if let Some(player) = &mut self.audio_player {
            player.stop();
        }
        self.audio_player = None;
        self.loop_range = None;
        self.loop_anchor = None;
        self.song_info = Some(SongDisplayInfo::new(&song, file.name()));
        self.all_tracks = tracks;
        self.track_selection = first_track;

        // the tablature and the player share the song
        let song = Rc::new(song);
        let playback_order = compute_playback_order(&song.measure_headers);
        let scroll_id = Id::new("tablature-scroll-elements");
        self.tablature = Some(Tablature::new(
            song.clone(),
            self.track_selection.index,
            scroll_id.clone(),
            &playback_order,
        ));
        self.load_audio_player(song, &playback_order)
            .map_err(|err| format!("Could not start the audio: {err}"))?;

        // back to the top, and lay the new tablature out
        Ok(Task::batch([
            scroll_to(scroll_id, AbsoluteOffset::<f32>::default()),
            Task::done(Message::WindowResized),
        ]))
    }

    /// Keep the file among the recent ones, and its folder for the dialog.
    fn remember_file(&mut self, file: &LoadedFile) {
        if let Err(err) = self.config.set_tabs_folder(file.folder()) {
            log::warn!("Could not save the tabs folder: {err}");
        }
        if let Err(err) = self.config.add_recent_file(file.path.clone()) {
            log::warn!("Could not save the recent files: {err}");
        }
        self.recent_files = self.config.recent_files();
    }

    /// Start a player for `song`, set as the previous one was.
    pub(super) fn load_audio_player(
        &mut self,
        song: Rc<Song>,
        playback_order: &[(usize, i64)],
    ) -> Result<(), String> {
        let volume = self
            .audio_player
            .as_ref()
            .map_or(1.0, AudioPlayer::master_volume);
        let player = AudioPlayer::new(
            song.clone(),
            song.tempo.value,
            self.playback.tempo.percentage,
            self.sound_font_file.clone(),
            self.current_tick.clone(),
            self.beat_notify.clone(),
            playback_order,
        )
        .map_err(|err| err.to_string())?;
        self.playback.apply_to(&player);
        player.set_master_volume(volume);
        player.set_loop(self.loop_range);
        self.audio_player = Some(player);
        Ok(())
    }

    pub(super) fn select_sound(&mut self, choice: SoundChoice) -> Task<Message> {
        match choice {
            SoundChoice::Browse => Task::perform(pick_sound_font(), Message::SoundFontPicked),
            SoundChoice::BuiltIn => self.change_sound_font(None),
            SoundChoice::File(path) => self.change_sound_font(Some(path)),
        }
    }

    /// Play with another SoundFont (`None` for the built-in one), kept for
    /// the next launches. A song being played restarts from where it was.
    pub(super) fn change_sound_font(&mut self, sound_font: Option<PathBuf>) -> Task<Message> {
        if self.sound_font_file == sound_font {
            return Task::none();
        }
        self.sound_font_file = sound_font.clone();
        if let Err(err) = self.config.set_sound_font(sound_font) {
            log::warn!("Could not save the sound font choice: {err}");
        }
        let Some(tablature) = &self.tablature else {
            return Task::none();
        };
        let song = tablature.song.clone();
        let measure = tablature.focused_measure();
        if let Some(player) = &mut self.audio_player {
            player.stop();
        }
        let playback_order = compute_playback_order(&song.measure_headers);
        match self.load_audio_player(song.clone(), &playback_order) {
            Ok(()) => {
                self.with_player(|player| player.focus_measure_at(measure, 0));
                Task::none()
            }
            Err(err) => {
                // fall back to the built-in sound, which always loads
                self.sound_font_file = None;
                if let Err(err) = self.config.set_sound_font(None) {
                    log::warn!("Could not save the sound font choice: {err}");
                }
                let fallback = self
                    .load_audio_player(song, &playback_order)
                    .err()
                    .map(|err| format!(" (the built-in sound failed too: {err})"))
                    .unwrap_or_default();
                Task::done(Message::ReportError(format!(
                    "Could not load the SoundFont: {err}{fallback}"
                )))
            }
        }
    }
}
