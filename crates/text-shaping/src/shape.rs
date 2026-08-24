//! Turns a string into positioned glyphs.
//!
//! This is the part MapLibre has no way to do on its own: it assumes one codepoint is one glyph,
//! laid out left to right at the glyph's own advance. Real text is not like that. A Devanagari
//! cluster reorders and fuses into a single glyph, an Arabic letter takes a different shape
//! depending on its neighbours, and a Hebrew niqqud point has to be hung under the letter it
//! belongs to rather than placed after it.
//!
//! The text is split into runs of one direction, one script and one font, each run is shaped with
//! HarfBuzz (via `rustybuzz`, which is a port of it rather than a binding to it), and the glyphs
//! come back with the offsets and advances the font's own GSUB and GPOS tables ask for.
//!
//! Runs that shaping leaves untouched -- most Latin text -- are handed back as their original
//! characters instead of as glyphs, so that ordinary labels stay ordinary codepoints all the way
//! through MapLibre: line breaking still sees its spaces and hyphens, and the glyph atlas does not
//! fill up with a private copy of the Latin alphabet.

use maplibre_font_set::FontSet;

use crate::features::features_for;
use maplibre_text_domain::{Piece, ShapedGlyph};
use rustybuzz::{Direction, Script, UnicodeBuffer};
use ttf_parser::Tag;
use unicode_bidi::{BidiInfo, Level};
use unicode_script::{Script as UnicodeScript, UnicodeScript as _};

/// Shapes a whole string, returning its pieces in logical order.
///
/// Right-to-left runs are put back into logical order here; putting them into visual order is the
/// job of the second pass, once MapLibre has decided where the lines break.
pub fn shape(fonts: &FontSet, text: &str) -> Vec<Piece> {
    if fonts.is_empty() || text.is_empty() {
        return text.chars().map(Piece::Verbatim).collect();
    }

    let bidi = BidiInfo::new(text, None);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let levels: Vec<Level> = chars.iter().map(|&(at, _)| bidi.levels[at]).collect();
    let scripts = resolve_scripts(&chars);
    let fonts_per_char = resolve_fonts(fonts, &chars);

    let mut pieces = Vec::with_capacity(chars.len());
    let mut run_start = 0;

    for index in 0..=chars.len() {
        let ends_run = index == chars.len()
            || fonts_per_char[index].is_none()
            || chars[index].1 == '\n'
            || (index > run_start
                && (levels[index] != levels[run_start]
                    || scripts[index] != scripts[run_start]
                    || fonts_per_char[index] != fonts_per_char[run_start]));

        if ends_run && index > run_start {
            shape_run(
                fonts,
                text,
                &chars,
                run_start..index,
                levels[run_start],
                scripts[run_start],
                fonts_per_char[run_start].unwrap(),
                &mut pieces,
            );
            run_start = index;
        }

        if index < chars.len() && (fonts_per_char[index].is_none() || chars[index].1 == '\n') {
            // A newline, or a character no font covers: nothing to shape, and it must not be swept
            // into a neighbouring run.
            pieces.push(Piece::Verbatim(chars[index].1));
            run_start = index + 1;
        }
    }

    pieces
}

/// The script of every character, with the ones that have no script of their own -- digits,
/// punctuation, spaces, combining marks -- taking the script of the text around them.
fn resolve_scripts(chars: &[(usize, char)]) -> Vec<UnicodeScript> {
    let mut scripts: Vec<UnicodeScript> = chars
        .iter()
        .map(|&(_, ch)| ch.script())
        .collect();

    let is_own_script = |script: UnicodeScript| {
        !matches!(
            script,
            UnicodeScript::Common | UnicodeScript::Inherited | UnicodeScript::Unknown
        )
    };

    let mut current = UnicodeScript::Common;
    for script in scripts.iter_mut() {
        if is_own_script(*script) {
            current = *script;
        } else {
            *script = current;
        }
    }

    // Anything before the first character with a script of its own -- a leading quotation mark, a
    // house number -- belongs with what follows it.
    let mut current = UnicodeScript::Common;
    for script in scripts.iter_mut().rev() {
        if is_own_script(*script) {
            current = *script;
        } else if !is_own_script(*script) {
            *script = current;
        }
    }

    scripts
}

/// The font every character is shaped with.
///
/// Almost always this is simply the first font in the chain that has the character, which is what
/// makes it possible to hand a run back as characters rather than as glyphs: a character can only
/// stay a character if the glyph drawn for it on its own is the glyph shaping used for it, and the
/// glyph drawn for a lone character comes from the first font that has it.
///
/// The exception is a character with no script of its own that combines with what precedes it -- a
/// combining mark, a zero-width joiner. Those have to be shaped with the font their base was shaped
/// with, or the cluster falls apart, and it does not matter that the choice differs from what a lone
/// one would get: they never occur alone.
fn resolve_fonts(fonts: &FontSet, chars: &[(usize, char)]) -> Vec<Option<maplibre_text_domain::FontIndex>> {
    let mut chosen = Vec::with_capacity(chars.len());
    let mut previous: Option<maplibre_text_domain::FontIndex> = None;

    for (index, &(_, ch)) in chars.iter().enumerate() {
        let combines = index > 0 && ch.script() == UnicodeScript::Inherited;
        let font = fonts
            .lookup_preferring(ch, if combines { previous } else { None })
            .map(|(font, _)| font);
        previous = font;
        chosen.push(font);
    }

    chosen
}

