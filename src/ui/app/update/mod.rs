//! How the application reacts to each message: this module dispatches, the
//! submodules handle one concern each.

mod file;
mod playback;
mod practice;
mod tracks;
mod window;

use iced::Task;

use crate::ui::app::App;
use crate::ui::app::message::Message;

impl App {
    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // files
            Message::OpenFileDialog => self.open_file_dialog(),
            Message::TablaturePicked(path) => {
                self.is_loading = false;
                path.map_or_else(Task::none, |path| self.open_file(path))
            }
            Message::OpenFile(path) => self.open_file(path),
            Message::FileLoaded(result) => self.file_loaded(result),

            // playback
            Message::PlayPause => self.play_pause(),
            Message::StopPlayer => self.stop(),
            Message::FocusTick(tick) => self.follow_playback(tick),
            Message::FocusMeasure(measure, beat) => self.seek_to_beat(measure, beat),
            Message::NextMeasure => self.step_measure(1),
            Message::PreviousMeasure => self.step_measure(-1),
            Message::MasterVolumeChanged(volume) => {
                self.with_player(|player| player.set_master_volume(volume));
                Task::none()
            }

            // tracks and sound
            Message::TrackSelected(track) => self.select_track(track),
            Message::ToggleSolo => self.toggle_solo(self.track_selection.index),
            Message::ToggleMute => self.toggle_mute(self.track_selection.index),
            Message::SoloTrack(track) => self.toggle_solo(track),
            Message::MuteTrack(track) => self.toggle_mute(track),
            Message::TrackVolumeChanged(track, volume) => {
                self.with_player(|player| player.set_track_volume(track, volume));
                Task::none()
            }
            Message::SoundSelected(choice) => self.select_sound(choice),
            Message::SoundFontPicked(path) => {
                path.map_or_else(Task::none, |path| self.change_sound_font(Some(path)))
            }

            // practice
            Message::TempoSelected(tempo) => self.set_tempo(tempo),
            Message::IncreaseTempo => {
                let faster = self.playback.tempo.faster();
                faster.map_or_else(Task::none, |tempo| self.set_tempo(tempo))
            }
            Message::DecreaseTempo => {
                let slower = self.playback.tempo.slower();
                slower.map_or_else(Task::none, |tempo| self.set_tempo(tempo))
            }
            Message::ToggleMetronome => self.toggle_metronome(),
            Message::ToggleCountIn => self.toggle_count_in(),
            Message::Transpose(step) => self.retune(step),
            Message::LoopFrom(measure) => self.start_loop(measure),
            Message::LoopOver(measure) => self.extend_loop(measure),
            Message::LoopDrawn => self.finish_loop(),
            Message::ToggleLoop => self.toggle_loop(),

            // window
            Message::OpenMenu(menu) => Self::open_menu(menu),
            Message::MenuAnchored(menu, bounds) => {
                self.menu = Some((menu, bounds));
                Task::none()
            }
            Message::CloseMenu => {
                self.menu = None;
                Task::none()
            }
            Message::WindowResized => self.measure_tablature(),
            Message::TablatureResized(size) => {
                if let Some(tablature) = &mut self.tablature {
                    tablature.update_container_size(size.width, size.height);
                }
                Task::none()
            }
            Message::ToggleFullscreen => self.set_fullscreen(!self.is_fullscreen),
            Message::SetFullscreen(fullscreen) => self.set_fullscreen(fullscreen),
            Message::WindowModeChanged(mode) => {
                self.is_fullscreen = mode == iced::window::Mode::Fullscreen;
                Task::none()
            }
            Message::Escape => self.escape(),
            Message::DragWindow => iced::window::latest().and_then(iced::window::drag),
            Message::ToggleMaximize => {
                iced::window::latest().and_then(iced::window::toggle_maximize)
            }
            Message::SystemThemeChanged(mode) => self.follow_system_theme(mode),
            Message::ReportError(error) => self.report_error(error),
            Message::ClearError => {
                self.error_message = None;
                Task::none()
            }
        }
    }
}
