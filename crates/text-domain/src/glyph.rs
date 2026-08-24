//! Glyphs: which one, where, and what it looks like once drawn.

use crate::constants::OFFSET_STEPS_PER_PIXEL;

/// Which font of the fallback chain a glyph came from.
pub type FontIndex = u16;

/// A glyph's index within its font. Not a codepoint: after shaping there may be no codepoint that
/// corresponds to it.
pub type GlyphIndex = u16;

/// How far a glyph sits from the pen position, in quarters of a pixel, y upwards.
///
/// Whole pixels would be enough for a glyph that sits on the baseline, but not for a mark: a niqqud
/// point a quarter of a pixel out of place under a letter is visible at the sizes labels are drawn
/// at, because MapLibre scales the glyph up from its 24 pixel em.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Offset {
    pub x: i16,
    pub y: i16,
}

impl Offset {
    pub const ZERO: Offset = Offset { x: 0, y: 0 };

    pub fn from_pixels(x: f32, y: f32) -> Self {
        Offset {
            x: (x * OFFSET_STEPS_PER_PIXEL).round() as i16,
            y: (y * OFFSET_STEPS_PER_PIXEL).round() as i16,
        }
    }

    pub fn x_pixels(self) -> f32 {
        self.x as f32 / OFFSET_STEPS_PER_PIXEL
    }

    pub fn y_pixels(self) -> f32 {
        self.y as f32 / OFFSET_STEPS_PER_PIXEL
    }
}

/// Everything that distinguishes one drawn glyph from another.
///
/// Two glyphs with the same key are the same picture at the same place with the same advance, so
/// they can share one codepoint -- which is what keeps the pool from running out and the glyph
/// atlas from filling up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font: FontIndex,
    pub glyph: GlyphIndex,
    pub offset: Offset,
    /// How far the pen moves after this glyph, in whole pixels. A glyph protocol buffer cannot
    /// carry a fraction here, which is why [`Offset`] can.
    pub advance: i16,
    /// Whether the glyph was shaped right to left. It has no effect on what is drawn; it is here so
    /// that the direction survives into the pass that reorders lines, and so that the same picture
    /// in the two directions gets two codepoints, which is what makes that possible.
    pub rtl: bool,
}

/// The glyph a codepoint draws as when nothing has shaped it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainGlyph {
    pub font: FontIndex,
    pub glyph: GlyphIndex,
    /// The font's own advance for it, in whole pixels.
    pub advance: i16,
}

/// A glyph drawn into a signed distance field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RasterGlyph {
    /// The size of the glyph itself. The bitmap is `2 * GLYPH_BORDER` larger in each direction.
    pub width: u32,
    pub height: u32,
    /// From the pen position to the left edge of the glyph.
    pub left: i32,
    /// From the top edge of the glyph down to the baseline, positive above it. This is not the
    /// `top` a glyph protocol buffer carries; see `TOP_ORIGIN`.
    pub bearing_y: i32,
    /// `(width + 2 * GLYPH_BORDER) * (height + 2 * GLYPH_BORDER)` values, or empty for a glyph with
    /// nothing to draw, such as a space.
    pub bitmap: Vec<u8>,
}

impl RasterGlyph {
    pub fn blank() -> Self {
        Self::default()
    }

    pub fn is_blank(&self) -> bool {
        self.bitmap.is_empty()
    }
}
