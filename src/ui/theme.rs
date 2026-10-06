//! The look of the application: two palettes, light and dark, and the
//! widget styles built on them.

use iced::border::Radius;
use iced::theme::{Mode, Palette};
use iced::widget::{button, container, pick_list, progress_bar, scrollable, slider};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::cli::ThemeChoice;

/// Corner radius of the floating panels.
pub const PANEL_RADIUS: f32 = 14.0;
/// Corner radius of buttons, inputs and menus.
pub const CONTROL_RADIUS: f32 = 9.0;

/// Design tokens, picked per tone. Every style below derives from these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    /// Window background, behind the panels.
    pub background: Color,
    /// Panels: header, tablature sheet and transport.
    pub surface: Color,
    /// Inputs and hovered controls, one step above the surface.
    pub raised: Color,
    pub border: Color,
    pub text: Color,
    /// Secondary text: labels, metadata, hints.
    pub muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    /// Text drawn on the accent.
    pub on_accent: Color,
    pub danger: Color,
    pub is_dark: bool,
}

impl Tokens {
    const DARK: Self = Self {
        background: Color::from_rgb8(0x0d, 0x0f, 0x13),
        surface: Color::from_rgb8(0x16, 0x19, 0x20),
        raised: Color::from_rgb8(0x20, 0x24, 0x2d),
        border: Color::from_rgb8(0x2a, 0x2f, 0x3a),
        text: Color::from_rgb8(0xe8, 0xea, 0xef),
        muted: Color::from_rgb8(0x8a, 0x92, 0xa3),
        accent: Color::from_rgb8(0xff, 0x6a, 0x3d),
        accent_hover: Color::from_rgb8(0xff, 0x84, 0x59),
        on_accent: Color::WHITE,
        danger: Color::from_rgb8(0xf0, 0x52, 0x52),
        is_dark: true,
    };

    const LIGHT: Self = Self {
        background: Color::from_rgb8(0xee, 0xf0, 0xf4),
        surface: Color::from_rgb8(0xff, 0xff, 0xff),
        raised: Color::from_rgb8(0xf3, 0xf4, 0xf7),
        border: Color::from_rgb8(0xe0, 0xe3, 0xea),
        text: Color::from_rgb8(0x16, 0x1a, 0x22),
        muted: Color::from_rgb8(0x66, 0x70, 0x85),
        accent: Color::from_rgb8(0xf0, 0x55, 0x2a),
        accent_hover: Color::from_rgb8(0xff, 0x6a, 0x3d),
        on_accent: Color::WHITE,
        danger: Color::from_rgb8(0xd9, 0x2d, 0x20),
        is_dark: false,
    };

    pub const fn new(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }

    pub fn of(theme: &Theme) -> Self {
        Self::new(theme.extended_palette().is_dark)
    }

    /// The accent faded onto the surface, for selected and active states.
    pub fn accent_soft(&self) -> Color {
        Color {
            a: if self.is_dark { 0.16 } else { 0.12 },
            ..self.accent
        }
    }

    fn palette(&self) -> Palette {
        Palette {
            background: self.background,
            text: self.text,
            primary: self.accent,
            success: Color::from_rgb8(0x12, 0xb7, 0x6a),
            warning: Color::from_rgb8(0xf7, 0x90, 0x09),
            danger: self.danger,
        }
    }
}

/// Whether to draw dark: an explicit choice wins, then the desktop setting.
/// When the desktop reports none (Linux mostly) the tab stays dark.
pub fn is_dark(choice: Option<ThemeChoice>, system: Mode) -> bool {
    match choice {
        Some(ThemeChoice::Dark) => true,
        Some(ThemeChoice::Light) => false,
        None => system != Mode::Light,
    }
}

pub fn build(dark: bool) -> Theme {
    if dark {
        Theme::custom("MacGuitar Dark", Tokens::DARK.palette())
    } else {
        Theme::custom("MacGuitar Light", Tokens::LIGHT.palette())
    }
}

/// Blend `amount` of `over` into `base`.
pub fn mix(base: Color, over: Color, amount: f32) -> Color {
    Color {
        r: base.r + (over.r - base.r) * amount,
        g: base.g + (over.g - base.g) * amount,
        b: base.b + (over.b - base.b) * amount,
        a: base.a,
    }
}

fn soft_shadow(t: &Tokens) -> Shadow {
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, if t.is_dark { 0.35 } else { 0.06 }),
        offset: Vector::new(0.0, 2.0),
        blur_radius: 12.0,
    }
}

// --- containers ---

pub fn window(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.background.into()),
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

/// A floating panel: header, tablature sheet, transport.
pub fn panel(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.surface.into()),
        text_color: Some(t.text),
        border: Border::default()
            .color(t.border)
            .width(1)
            .rounded(PANEL_RADIUS),
        shadow: soft_shadow(&t),
        ..container::Style::default()
    }
}

/// A dialog laid over the dimmed window.
pub fn dialog(theme: &Theme) -> container::Style {
    container::Style {
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 40.0,
        },
        ..panel(theme)
    }
}

pub fn tooltip(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.raised.into()),
        text_color: Some(t.text),
        border: Border::default().color(t.border).width(1).rounded(7),
        shadow: soft_shadow(&t),
        ..container::Style::default()
    }
}

