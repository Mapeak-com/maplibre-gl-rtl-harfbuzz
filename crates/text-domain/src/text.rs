//! What comes out of shaping, and what comes out of reordering.

use crate::glyph::{FontIndex, GlyphIndex};

/// One glyph, positioned by the font's own rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapedGlyph {
    pub font: FontIndex,
    pub glyph: GlyphIndex,
    /// Offset from the pen position, in pixels, x rightwards and y upwards.
    pub dx: f32,
    pub dy: f32,
    /// How far the pen moves after this glyph, in pixels. Zero for a mark.
    pub advance: f32,
    /// Whether the glyph sits in a right-to-left run.
    pub rtl: bool,
}

/// A piece of shaped text.
///
/// Shaping does not have to change everything it is given. A run it leaves alone comes back as the
/// characters it was, which is worth keeping: MapLibre's line breaking looks for spaces and hyphens
/// by codepoint, and text that is still text keeps working with everything else that reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    Verbatim(char),
    Glyph(ShapedGlyph),
}

/// One line of text in the order it is read, as codepoints paired with the index of the style
/// section each came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisualLine {
    pub codepoints: Vec<u32>,
    pub sections: Vec<u32>,
}

/// A shaped glyph as a tool wants to see it: everything needed to draw it, plus the character it
/// came through unchanged as, where there was one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InspectedGlyph {
    pub font: FontIndex,
    pub glyph: GlyphIndex,
    pub offset: crate::Offset,
    pub advance: i16,
    pub rtl: bool,
    pub codepoint: Option<u32>,
}
