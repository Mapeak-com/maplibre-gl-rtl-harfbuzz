//! Flat encodings for the values that cross into JavaScript.
//!
//! Everything here is a plain typed array rather than an object graph. Crossing the boundary is the
//! expensive part, and these are crossed per label and per glyph block, so each value goes over as
//! one array of numbers that the JavaScript side unpacks with a loop.

use maplibre_text_domain::{GlyphKey, InspectedGlyph, Offset, RasterGlyph, VisualLine};

/// How many 32 bit integers one glyph entry takes.
pub(crate) const ENTRY_WIDTH: usize = 7;

/// `[codepoint, font, glyph, dx, dy, advance, rtl]` per entry.
pub(crate) fn encode_glyph_entries(entries: Vec<(u32, GlyphKey)>) -> Vec<i32> {
    let mut out = Vec::with_capacity(entries.len() * ENTRY_WIDTH);
    for (codepoint, key) in entries {
        out.extend_from_slice(&[
            codepoint as i32,
            key.font as i32,
            key.glyph as i32,
            key.offset.x as i32,
            key.offset.y as i32,
            key.advance as i32,
            key.rtl as i32,
        ]);
    }
    out
}

pub(crate) fn decode_glyph_entries(entries: &[i32]) -> impl Iterator<Item = (u32, GlyphKey)> + '_ {
    entries.chunks_exact(ENTRY_WIDTH).map(|entry| {
        (
            entry[0] as u32,
            GlyphKey {
                font: entry[1] as u16,
                glyph: entry[2] as u16,
                offset: Offset {
                    x: entry[3] as i16,
                    y: entry[4] as i16,
                },
                advance: entry[5] as i16,
                rtl: entry[6] != 0,
            },
        )
    })
}

/// `[font, glyph, dx, dy, advance, rtl, codepoint]` per glyph, with -1 where a glyph no longer
/// stands for a codepoint.
pub(crate) fn encode_pieces(glyphs: Vec<InspectedGlyph>) -> Vec<i32> {
    let mut out = Vec::with_capacity(glyphs.len() * ENTRY_WIDTH);
    for glyph in glyphs {
        out.extend_from_slice(&[
            glyph.font as i32,
            glyph.glyph as i32,
            glyph.offset.x as i32,
            glyph.offset.y as i32,
            glyph.advance as i32,
            glyph.rtl as i32,
            glyph.codepoint.map_or(-1, |codepoint| codepoint as i32),
        ]);
    }
    out
}

/// A line count, then for each line its length followed by that many `codepoint, section` pairs.
pub(crate) fn encode_visual_lines(lines: Vec<VisualLine>) -> Vec<u32> {
    let mut out = vec![lines.len() as u32];
    for line in lines {
        out.push(line.codepoints.len() as u32);
        for (codepoint, section) in line.codepoints.iter().zip(&line.sections) {
            out.push(*codepoint);
            out.push(*section);
        }
    }
    out
}

/// Four little-endian 32 bit integers -- width, height, left, bearing -- then the distance field.
pub(crate) fn encode_raster(glyph: &RasterGlyph) -> Vec<u8> {
    let header = [
        glyph.width as i32,
        glyph.height as i32,
        glyph.left,
        glyph.bearing_y,
    ];
    let mut out = Vec::with_capacity(header.len() * 4 + glyph.bitmap.len());
    for value in header {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&glyph.bitmap);
    out
}
