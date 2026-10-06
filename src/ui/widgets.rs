//! Widgets shared across the interface: buttons, tooltips, dialogs, fonts.

use crate::ui::app::Message;
use crate::ui::icons::{Icon, icon};
use crate::ui::theme::{self, Tokens};
use iced::font::Weight;
use iced::widget::{
    Space, button, center, column, container, mouse_area, opaque, row, scrollable, text, tooltip,
};
use iced::{Alignment, Element, Font, Length, Theme};

/// The interface font, bundled so the app looks the same everywhere.
pub const UI_FONT: Font = Font::with_name("Inter");
pub const UI_FONT_BOLD: Font = Font {
    weight: Weight::Bold,
    ..UI_FONT
};

/// Muted secondary text.
pub fn muted<'a>(content: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(content).style(|theme: &Theme| text::Style {
        color: Some(Tokens::of(theme).muted),
    })
}

/// A short vertical line between groups of controls.
pub fn divider<'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(1)
        .height(22)
        .style(theme::divider)
        .into()
}

pub fn with_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(12)).padding([4, 8]),
        position,
    )
    .gap(8)
    .style(theme::tooltip)
    .into()
}

/// A square icon button; disabled when `on_press` is `None`.
pub fn icon_button<'a>(
    glyph: Icon,
    label: &'a str,
    on_press: Option<Message>,
    tokens: Tokens,
) -> Element<'a, Message> {
    let color = if on_press.is_some() {
        tokens.text
    } else {
        tokens.muted
    };
    let action = button(center(icon(glyph, 18.0, color)))
        .width(34)
        .height(34)
        .padding(0)
        .style(theme::ghost_button)
        .on_press_maybe(on_press);
    with_tooltip(action, label, tooltip::Position::Top)
}

/// An icon button that stays tinted while its setting is on.
pub fn toggle_button<'a>(
    glyph: Icon,
    label: &'a str,
    on_press: Message,
    active: bool,
    tokens: Tokens,
) -> Element<'a, Message> {
    let color = if active { tokens.accent } else { tokens.text };
    let action = button(center(icon(glyph, 18.0, color)))
        .width(34)
        .height(34)
        .padding(0)
        .style(theme::toggle_button(active))
        .on_press(on_press);
    with_tooltip(action, label, tooltip::Position::Top)
}

/// The round accent play/pause button.
pub fn play_button<'a>(playing: bool, tokens: Tokens) -> Element<'a, Message> {
    let (glyph, label) = if playing {
        (Icon::Pause, "Pause (Space)")
    } else {
        (Icon::Play, "Play (Space)")
    };
    let action = button(center(icon(glyph, 20.0, tokens.on_accent)))
        .width(42)
        .height(42)
        .padding(0)
        .style(theme::play_button)
        .on_press(Message::PlayPause);
    with_tooltip(action, label, tooltip::Position::Top)
}

/// The accent "open a file" button, with its label.
pub fn open_button<'a>(label: &'a str, enabled: bool, tokens: Tokens) -> Element<'a, Message> {
    open_button_sized(label, enabled, tokens, 14.0, [8, 14])
}

/// The accent "open a file" button as its icon alone, for a phone.
pub fn open_icon_button<'a>(enabled: bool, tokens: Tokens) -> Element<'a, Message> {
    button(center(icon(Icon::Open, 18.0, tokens.on_accent)))
        .width(36)
        .height(34)
        .padding(0)
        .style(theme::primary_button)
        .on_press_maybe(enabled.then_some(Message::OpenFileDialog))
        .into()
}

fn open_button_sized<'a>(
    label: &'a str,
    enabled: bool,
    tokens: Tokens,
    size: f32,
    padding: [u16; 2],
) -> Element<'a, Message> {
    let content = row![
        icon(Icon::Open, size + 2.0, tokens.on_accent),
        text(label).size(size).font(UI_FONT_BOLD)
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    button(content)
        .padding(padding)
        .style(theme::primary_button)
        .on_press_maybe(enabled.then_some(Message::OpenFileDialog))
        .into()
}

/// A dialog explaining what went wrong.
pub fn error_dialog<'a>(message: &'a str, tokens: Tokens) -> Element<'a, Message> {
    let title = row![
        icon(Icon::Alert, 22.0, tokens.danger),
        text("Something went wrong").size(17).font(UI_FONT_BOLD)
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let dismiss = button(text("Dismiss").size(14).font(UI_FONT_BOLD))
        .padding([8, 16])
        .style(theme::secondary_button)
        .on_press(Message::ClearError);

    container(
        column![
            title,
            // a long report scrolls, the button stays in reach
            container(scrollable(muted(message).size(14)).height(Length::Shrink)).max_height(360),
            row![Space::new().width(Length::Fill), dismiss]
        ]
        .spacing(16),
    )
    .padding(24)
    .max_width(460)
    .style(theme::dialog)
    .into()
}

/// A dialog over the window, which it dims; a click beside the dialog
/// sends `on_blur`. Meant as the top layer of a stack.
pub fn modal_layer<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    opaque(mouse_area(center(opaque(content)).style(theme::backdrop)).on_press(on_blur))
}
