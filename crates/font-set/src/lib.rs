//! The font files a style is shaped and drawn with.
//!
//! Fonts are tried in the order they were added, which makes the list a fallback chain: the first
//! file with a glyph for a character is the one that character is shaped and drawn with. It is the
//! rule the style specification already gives for a `text-font` stack, applied to files rather than
//! to names -- and applied per character, so that a label reading `שדרות רוטשילד 12` can take its
//! letters from a Hebrew font and its digits from wherever the chain finds them.

#![forbid(unsafe_code)]

mod variations;

pub use variations::FontInstance;

use maplibre_text_domain::{FontIndex, GlyphIndex, PlainGlyph, EM_PX};
use rustybuzz::Face;

/// One font file, parsed, with the scale that takes it to MapLibre's 24 pixel em.
pub struct Font {
    face: Face<'static>,
    scale: f32,
}

impl Font {
    /// The face, for shaping.
    pub fn face(&self) -> &Face<'static> {
        &self.face
    }

    /// Font units to pixels.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn glyph_index(&self, ch: char) -> Option<GlyphIndex> {
        self.face.glyph_index(ch).map(|id| id.0)
    }

    /// The font's own advance for a glyph, in pixels.
    pub fn advance(&self, glyph: GlyphIndex) -> f32 {
        self.face
            .glyph_hor_advance(ttf_parser::GlyphId(glyph))
            .unwrap_or(0) as f32
            * self.scale
    }
}

#[derive(Default)]
pub struct FontSet {
    fonts: Vec<Font>,
}

impl FontSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a font file at one instance and appends it to the chain, returning its index, or
    /// `None` if the file is not a font this can read.
    ///
    /// The bytes are deliberately leaked. A parsed face borrows the file it came from, and the
    /// fonts a map draws with live exactly as long as the map does, so the alternative would be a
    /// self-referential structure bought with `unsafe` for no gain.
    pub fn add(&mut self, data: Vec<u8>, instance: FontInstance) -> Option<FontIndex> {
        let data: &'static [u8] = Box::leak(data.into_boxed_slice());
        let mut face = Face::from_slice(data, 0)?;
        // A variable font file has to be told which instance to read, or it draws whichever master
        // happens to be in the file -- for several Noto families, the thin one.
        variations::apply(&mut face, instance);
        let units_per_em = face.units_per_em();
        if units_per_em <= 0 {
            return None;
        }
        self.fonts.push(Font {
            face,
            scale: EM_PX / units_per_em as f32,
        });
        Some((self.fonts.len() - 1) as FontIndex)
    }

    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    pub fn get(&self, index: FontIndex) -> Option<&Font> {
        self.fonts.get(index as usize)
    }

    /// The first font with a glyph for this character, and that glyph.
    pub fn lookup(&self, ch: char) -> Option<(FontIndex, GlyphIndex)> {
        self.fonts
            .iter()
            .enumerate()
            .find_map(|(index, font)| font.glyph_index(ch).map(|glyph| (index as FontIndex, glyph)))
    }

    /// The font a character should be shaped with, preferring one the run is already using so that
    /// a shared character -- a space, a digit, a combining mark -- does not split a run away from
    /// the letters it belongs with.
    pub fn lookup_preferring(
        &self,
        ch: char,
        preferred: Option<FontIndex>,
    ) -> Option<(FontIndex, GlyphIndex)> {
        if let Some(index) = preferred {
            if let Some(glyph) = self.get(index).and_then(|font| font.glyph_index(ch)) {
                return Some((index, glyph));
            }
        }
        self.lookup(ch)
    }

    /// What a codepoint draws as on its own, with the advance a glyph server would report for it.
    pub fn plain_glyph(&self, codepoint: u32) -> Option<PlainGlyph> {
        let ch = char::from_u32(codepoint)?;
        let (font_index, glyph) = self.lookup(ch)?;
        Some(PlainGlyph {
            font: font_index,
            glyph,
            // Truncated rather than rounded, which is what a glyph server emits. Keeping to its
            // convention means a label of Latin text is laid out to the same width with this plugin
            // as without it, so turning the plugin on does not reflow every label on the map.
            advance: self.get(font_index)?.advance(glyph) as i16,
        })
    }
}
