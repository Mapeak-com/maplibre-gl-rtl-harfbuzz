//! The vocabulary the rest of the workspace is written in, and the two capabilities the layers
//! above depend on rather than implement.
//!
//! Nothing here knows how to shape text or how to draw a glyph. It states what a shaped glyph is,
//! what a drawn glyph is, and what the constants of MapLibre's text pipeline are; the crates that
//! can actually do those things implement [`TextShaping`] and [`GlyphRasterizing`], and the crate
//! that orchestrates them names only these traits. That is what keeps the orchestration testable
//! without a font, and what would let a different shaping engine or a different rasterizer be put
//! in without the layer above noticing.

#![forbid(unsafe_code)]

mod constants;
mod glyph;
mod text;

pub use constants::*;
pub use glyph::*;
pub use text::*;

/// Turning a string into glyphs, and a shaped string back into reading order.
pub trait TextShaping {
    /// Shapes a whole string, in logical order.
    fn shape(&self, text: &str) -> Vec<Piece>;

    /// Reorders an already-shaped string into lines in visual order.
    ///
    /// `direction_of` reports, for a codepoint, whether it stands for a shaped glyph and whether
    /// that glyph was shaped right to left. `sections` and `line_breaks` are indexed in UTF-16 code
    /// units, which is how MapLibre counts.
    fn reorder(
        &self,
        text: &str,
        direction_of: &dyn Fn(u32) -> Option<bool>,
        sections: &[u32],
        line_breaks: &[u32],
    ) -> Vec<VisualLine>;
}

/// Drawing a glyph, and answering what a lone codepoint would draw as.
pub trait GlyphRasterizing {
    /// Draws a glyph into a distance field, offset from the pen position by `offset`.
    fn rasterize(&self, font: FontIndex, glyph: GlyphIndex, offset: Offset) -> RasterGlyph;

    /// The glyph a codepoint draws as when nothing has shaped it, in the first font that has one.
    fn plain_glyph(&self, codepoint: u32) -> Option<PlainGlyph>;
}
