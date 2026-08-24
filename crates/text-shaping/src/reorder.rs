//! The second pass: putting a shaped, line-broken string into the order it is read in.
//!
//! MapLibre asks the plugin to do this after it has decided where the lines break, because the
//! Unicode Bidirectional Algorithm resolves the order of a *line*, not of a paragraph: the same
//! words reorder differently depending on where the line ends.
//!
//! By this point the text is no longer the text the map's data had -- shaping has replaced some of
//! it with glyph references, which have no direction of their own. Rather than remembering what
//! every string was, the algorithm is run over a stand-in string in which each glyph reference is
//! replaced by a character of the direction that glyph was shaped in. Everything shaping left
//! alone -- spaces, digits, punctuation, Latin words, and the invisible marks that exist only to
//! steer this algorithm -- is still there as itself, so the neutral characters between the glyphs
//! resolve exactly as they would have in the original.

use maplibre_text_domain::VisualLine;
use unicode_bidi::{BidiInfo, Level};

/// Stand-ins for a shaped glyph, chosen only for their strong direction.
const RTL_STAND_IN: char = '\u{05D0}'; // Hebrew letter alef
const LTR_STAND_IN: char = 'A';

/// Reorders `text` into lines in visual order.
///
/// `direction_of` reports whether a codepoint is a shaped glyph and, if so, whether it was shaped
/// right to left. `sections` holds one entry per UTF-16 code unit, as MapLibre passes it, and
/// `line_breaks` are offsets in UTF-16 code units, as MapLibre computes them.
pub fn reorder(
    text: &str,
    direction_of: &dyn Fn(u32) -> Option<bool>,
    sections: &[u32],
    line_breaks: &[u32],
) -> Vec<VisualLine> {
    let mut codepoints: Vec<u32> = Vec::new();
    let mut char_sections: Vec<u32> = Vec::new();
    let mut stand_in = String::with_capacity(text.len());
    // Line breaks arrive as UTF-16 offsets; everything here counts in characters.
    let mut breaks: Vec<usize> = Vec::with_capacity(line_breaks.len());
    let mut remaining_breaks = line_breaks;

    let mut code_unit = 0usize;
    for ch in text.chars() {
        while let Some((&next, rest)) = remaining_breaks.split_first() {
            if next as usize > code_unit {
                break;
            }
            breaks.push(codepoints.len());
            remaining_breaks = rest;
        }

        codepoints.push(ch as u32);
        char_sections.push(sections.get(code_unit).copied().unwrap_or(0));
        stand_in.push(match direction_of(ch as u32) {
            Some(true) => RTL_STAND_IN,
            Some(false) => LTR_STAND_IN,
            None => ch,
        });
        code_unit += ch.len_utf16();
    }
    breaks.resize(line_breaks.len(), codepoints.len());

    // Byte offsets into the stand-in string, which is not as long as the original.
    let mut char_to_byte: Vec<usize> = stand_in.char_indices().map(|(at, _)| at).collect();
    char_to_byte.push(stand_in.len());

    let bidi = BidiInfo::new(&stand_in, None);

    // The same split MapLibre's own `breakLines` makes, so that a plugin-shaped label breaks into
    // the same number of lines as an unshaped one.
    let mut lines = Vec::new();
    let mut start = 0usize;
    for &line_break in &breaks {
        let line_break = line_break.min(codepoints.len()).max(start);
        lines.push(reorder_line(&bidi, &char_to_byte, &codepoints, &char_sections, start..line_break));
        start = line_break;
    }
    if start < codepoints.len() {
        lines.push(reorder_line(&bidi, &char_to_byte, &codepoints, &char_sections, start..codepoints.len()));
    }

    lines
}

/// The index of the character starting at or after a byte offset.
fn char_at_byte(char_to_byte: &[usize], byte: usize) -> usize {
    char_to_byte.partition_point(|&at| at < byte)
}

fn reorder_line(
    bidi: &BidiInfo,
    char_to_byte: &[usize],
    codepoints: &[u32],
    sections: &[u32],
    line: core::ops::Range<usize>,
) -> VisualLine {
    let mut visual = VisualLine {
        codepoints: Vec::with_capacity(line.len()),
        sections: Vec::with_capacity(line.len()),
    };
    if line.is_empty() {
        return visual;
    }

    let line_bytes = char_to_byte[line.start]..char_to_byte[line.end];

    // A hard line break starts a new paragraph, so a line can in principle straddle two of them.
    // Each part is reordered against its own paragraph, and the parts stay in the order they were
    // written.
    for paragraph in &bidi.paragraphs {
        let overlap =
            line_bytes.start.max(paragraph.range.start)..line_bytes.end.min(paragraph.range.end);
        if overlap.start >= overlap.end {
            continue;
        }

        // One level per character of the whole stand-in string, with rule L1 applied to this part.
        let levels = bidi.reordered_levels_per_char(paragraph, overlap.clone());
        let first = char_at_byte(char_to_byte, overlap.start).max(line.start);
        let last = char_at_byte(char_to_byte, overlap.end).min(line.end);

        let part: Vec<Level> = levels[first..last].to_vec();
        for offset in BidiInfo::reorder_visual(&part) {
            let index = first + offset;
            visual.codepoints.push(codepoints[index]);
            visual.sections.push(sections[index]);
        }
    }

    visual
}
