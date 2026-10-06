//! The colors of the tablature, and the font of its frets.

use iced::{Color, Font, Theme};

use crate::ui::theme::{Tokens, mix};

/// Fret numbers: taller, narrower digits than the interface font, which
/// read better at the size of a tab.
pub const FRET_FONT: Font = Font::with_name("Noto Sans");

/// Colors the tablature draws with, taken from the active theme so the tab
/// follows the system light or dark setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TablatureColors {
    /// Notes, bar lines and effect glyphs.
    pub foreground: Color,
    /// String lines, which sit behind the notes.
    pub string_line: Color,
    /// Stems and beams, a step behind the frets.
    pub rhythm: Color,
    /// The beat being played.
    pub accent: Color,
    /// Measure numbers and playing annotations.
    pub muted: Color,
    /// The sheet the tab is drawn on: frets cut the string lines with it.
    pub paper: Color,
    /// The sheet behind the measure being played.
    pub highlight: Color,
    /// The bar marking the beat being played.
    pub cursor: Color,
    /// The sheet behind the measures being looped.
    pub loop_band: Color,
}

impl TablatureColors {
    pub fn of(theme: &Theme) -> Self {
        let t = Tokens::of(theme);
        // opaque blends, so the frets can cut the string lines with them
        let highlight = mix(t.surface, t.text, if t.is_dark { 0.055 } else { 0.045 });
        Self {
            foreground: t.text,
            // the strings recede behind the notes: mostly sheet, with enough
            // text mixed in to stay visible on either tone
            string_line: mix(t.surface, t.text, if t.is_dark { 0.24 } else { 0.28 }),
            rhythm: mix(t.surface, t.text, 0.6),
            accent: t.accent,
            muted: t.muted,
            paper: t.surface,
            highlight,
            // neutral, like the highlight: the accent is kept for the frets
            // being played, and faded onto the dark sheet it turns brown
            cursor: mix(highlight, t.text, if t.is_dark { 0.12 } else { 0.1 }),
            loop_band: mix(t.surface, t.accent, if t.is_dark { 0.08 } else { 0.06 }),
        }
    }

    /// The colors of a measure drawn over the highlight.
    pub fn highlighted(self) -> Self {
        Self {
            paper: self.highlight,
            ..self
        }
    }

    /// The colors of a measure drawn over the loop band.
    pub fn looped(self) -> Self {
        Self {
            paper: self.loop_band,
            ..self
        }
    }

    /// Draw the rhythm with these colors, so it sits behind the frets.
    pub fn for_rhythm(self) -> Self {
        Self {
            foreground: self.rhythm,
            ..self
        }
    }
}
