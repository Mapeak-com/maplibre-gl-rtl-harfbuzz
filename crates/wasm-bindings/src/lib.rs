//! The JavaScript face of the shaper: it wires the layers together and translates between Rust
//! values and the flat typed arrays that cross into JavaScript cheaply.
//!
//! Nothing here decides anything. Shaping lives in `maplibre-text-shaping`, drawing in
//! `maplibre-sdf-rasterizer`, and the bookkeeping that connects them in `maplibre-glyph-registry`;
//! this crate chooses which implementations to use and hands them a font set.

#![forbid(unsafe_code)]

mod encoding;

use std::{cell::RefCell, rc::Rc};

use maplibre_font_set::FontSet;
use maplibre_glyph_registry::GlyphRegistry;
use maplibre_sdf_rasterizer::FontSetRasterizer;
use maplibre_text_domain::{FontIndex, GlyphIndex, Offset};
use maplibre_text_shaping::HarfBuzzShaping;
use wasm_bindgen::prelude::*;

use encoding::{decode_glyph_entries, encode_glyph_entries, encode_pieces, encode_visual_lines};

type Registry = GlyphRegistry<HarfBuzzShaping, FontSetRasterizer>;

/// Shapes text and draws glyphs for one MapLibre context -- a worker, or the main thread.
///
/// The same type plays both parts. In a worker it shapes labels and allocates codepoints for the
/// glyphs it produces; on the main thread it is told what those codepoints mean and draws them.
#[wasm_bindgen]
pub struct Shaper {
    fonts: Rc<RefCell<FontSet>>,
    registry: Registry,
}

#[wasm_bindgen]
impl Shaper {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Shaper {
        let fonts = Rc::new(RefCell::new(FontSet::new()));
        Shaper {
            registry: GlyphRegistry::new(
                HarfBuzzShaping::new(fonts.clone()),
                FontSetRasterizer::new(fonts.clone()),
            ),
            fonts,
        }
    }

    /// Adds a font file to the fallback chain, returning its index, or -1 if it cannot be read.
    ///
    /// `weight` and `width` choose the instance of a variable font, on the same scales CSS uses:
    /// 400 and 100 are what `font-weight: normal` and `font-stretch: normal` mean. They are ignored
    /// by a font that is not variable, and clamped to what a variable one offers.
    ///
    /// Any format `ttf-parser` reads will do -- TrueType, OpenType, variable fonts, collections --
    /// but not the compressed web formats: WOFF and WOFF2 have to be decompressed first.
    #[wasm_bindgen(js_name = addFont)]
    pub fn add_font(&mut self, data: Vec<u8>, weight: f32, width: f32) -> i32 {
        match self
            .fonts
            .borrow_mut()
            .add(data, maplibre_font_set::FontInstance {weight, width})
        {
            Some(index) => index as i32,
            None => -1,
        }
    }

    #[wasm_bindgen(js_name = fontCount)]
    pub fn font_count(&self) -> usize {
        self.fonts.borrow().len()
    }

    /// Narrows the codepoints this instance allocates to one stretch of the pool, so that the
    /// workers of one map never disagree about what a codepoint means.
    #[wasm_bindgen(js_name = restrictCodepoints)]
    pub fn restrict_codepoints(&mut self, start: u32, end: u32) {
        self.registry.restrict_codepoints(start, end);
    }

    /// Shapes one run of text, returning one codepoint per glyph to be drawn, in logical order.
    ///
    /// This is what MapLibre calls `applyArabicShaping`. It is called on a label's text before
    /// MapLibre works out which glyphs the tile will need, which is exactly where a shaped glyph
    /// has to appear for MapLibre to go on and ask for it.
    #[wasm_bindgen(js_name = applyShaping)]
    pub fn apply_shaping(&mut self, text: &str) -> String {
        self.registry.shape(text)
    }

    /// Puts shaped, line-broken text into visual order: MapLibre's `processStyledBidirectionalText`.
    ///
    /// `sections` and `line_breaks` are indexed in UTF-16 code units, as MapLibre passes them. The
    /// result is flat so that it crosses the boundary as one array: a line count, then for each
    /// line its length followed by that many `codepoint, section` pairs.
    #[wasm_bindgen(js_name = processBidi)]
    pub fn process_bidi(&self, text: &str, sections: Vec<u32>, line_breaks: Vec<u32>) -> Vec<u32> {
        encode_visual_lines(self.registry.reorder(text, &sections, &line_breaks))
    }