fn shape_run(
    fonts: &FontSet,
    text: &str,
    chars: &[(usize, char)],
    range: core::ops::Range<usize>,
    level: Level,
    script: UnicodeScript,
    font_index: maplibre_text_domain::FontIndex,
    out: &mut Vec<Piece>,
) {
    let font = match fonts.get(font_index) {
        Some(font) => font,
        None => {
            out.extend(chars[range].iter().map(|&(_, ch)| Piece::Verbatim(ch)));
            return;
        }
    };

    let start_byte = chars[range.start].0;
    let end_byte = chars
        .get(range.end)
        .map(|&(at, _)| at)
        .unwrap_or(text.len());
    let run_text = &text[start_byte..end_byte];
    let rtl = level.is_rtl();

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(run_text);
    buffer.set_direction(if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    });
    if let Some(script) = Script::from_iso15924_tag(Tag::from_bytes_lossy(
        script.short_name().as_bytes(),
    )) {
        buffer.set_script(script);
    }

    let shaped = rustybuzz::shape(font.face(), &features_for(script), buffer);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();

    // HarfBuzz returns a right-to-left run already in visual order; logical order is what the rest
    // of the pipeline works in, up until the line breaks are known.
    let order: Vec<usize> = if rtl {
        (0..infos.len()).rev().collect()
    } else {
        (0..infos.len()).collect()
    };

    if let Some(unchanged) = unchanged_run(fonts, font_index, chars, &range, &infos, &positions, &order) {
        out.extend(unchanged);
        return;
    }

    // A glyph PBF can only carry a whole number of pixels of advance, so the pen would drift away
    // from where the font puts it -- by up to half a pixel per glyph, and in the same direction for
    // a whole word. Instead the pen is tracked exactly, each glyph is advanced to the nearest whole
    // pixel of the true position, and whatever is left over is added to that glyph's own offset,
    // which the glyph PBF *can* carry at sub-pixel precision. The run then ends where the font says
    // it should, with every glyph within a quarter pixel of its true place.
    //
    // This has to be counted in the order the glyphs are drawn in, which for a right-to-left run is
    // the order HarfBuzz returned them in, before they are put back into logical order below.
    let mut pen = 0.0f32;
    let mut whole_pixels = 0i32;
    let mut glyphs = Vec::with_capacity(infos.len());

    for index in 0..infos.len() {
        let position = &positions[index];
        let next_pen = pen + position.x_advance as f32 * font.scale();
        let next_whole_pixels = next_pen.round() as i32;

        glyphs.push(ShapedGlyph {
            font: font_index,
            glyph: infos[index].glyph_id as maplibre_text_domain::GlyphIndex,
            dx: position.x_offset as f32 * font.scale() + (pen - whole_pixels as f32),
            dy: position.y_offset as f32 * font.scale(),
            advance: (next_whole_pixels - whole_pixels) as f32,
            rtl,
        });

        pen = next_pen;
        whole_pixels = next_whole_pixels;
    }

    out.extend(order.iter().map(|&index| Piece::Glyph(glyphs[index])));
}

/// Whether shaping left a run exactly as it found it -- one glyph per character, in order, at the
/// font's plain advances -- in which case the run is better off staying as characters.
fn unchanged_run<'a>(
    fonts: &FontSet,
    font_index: maplibre_text_domain::FontIndex,
    chars: &'a [(usize, char)],
    range: &core::ops::Range<usize>,
    infos: &[rustybuzz::GlyphInfo],
    positions: &[rustybuzz::GlyphPosition],
    order: &[usize],
) -> Option<impl Iterator<Item = Piece> + 'a> {
    if infos.len() != range.len() {
        return None;
    }

    let font = fonts.get(font_index)?;
    for (offset, &index) in order.iter().enumerate() {
        let ch = chars[range.start + offset].1;
        let position = &positions[index];
        if position.x_offset != 0 || position.y_offset != 0 {
            return None;
        }
        let glyph = infos[index].glyph_id as maplibre_text_domain::GlyphIndex;
        // The glyph has to be the one the glyph URL would serve for this character on its own,
        // which is the one the first font covering it has.
        if fonts.lookup(ch) != Some((font_index, glyph)) {
            return None;
        }
        let plain_advance = font
            .face()
            .glyph_hor_advance(ttf_parser::GlyphId(glyph))?;
        if position.x_advance != plain_advance as i32 {
            return None;
        }
    }

    Some(chars[range.clone()].iter().map(|&(_, ch)| Piece::Verbatim(ch)))
}