/// A keyboard key, in the shortcut list.
pub fn kbd(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.raised.into()),
        text_color: Some(t.text),
        border: Border::default().color(t.border).width(1).rounded(6),
        ..container::Style::default()
    }
}

/// A small label set off by the accent, like the file format.
pub fn badge(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.accent_soft().into()),
        text_color: Some(t.accent),
        border: Border::default().rounded(6),
        ..container::Style::default()
    }
}

/// A thin vertical separator between groups of controls.
pub fn divider(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.border.into()),
        ..container::Style::default()
    }
}

pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
        ..container::Style::default()
    }
}

// --- buttons ---

/// The main call to action: accent filled.
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let background = match status {
        button::Status::Hovered => t.accent_hover,
        button::Status::Pressed => mix(t.accent, Color::BLACK, 0.12),
        button::Status::Disabled => mix(t.accent, t.surface, 0.5),
        button::Status::Active => t.accent,
    };
    button::Style {
        background: Some(background.into()),
        text_color: t.on_accent,
        border: Border::default().rounded(CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The play button: an accent disc.
pub fn play_button(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: Border::default().rounded(Radius::new(999.0)),
        ..primary_button(theme, status)
    }
}

/// Icon buttons that only show their frame when hovered.
pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let background = match status {
        button::Status::Hovered => Some(t.raised.into()),
        button::Status::Pressed => Some(mix(t.raised, t.text, 0.08).into()),
        button::Status::Active | button::Status::Disabled => None,
    };
    button::Style {
        background,
        text_color: if status == button::Status::Disabled {
            t.muted
        } else {
            t.text
        },
        border: Border::default().rounded(CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A switch drawn as a button: tinted with the accent while on.
pub fn toggle_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        if !active {
            return ghost_button(theme, status);
        }
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Color {
                a: 0.26,
                ..t.accent
            },
            _ => t.accent_soft(),
        };
        button::Style {
            background: Some(background.into()),
            text_color: t.accent,
            border: Border::default().rounded(CONTROL_RADIUS),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// A button opening a menu, drawn like the lists.
pub fn select_button(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let border_color = match status {
        button::Status::Hovered | button::Status::Pressed => mix(t.border, t.text, 0.25),
        _ => t.border,
    };
    button::Style {
        background: Some(t.raised.into()),
        text_color: t.text,
        border: Border::default()
            .color(border_color)
            .width(1)
            .rounded(CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A floating menu, like the lists open.
pub fn menu_panel(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.surface.into()),
        text_color: Some(t.text),
        border: Border::default()
            .color(t.border)
            .width(1)
            .rounded(CONTROL_RADIUS + 2.0),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, if t.is_dark { 0.45 } else { 0.12 }),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..container::Style::default()
    }
}

/// An entry of a menu: the selected one carries the accent.
pub fn menu_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let background = match (selected, status) {
            (true, _) => Some(t.accent_soft().into()),
            (false, button::Status::Hovered | button::Status::Pressed) => Some(t.raised.into()),
            _ => None,
        };
        button::Style {
            background,
            text_color: if selected { t.accent } else { t.text },
            border: Border::default().rounded(CONTROL_RADIUS - 2.0),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// Plain bordered button, for secondary actions.
pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => mix(t.raised, t.text, 0.06),
        _ => t.raised,
    };
    button::Style {
        background: Some(background.into()),
        text_color: t.text,
        border: Border::default()
            .color(t.border)
            .width(1)
            .rounded(CONTROL_RADIUS),
        shadow: Shadow::default(),
        snap: true,
    }
}

// --- inputs ---

pub fn pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let t = Tokens::of(theme);
    let border_color = match status {
        pick_list::Status::Active => t.border,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => {
            mix(t.border, t.text, 0.25)
        }
    };
    pick_list::Style {
        text_color: t.text,
        placeholder_color: t.muted,
        handle_color: t.muted,
        background: t.raised.into(),
        border: Border::default()
            .color(border_color)
            .width(1)
            .rounded(CONTROL_RADIUS),
    }
}

pub fn menu(theme: &Theme) -> iced::overlay::menu::Style {
    let t = Tokens::of(theme);
    iced::overlay::menu::Style {
        background: t.surface.into(),
        border: Border::default()
            .color(t.border)
            .width(1)
            .rounded(CONTROL_RADIUS),
        text_color: t.text,
        selected_text_color: t.accent,
        selected_background: t.accent_soft().into(),
        shadow: soft_shadow(&t),
    }
}

pub fn slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let t = Tokens::of(theme);
    let handle = match status {
        slider::Status::Active => t.accent,
        slider::Status::Hovered | slider::Status::Dragged => t.accent_hover,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (t.accent.into(), t.border.into()),
            width: 4.0,
            border: Border::default().rounded(2),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: handle.into(),
            border_width: 2.0,
            border_color: t.surface,
        },
    }
}

pub fn progress(theme: &Theme) -> progress_bar::Style {
    let t = Tokens::of(theme);
    progress_bar::Style {
        background: t.border.into(),
        bar: Background::Color(t.accent),
        border: Border::default().rounded(2),
    }
}

pub fn scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );
    let scroller = Color {
        a: if hovered { 0.45 } else { 0.22 },
        ..t.text
    };
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: scroller.into(),
            border: Border::default().rounded(999),
        },
    };
    let default = scrollable::default(theme, status);
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: default.auto_scroll,
    }
}
