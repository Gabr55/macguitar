//! The window: menus, layout, fullscreen, theme and errors.

use iced::theme::Mode;
use iced::widget::{Id, selector};
use iced::{Task, window};

use crate::ui::app::App;
use crate::ui::app::message::{Menu, Message};
use crate::ui::theme;

impl App {
    /// Open `menu` under its button, wherever the header put it.
    pub(super) fn open_menu(menu: Menu) -> Task<Message> {
        selector::find(Id::new(menu.anchor_id())).then(move |target| {
            target
                .and_then(|target| target.visible_bounds())
                .map_or_else(Task::none, |bounds| {
                    Task::done(Message::MenuAnchored(menu, bounds))
                })
        })
    }

    /// Measure the tablature's container, to lay the measures out in it,
    /// and check whether the window went in or out of fullscreen.
    pub(super) fn measure_tablature(&mut self) -> Task<Message> {
        // an open menu would hang off a button that moved
        self.menu = None;
        let measure = selector::find(self.tablature_id.clone()).then(|target| {
            // the container can be clipped out of the window (resized very
            // short): nothing to lay out then
            target
                .and_then(|target| target.visible_bounds())
                .map_or_else(Task::none, |bounds| {
                    Task::done(Message::TablatureResized(bounds.size()))
                })
        });
        Task::batch([measure, Self::read_window_mode()])
    }

    pub(super) fn set_fullscreen(&mut self, fullscreen: bool) -> Task<Message> {
        self.menu = None;
        self.is_fullscreen = fullscreen;
        let mode = if fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };
        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    /// Escape closes the innermost thing open: the error, the menu, then
    /// fullscreen.
    pub(super) fn escape(&mut self) -> Task<Message> {
        if self.error_message.is_some() {
            self.error_message = None;
            Task::none()
        } else if self.menu.is_some() {
            self.menu = None;
            Task::none()
        } else if self.is_fullscreen {
            self.set_fullscreen(false)
        } else {
            Task::none()
        }
    }

    /// Read the window's mode back, which the system may have changed.
    fn read_window_mode() -> Task<Message> {
        window::latest().and_then(|id| window::mode(id).map(Message::WindowModeChanged))
    }

    pub(super) fn follow_system_theme(&mut self, mode: Mode) -> Task<Message> {
        self.theme = theme::build(theme::is_dark(self.theme_choice, mode));
        Task::none()
    }

    pub(super) fn report_error(&mut self, error: String) -> Task<Message> {
        log::warn!("{error}");
        self.error_message = Some(error);
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::ui::app::App;
    use crate::ui::app::message::{Menu, Message};
    use iced::Rectangle;

    #[test]
    fn escape_closes_the_innermost_first() {
        let mut app = App::new(None, Config::default(), None);
        app.error_message = Some("oops".to_string());
        app.menu = Some((Menu::Tracks, Rectangle::default()));
        app.is_fullscreen = true;

        let _ = app.update(Message::Escape);
        assert!(app.error_message.is_none());
        assert!(app.menu.is_some() && app.is_fullscreen);

        let _ = app.update(Message::Escape);
        assert!(app.menu.is_none());
        assert!(app.is_fullscreen);

        let _ = app.update(Message::Escape);
        assert!(!app.is_fullscreen);
    }

    #[test]
    fn the_window_mode_is_followed() {
        // the green button or a gesture leaves fullscreen behind our back
        let mut app = App::new(None, Config::default(), None);
        let _ = app.update(Message::ToggleFullscreen);
        assert!(app.is_fullscreen);
        let _ = app.update(Message::WindowModeChanged(iced::window::Mode::Windowed));
        assert!(!app.is_fullscreen);
    }
}
