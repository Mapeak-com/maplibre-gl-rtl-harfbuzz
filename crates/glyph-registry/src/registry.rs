//! Ties shaping and drawing together through the codepoints that stand for shaped glyphs.
//!
//! This is the whole trick the plugin turns, in one type. Shaping produces glyphs that no longer
//! correspond to any character; MapLibre can only carry characters. So each distinct glyph is given
//! a codepoint out of a private stretch of Unicode, the shaped text is handed back as a string of
//! those, and when MapLibre later asks what those codepoints look like, the registry draws the
//! glyphs they stand for -- offsets and all -- into the reply.
//!
//! It depends on the two capabilities by name only ([`TextShaping`], [`GlyphRasterizing`]), so the
//! bookkeeping here can be tested without a font, and neither the shaping engine nor the rasterizer
//! is baked in.

use std::collections::HashMap;

use maplibre_glyph_pbf::{write_range, PbfGlyph};
use maplibre_text_domain::{
    is_pool_codepoint, FontIndex, GlyphIndex, GlyphKey, GlyphRasterizing, InspectedGlyph, Offset,
    Piece, RasterGlyph, TextShaping, VisualLine, RANGE_SIZE,
};

use crate::allocator::CodepointAllocator;

/// What a drawn picture depends on: the advance and the direction a [`GlyphKey`] also carries do
/// not change a single pixel of it, so glyphs that differ only in those share one drawing.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RasterKey {
    font: FontIndex,
    glyph: GlyphIndex,
    offset: Offset,
}

impl From<GlyphKey> for RasterKey {
    fn from(key: GlyphKey) -> Self {
        Self {
            font: key.font,
            glyph: key.glyph,
            offset: key.offset,
        }
    }
}

/// How many labels' shaping results to remember. Shaping the same name over and over is the common
/// case -- a road is named on every tile it crosses, at every zoom -- and the cache is what makes
/// running a full shaping engine on every label affordable.
const SHAPING_CACHE_LIMIT: usize = 8192;

pub struct GlyphRegistry<S: TextShaping, R: GlyphRasterizing> {
    shaping: S,
    rasterizing: R,
    allocator: CodepointAllocator,

    by_key: HashMap<GlyphKey, u32>,
    by_codepoint: HashMap<u32, GlyphKey>,
    /// Codepoints allocated since they were last taken, to be passed to whoever draws them.
    unreported: Vec<(u32, GlyphKey)>,

    shaped: HashMap<String, String>,
    drawn: HashMap<RasterKey, RasterGlyph>,
}

impl<S: TextShaping, R: GlyphRasterizing> GlyphRegistry<S, R> {
    pub fn new(shaping: S, rasterizing: R) -> Self {
        Self {
            shaping,
            rasterizing,
            allocator: CodepointAllocator::new(),
            by_key: HashMap::new(),
            by_codepoint: HashMap::new(),
            unreported: Vec::new(),
            shaped: HashMap::new(),
            drawn: HashMap::new(),
        }
    }

    /// Narrows the codepoints this registry allocates to one stretch of the pool.
    pub fn restrict_codepoints(&mut self, start: u32, end: u32) {
        self.allocator.restrict(start, end);
    }

    /// Shapes one run of text into one codepoint per glyph to be drawn, in logical order.
    pub fn shape(&mut self, text: &str) -> String {
        if let Some(shaped) = self.shaped.get(text) {
            return shaped.clone();
        }

        let mut out = String::with_capacity(text.len());
        for piece in self.shaping.shape(text) {
            match piece {
                Piece::Verbatim(ch) => out.push(ch),
                Piece::Glyph(glyph) => {
                    let key = GlyphKey {
                        font: glyph.font,
                        glyph: glyph.glyph,
                        offset: Offset::from_pixels(glyph.dx, glyph.dy),
                        advance: glyph.advance.round() as i16,
                        rtl: glyph.rtl,
                    };
                    // Running out of codepoints drops the glyph rather than drawing a wrong one.
                    // It takes a map with tens of thousands of distinct shaped glyphs on screen to
                    // get here, and a dropped glyph is at least visibly missing.
                    if let Some(codepoint) = self.allocate(key) {
                        out.push(char::from_u32(codepoint).expect("the pool holds valid codepoints"));
                    }
                }
            }
        }

        if self.shaped.len() >= SHAPING_CACHE_LIMIT {
            self.shaped.clear();
        }
        self.shaped.insert(text.to_string(), out.clone());
        out
    }

    /// Puts shaped, line-broken text into the order it is read in.
    pub fn reorder(&self, text: &str, sections: &[u32], line_breaks: &[u32]) -> Vec<VisualLine> {
        self.shaping.reorder(
            text,
            &|codepoint| self.by_codepoint.get(&codepoint).map(|key| key.rtl),
            sections,
            line_breaks,
        )
    }

    /// The codepoints allocated since this was last called. They have to reach whoever draws the
    /// glyphs before the block they fall in is asked for.
    pub fn take_new_glyphs(&mut self) -> Vec<(u32, GlyphKey)> {
        core::mem::take(&mut self.unreported)
    }

    /// Records what a codepoint allocated elsewhere stands for.
    pub fn register(&mut self, codepoint: u32, key: GlyphKey) {
        self.by_codepoint.insert(codepoint, key);
        self.by_key.insert(key, codepoint);
    }

    /// Marks a block of codepoints as drawn, so that nothing new is allocated into it.
    pub fn seal_range(&mut self, range: u32) {
        self.allocator.seal(range);
    }

