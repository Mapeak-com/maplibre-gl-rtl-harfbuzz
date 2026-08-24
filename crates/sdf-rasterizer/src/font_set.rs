//! Binds the rasterizer to a set of font files, which is what the layer above asks for.

use maplibre_font_set::FontSet;
use maplibre_text_domain::{
    FontIndex, GlyphIndex, GlyphRasterizing, Offset, PlainGlyph, RasterGlyph,
};
use std::{cell::RefCell, rc::Rc};

/// Draws glyphs out of a [`FontSet`].
pub struct FontSetRasterizer {
    fonts: Rc<RefCell<FontSet>>,
}

impl FontSetRasterizer {
    pub fn new(fonts: Rc<RefCell<FontSet>>) -> Self {
        Self { fonts }
    }
}

impl GlyphRasterizing for FontSetRasterizer {
    fn rasterize(&self, font: FontIndex, glyph: GlyphIndex, offset: Offset) -> RasterGlyph {
        let fonts = self.fonts.borrow();
        let Some(font) = fonts.get(font) else {
            return RasterGlyph::blank();
        };
        crate::rasterize(
            font.face(),
            ttf_parser::GlyphId(glyph),
            offset.x_pixels(),
            // The offset is measured upwards and the raster grid downwards.
            -offset.y_pixels(),
        )
    }

    fn plain_glyph(&self, codepoint: u32) -> Option<PlainGlyph> {
        self.fonts.borrow().plain_glyph(codepoint)
    }
}
