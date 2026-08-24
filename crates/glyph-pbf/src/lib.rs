//! Writes the protocol buffer MapLibre GL JS fetches from a `glyphs` URL.
//!
//! The schema is the one `src/style/parse_glyph_pbf.ts` reads:
//!
//! ```proto
//! message glyphs    { repeated fontstack stacks = 1; }
//! message fontstack { required string name = 1; required string range = 2; repeated glyph glyphs = 3; }
//! message glyph     { required uint32 id = 1; optional bytes bitmap = 2;
//!                     required uint32 width = 3; required uint32 height = 4;
//!                     required sint32 left = 5; required sint32 top = 6; required uint32 advance = 7; }
//! ```
//!
//! Two details of it matter more than the rest, because they are what let a shaped glyph be
//! expressed at all: `left` and `top` are *signed*, so a glyph can be asked for at an offset from
//! the pen position, and nothing anywhere ties the `id` to a character.

#![forbid(unsafe_code)]

mod writer;

use maplibre_text_domain::{RasterGlyph, RANGE_SIZE, TOP_ORIGIN};
use writer::PbfWriter;

/// A glyph as it goes into the file: which codepoint MapLibre will ask for it by, what it looks
/// like, and how far the pen moves after it.
pub struct PbfGlyph<'a> {
    pub codepoint: u32,
    pub raster: &'a RasterGlyph,
    pub advance: u32,
}

/// Writes one block of glyphs.
///
/// A block is written even when it holds nothing: MapLibre notes that it has asked for a block and
/// will not ask again, so an error or a missing response would leave those codepoints blank
/// forever, while an empty block simply says there is nothing there.
pub fn write_range<'a>(
    fontstack: &str,
    range: u32,
    glyphs: impl IntoIterator<Item = PbfGlyph<'a>>,
) -> Vec<u8> {
    let start = range * RANGE_SIZE;
    let mut writer = PbfWriter::new();

    writer.message_field(1, |stack| {
        stack.string_field(1, fontstack);
        stack.string_field(2, &format!("{}-{}", start, start + RANGE_SIZE - 1));
        for glyph in glyphs {
            stack.message_field(3, |out| {
                out.uint32_field(1, glyph.codepoint);
                if !glyph.raster.is_blank() {
                    out.bytes_field(2, &glyph.raster.bitmap);
                }
                out.uint32_field(3, glyph.raster.width);
                out.uint32_field(4, glyph.raster.height);
                out.sint32_field(5, glyph.raster.left);
                out.sint32_field(6, glyph.raster.bearing_y - TOP_ORIGIN);
                out.uint32_field(7, glyph.advance);
            });
        }
    });

    writer.finish()
}
