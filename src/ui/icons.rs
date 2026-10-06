//! Interface icons, drawn as small SVGs on a 24 unit grid.
//!
//! They are painted black and tinted at draw time, so one shape serves every
//! state and both themes.

use iced::widget::svg;
use iced::{Color, Element};

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Open,
    Play,
    Pause,
    Stop,
    Previous,
    Next,
    Solo,
    Mute,
    Metronome,
    CountIn,
    Tempo,
    Volume,
    Fullscreen,
    ExitFullscreen,
    Alert,
    Up,
    Down,
    Sound,
    Loop,
    Recent,
    ZoomIn,
    ZoomOut,
}

impl Icon {
    fn shape(self) -> &'static str {
        match self {
            Self::Open => {
                r#"<path d="M3 7.5V6a2 2 0 0 1 2-2h3.6l2 2.2H19a2 2 0 0 1 2 2v1.3"/><path d="M3.2 9.5h17.6l-1.6 8.7a2 2 0 0 1-2 1.6H5.8a2 2 0 0 1-2-1.6z"/>"#
            }
            Self::Play => {
                r##"<path d="M8 5.3v13.4a.8.8 0 0 0 1.2.7l10.6-6.7a.8.8 0 0 0 0-1.4L9.2 4.6A.8.8 0 0 0 8 5.3z" fill="#000"/>"##
            }
            Self::Pause => {
                r##"<rect x="6" y="4.5" width="4" height="15" rx="1.2" fill="#000"/><rect x="14" y="4.5" width="4" height="15" rx="1.2" fill="#000"/>"##
            }
            Self::Stop => {
                r##"<rect x="5.5" y="5.5" width="13" height="13" rx="2.2" fill="#000"/>"##
            }
            Self::Previous => {
                r##"<path d="M18 6.2v11.6a.7.7 0 0 1-1.1.6L8.6 12.6a.7.7 0 0 1 0-1.2l8.3-5.8a.7.7 0 0 1 1.1.6z" fill="#000"/><path d="M6 5.5v13"/>"##
            }
            Self::Next => {
                r##"<path d="M6 6.2v11.6a.7.7 0 0 0 1.1.6l8.3-5.8a.7.7 0 0 0 0-1.2L7.1 5.6a.7.7 0 0 0-1.1.6z" fill="#000"/><path d="M18 5.5v13"/>"##
            }
            Self::Solo => {
                r#"<path d="M4 15.5v-3.5a8 8 0 0 1 16 0v3.5"/><path d="M4 15h3.2v5.5H5.6A1.6 1.6 0 0 1 4 18.9z"/><path d="M20 15h-3.2v5.5h1.6a1.6 1.6 0 0 0 1.6-1.6z"/>"#
            }
            Self::Mute => {
                r#"<path d="M3.5 9.5h3.2L11.5 5.5v13l-4.8-4H3.5z"/><path d="M15.5 9.5l5 5"/><path d="M20.5 9.5l-5 5"/>"#
            }
            Self::Metronome => {
                r#"<path d="M9 3.5h6l4 17H5z"/><path d="M12 16l5-8.5"/><path d="M7.3 15.5h9.4"/>"#
            }
            Self::CountIn => {
                r#"<circle cx="12" cy="13.5" r="7.5"/><path d="M12 13.5V9.5"/><path d="M9.5 2.8h5"/><path d="M19 6.5l-1.3 1.3"/>"#
            }
            Self::Tempo => {
                r##"<path d="M4.6 18.5a8.5 8.5 0 1 1 14.8 0"/><path d="M12 14.5l4-4.5"/><circle cx="12" cy="14.5" r="1" fill="#000"/>"##
            }
            Self::Volume => {
                r#"<path d="M3.5 9.5h3.2L11.5 5.5v13l-4.8-4H3.5z"/><path d="M15.3 9a4.2 4.2 0 0 1 0 6"/><path d="M18 6.3a8 8 0 0 1 0 11.4"/>"#
            }
            Self::Fullscreen => {
                r#"<path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9"/><path d="M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9"/><path d="M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15"/><path d="M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15"/>"#
            }
            Self::ExitFullscreen => {
                r#"<path d="M9 4v3.5A1.5 1.5 0 0 1 7.5 9H4"/><path d="M20 9h-3.5A1.5 1.5 0 0 1 15 7.5V4"/><path d="M15 20v-3.5a1.5 1.5 0 0 1 1.5-1.5H20"/><path d="M4 15h3.5A1.5 1.5 0 0 1 9 16.5V20"/>"#
            }
            Self::Recent => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#,
            Self::Loop => {
                r#"<path d="M17 2.5l3 3-3 3"/><path d="M4 11.5v-2a4 4 0 0 1 4-4h12"/><path d="M7 21.5l-3-3 3-3"/><path d="M20 12.5v2a4 4 0 0 1-4 4H4"/>"#
            }
            Self::Sound => {
                r#"<path d="M9 18V5.5l11-2V16"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="17.5" cy="16" r="2.5"/>"#
            }
            Self::Up => r#"<path d="M6 15l6-6 6 6"/>"#,
            Self::ZoomIn => {
                r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.3 15.3l5 5"/><path d="M10.5 7.8v5.4"/><path d="M7.8 10.5h5.4"/>"#
            }
            Self::ZoomOut => {
                r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.3 15.3l5 5"/><path d="M7.8 10.5h5.4"/>"#
            }
            Self::Down => r#"<path d="M6 9l6 6 6-6"/>"#,
            Self::Alert => {
                r##"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5.5"/><circle cx="12" cy="16.5" r="1.1" fill="#000" stroke="none"/>"##
            }
        }
    }

    fn handle(self) -> svg::Handle {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
            self.shape()
        );
        svg::Handle::from_memory(source.into_bytes())
    }
}

/// The icon at `size` pixels, tinted with `color`.
pub fn icon<'a, Message: 'a>(icon: Icon, size: f32, color: Color) -> Element<'a, Message> {
    svg(icon.handle())
        .width(size)
        .height(size)
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}

/// The application logo, in its own colors.
pub fn logo<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    const LOGO: &[u8] = include_bytes!("../../resources/icon/icon.svg");
    svg(svg::Handle::from_memory(LOGO))
        .width(size)
        .height(size)
        .into()
}