    /// Draws every glyph of one block into the protocol buffer MapLibre parses.
    ///
    /// Codepoints from the pool are drawn as the glyphs they were allocated for. Every other
    /// codepoint is drawn as itself, which is what lets the plugin serve a style's whole fontstack
    /// rather than only the parts of it that needed shaping -- and means the text that did not need
    /// shaping is drawn from the same font files as the text that did.
    pub fn glyph_pbf(&mut self, fontstack: &str, range: u32) -> Vec<u8> {
        let start = range * RANGE_SIZE;
        let keys: Vec<(u32, GlyphKey)> = (start..start.saturating_add(RANGE_SIZE))
            .filter_map(|codepoint| Some((codepoint, self.key_for(codepoint)?)))
            .collect();

        for &(_, key) in &keys {
            self.draw(key);
        }

        write_range(
            fontstack,
            range,
            keys.iter().map(|&(codepoint, key)| PbfGlyph {
                codepoint,
                raster: &self.drawn[&RasterKey::from(key)],
                advance: key.advance.max(0) as u32,
            }),
        )
    }

    /// Shapes text without allocating anything, resolving the characters shaping left alone to the
    /// glyphs they would be drawn as, so that a tool sees the whole line the way the map will.
    pub fn inspect(&self, text: &str) -> Vec<InspectedGlyph> {
        self.shaping
            .shape(text)
            .into_iter()
            .filter_map(|piece| match piece {
                Piece::Glyph(glyph) => Some(InspectedGlyph {
                    font: glyph.font,
                    glyph: glyph.glyph,
                    offset: Offset::from_pixels(glyph.dx, glyph.dy),
                    advance: glyph.advance.round() as i16,
                    rtl: glyph.rtl,
                    codepoint: None,
                }),
                Piece::Verbatim(ch) => {
                    let plain = self.rasterizing.plain_glyph(ch as u32)?;
                    Some(InspectedGlyph {
                        font: plain.font,
                        glyph: plain.glyph,
                        offset: Offset::ZERO,
                        advance: plain.advance,
                        rtl: false,
                        codepoint: Some(ch as u32),
                    })
                }
            })
            .collect()
    }

    /// The glyphs of a line in the order they are drawn, which for right-to-left text is not the
    /// order they were shaped in.
    ///
    /// This runs the very path the map runs -- shape, then reorder -- rather than approximating it,
    /// so it allocates codepoints as it goes and belongs on a registry kept for looking at text
    /// rather than on one drawing a map.
    pub fn inspect_visual(&mut self, text: &str) -> Vec<InspectedGlyph> {
        let shaped = self.shape(text);
        self.reorder(&shaped, &[], &[])
            .into_iter()
            .flat_map(|line| line.codepoints)
            .filter_map(|codepoint| self.describe(codepoint))
            .collect()
    }

    /// What a codepoint of a shaped string draws as.
    fn describe(&self, codepoint: u32) -> Option<InspectedGlyph> {
        if let Some(&key) = self.by_codepoint.get(&codepoint) {
            return Some(InspectedGlyph {
                font: key.font,
                glyph: key.glyph,
                offset: key.offset,
                advance: key.advance,
                rtl: key.rtl,
                codepoint: None,
            });
        }

        let plain = self.rasterizing.plain_glyph(codepoint)?;
        Some(InspectedGlyph {
            font: plain.font,
            glyph: plain.glyph,
            offset: Offset::ZERO,
            advance: plain.advance,
            rtl: false,
            codepoint: Some(codepoint),
        })
    }

    /// The glyphs MapLibre would draw for this text with no plugin at all: one per codepoint, at
    /// the font's own advance, with nothing moved. Only useful for showing what shaping changed.
    pub fn inspect_unshaped(&self, text: &str) -> Vec<InspectedGlyph> {
        text.chars()
            .filter_map(|ch| {
                let plain = self.rasterizing.plain_glyph(ch as u32)?;
                Some(InspectedGlyph {
                    font: plain.font,
                    glyph: plain.glyph,
                    offset: Offset::ZERO,
                    advance: plain.advance,
                    rtl: false,
                    codepoint: Some(ch as u32),
                })
            })
            .collect()
    }

    /// Draws one glyph on its own, the same way it would be drawn into a block.
    pub fn draw_glyph(
        &mut self,
        font: FontIndex,
        glyph: GlyphIndex,
        offset: Offset,
    ) -> &RasterGlyph {
        let key = GlyphKey {
            font,
            glyph,
            offset,
            advance: 0,
            rtl: false,
        };
        self.draw(key);
        &self.drawn[&RasterKey::from(key)]
    }

    /// The codepoint standing for a glyph, allocating one the first time it is needed.
    fn allocate(&mut self, key: GlyphKey) -> Option<u32> {
        if let Some(&codepoint) = self.by_key.get(&key) {
            return Some(codepoint);
        }
        let codepoint = self.allocator.next()?;
        self.register(codepoint, key);
        self.unreported.push((codepoint, key));
        Some(codepoint)
    }

    /// What to draw for a codepoint: the glyph allocated for it, or the codepoint's own glyph.
    fn key_for(&self, codepoint: u32) -> Option<GlyphKey> {
        if let Some(&key) = self.by_codepoint.get(&codepoint) {
            return Some(key);
        }
        if is_pool_codepoint(codepoint) {
            return None;
        }

        let plain = self.rasterizing.plain_glyph(codepoint)?;
        Some(GlyphKey {
            font: plain.font,
            glyph: plain.glyph,
            offset: Offset::ZERO,
            advance: plain.advance,
            rtl: false,
        })
    }

    fn draw(&mut self, key: GlyphKey) {
        let raster_key = RasterKey::from(key);
        if self.drawn.contains_key(&raster_key) {
            return;
        }
        let glyph = self
            .rasterizing
            .rasterize(key.font, key.glyph, key.offset);
        self.drawn.insert(raster_key, glyph);
    }
}