    /// The codepoints allocated since this was last called, as `[codepoint, font, glyph, dx, dy,
    /// advance, rtl]` for each. They have to reach whoever draws the glyphs before the block they
    /// fall in is asked for.
    #[wasm_bindgen(js_name = takeNewGlyphs)]
    pub fn take_new_glyphs(&mut self) -> Vec<i32> {
        encode_glyph_entries(self.registry.take_new_glyphs())
    }

    /// Records what codepoints allocated elsewhere stand for, in the same encoding.
    #[wasm_bindgen(js_name = registerGlyphs)]
    pub fn register_glyphs(&mut self, entries: Vec<i32>) {
        for (codepoint, key) in decode_glyph_entries(&entries) {
            self.registry.register(codepoint, key);
        }
    }

    /// Marks a block of codepoints as drawn, so nothing new is allocated into it.
    #[wasm_bindgen(js_name = sealRange)]
    pub fn seal_range(&mut self, range: u32) {
        self.registry.seal_range(range);
    }

    /// Draws one block of codepoints into the protocol buffer MapLibre's glyph pipeline parses.
    #[wasm_bindgen(js_name = glyphPbf)]
    pub fn glyph_pbf(&mut self, fontstack: &str, range: u32) -> Vec<u8> {
        self.registry.glyph_pbf(fontstack, range)
    }

    /// Shapes text without allocating anything, for tools that want to look at the result:
    /// `[font, glyph, dx, dy, advance, rtl, codepoint]` per glyph, with -1 for the codepoint of a
    /// glyph that no longer stands for one.
    ///
    /// Offsets are in quarters of a pixel and advances in whole pixels, both at a 24 pixel em.
    pub fn inspect(&self, text: &str) -> Vec<i32> {
        encode_pieces(self.registry.inspect(text))
    }

    /// The glyphs of a line in the order they are drawn, in the same encoding as `inspect`.
    ///
    /// Unlike `inspect`, this allocates codepoints, because it runs the same path the map runs. Use
    /// it on a shaper kept for looking at text rather than on one drawing a map.
    #[wasm_bindgen(js_name = inspectVisual)]
    pub fn inspect_visual(&mut self, text: &str) -> Vec<i32> {
        encode_pieces(self.registry.inspect_visual(text))
    }

    /// The glyphs MapLibre would draw with no plugin at all -- one per codepoint, unmoved -- in the
    /// same encoding as `inspect`, so that a tool can show what shaping changed.
    #[wasm_bindgen(js_name = inspectUnshaped)]
    pub fn inspect_unshaped(&self, text: &str) -> Vec<i32> {
        encode_pieces(self.registry.inspect_unshaped(text))
    }

    /// Draws one glyph on its own, as four little-endian 32 bit integers -- width, height, left and
    /// the distance from the top of the glyph up to the baseline -- followed by the distance field.
    ///
    /// This is the same drawing that goes into a glyph block, so a tool using it shows what the map
    /// shows rather than an approximation of it.
    #[wasm_bindgen(js_name = glyphImage)]
    pub fn glyph_image(&mut self, font: FontIndex, glyph: GlyphIndex, dx: i16, dy: i16) -> Vec<u8> {
        encoding::encode_raster(
            self.registry
                .draw_glyph(font, glyph, Offset { x: dx, y: dy }),
        )
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Re-exported so that a caller can size its own share of the pool without hard-coding the numbers.
#[wasm_bindgen(js_name = codepointPool)]
pub fn codepoint_pool() -> Vec<u32> {
    vec![
        maplibre_text_domain::POOL_START,
        maplibre_text_domain::POOL_END,
    ]
}

/// The width of one entry of the glyph encoding, in 32 bit integers.
#[wasm_bindgen(js_name = glyphEntryWidth)]
pub fn glyph_entry_width() -> usize {
    encoding::ENTRY_WIDTH
}
