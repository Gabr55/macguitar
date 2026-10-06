//! The window: header, tablature sheet and transport.

use iced::widget::space::Space;
use iced::widget::{column, container, mouse_area, opaque, pin, stack};
use iced::{Element, Length};

use crate::ui::app::message::Message;
use crate::ui::app::{App, TITLE_BAR_INSET};
use crate::ui::icons::Icon;
use crate::ui::tablature::Tablature;
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets::{error_dialog, icon_button, modal_layer};

mod header;
mod recent_files;
mod track_menu;
mod transport;
mod tuning_line;
mod welcome;

use crate::ui::app::message::Menu;
use recent_files::RECENT_MENU_WIDTH;

impl App {
    /// The window, in a tree of one shape whatever is shown: iced keeps the
    /// state of a widget (the tablature's scroll offset among them) by its
    /// place in the tree, so a layer or a row appearing around the sheet
    /// would scroll it back to the start. Absent parts are empty spaces.
    pub(super) fn view(&self) -> Element<'_, Message> {
        let tokens = Tokens::of(&self.theme);
        let nothing = || Element::from(Space::new());

        let sheet_content = self
            .tablature
            .as_ref()
            .map_or_else(|| self.welcome(tokens), Tablature::view);
        // the inner container is measured to lay the tablature out, so the
        // tuning line sits outside it
        let tuning = self
            .tablature
            .as_ref()
            .and_then(|_| self.tuning_line())
            .unwrap_or_else(nothing);
        let sheet = container(column![
            tuning,
            container(sheet_content).id(self.tablature_id.clone())
        ])
        .padding(6)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::panel);

        // fullscreen keeps only the sheet
        let (header, transport, top) = if self.is_fullscreen {
            (nothing(), nothing(), 10.0)
        } else {
            (
                self.header(tokens),
                self.transport(tokens).unwrap_or_else(nothing),
                10.0 + TITLE_BAR_INSET,
            )
        };
        let window = container(
            column![header, sheet, transport]
                .spacing(if self.is_fullscreen { 0.0 } else { 10.0 })
                .padding(iced::Padding {
                    top,
                    ..iced::Padding::new(10.0)
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::window);

        // in a window, the window moves by its empty top strip, where the
        // title bar was; in fullscreen, a button in the corner leaves it
        let title_bar: Element<Message> = if self.is_fullscreen {
            container(
                container(icon_button(
                    Icon::ExitFullscreen,
                    "Exit fullscreen (Esc)",
                    Some(Message::SetFullscreen(false)),
                    tokens,
                ))
                .padding(3)
                .style(theme::panel),
            )
            .padding(16)
            .width(Length::Fill)
            .align_x(iced::Alignment::End)
            .into()
        } else if TITLE_BAR_INSET > 0.0 {
            mouse_area(Space::new().width(Length::Fill).height(TITLE_BAR_INSET))
                .on_press(Message::DragWindow)
                .on_double_click(Message::ToggleMaximize)
                .into()
        } else {
            nothing()
        };

        let menu = match self.menu {
            Some((menu, anchor)) if !self.is_fullscreen => {
                // the track menu opens rightwards from its button, the
                // recent files one leftwards, from the window's right side
                let (content, x) = match menu {
                    Menu::Tracks => (self.track_menu(anchor, tokens), anchor.x),
                    Menu::RecentFiles => (
                        self.recent_menu(),
                        (anchor.x + anchor.width - RECENT_MENU_WIDTH).max(8.0),
                    ),
                };
                stack![
                    // a click anywhere else closes the menu
                    opaque(
                        mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                            .on_press(Message::CloseMenu)
                    ),
                    pin(opaque(content)).x(x).y(anchor.y + anchor.height + 6.0),
                ]
                .into()
            }
            _ => nothing(),
        };

        let error = self.error_message.as_ref().map_or_else(nothing, |message| {
            modal_layer(error_dialog(message, tokens), Message::ClearError)
        });

        stack![window, title_bar, menu, error].into()
    }
}
