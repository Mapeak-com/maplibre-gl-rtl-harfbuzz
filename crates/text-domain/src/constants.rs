//! The fixed numbers of MapLibre's text pipeline, and of the codepoint pool this plugin allocates
//! from. Every one of them is a property of MapLibre or of the glyph protocol buffer format rather
//! than a choice made here, so each says where it comes from.

/// MapLibre lays text out in a 24 pixel em: `src/symbol/one_em.ts`.
pub const EM_PX: f32 = 24.0;

/// The padding a glyph bitmap carries on every side: `GLYPH_PBF_BORDER` in
/// `src/style/parse_glyph_pbf.ts`.
pub const GLYPH_BORDER: i32 = 3;

/// How far a distance field reaches, in pixels: `SDF_PX` in `src/shaders/glsl/symbol_sdf.fragment.glsl`.
pub const SDF_RADIUS: f32 = 8.0;

/// Where the contour sits within the field's range. The shader finds it again at
/// `(256 - 64) / 256`, which is this same quarter.
pub const SDF_CUTOFF: f32 = 0.25;

/// A glyph's `top` metric is measured down from an origin this many pixels above the baseline
/// rather than from the baseline itself.
///
/// This is a property of the glyph protocol buffer format that no document states.
/// `GlyphManager._drawGlyph` applies the same correction, at its own approximation of 27.5, to make
/// locally drawn glyphs line up with server-drawn ones. The value here is what real glyph servers
/// emit, measured against the Noto Sans that `demotiles.maplibre.org` serves.
pub const TOP_ORIGIN: i32 = 26;

/// Glyphs are fetched in blocks of this many codepoints: the `{range}` of a `glyphs` URL.
pub const RANGE_SIZE: u32 = 256;

/// The first codepoint of plane 15, where the supplementary private use areas begin. Everything
/// from here to [`POOL_END`] is set aside for shaped glyphs; no text a map draws will contain one
/// of them, and MapLibre carries them like any other codepoint.
pub const POOL_START: u32 = 0xF_0000;

/// The last codepoint of plane 16 that a string may contain.
pub const POOL_END: u32 = 0x10_FFFD;

/// Codepoints inside the pool's span that no string may contain.
pub const NONCHARACTERS: [u32; 2] = [0xF_FFFE, 0xF_FFFF];

/// Offsets are kept to a quarter of a pixel. That is finer than the distance field itself resolves,
/// and coarse enough that the same mark under the same letter reuses one glyph.
pub const OFFSET_STEPS_PER_PIXEL: f32 = 4.0;

/// Whether a codepoint belongs to the pool rather than standing for a character.
pub fn is_pool_codepoint(codepoint: u32) -> bool {
    (POOL_START..=POOL_END).contains(&codepoint)
}
