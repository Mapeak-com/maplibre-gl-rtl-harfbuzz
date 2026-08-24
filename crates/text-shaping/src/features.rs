//! Which OpenType features to let a run use.
//!
//! Shaping is only worth doing where it changes something. A run of Latin text that comes back
//! exactly as it went in can stay characters, and staying characters is worth a great deal: it
//! keeps a private-use codepoint from being spent on every letter of every label, keeps the glyph
//! atlas from filling up with a second copy of the alphabet, and keeps MapLibre's line breaking
//! able to see the spaces and hyphens it breaks on.
//!
//! What stops that from happening is not shaping proper but typographic refinement. Kerning moves
//! the `e` in `Tel` by a fraction of a pixel; a standard ligature fuses `fi`. Both are improvements
//! on their own terms, and both turn an entire word into glyphs no codepoint stands for.
//!
//! So for the scripts MapLibre already draws correctly, refinements are turned off, leaving text in
//! those scripts laid out exactly as it is without this plugin. Everything a script actually needs
//! -- required ligatures, mark attachment, Indic reordering, Arabic joining -- is untouched,
//! because none of it is on this list. A run in one of those scripts that *does* need shaping, such
//! as Hebrew with vowel points, still gets it: what is disabled here is optional, and the mark
//! positioning that puts a niqqud point under its letter is not.

use rustybuzz::Feature;
use ttf_parser::Tag;
use unicode_script::Script;

/// Features that make text look better without being needed to write it correctly.
const REFINEMENTS: [&[u8; 4]; 6] = [b"kern", b"liga", b"clig", b"dlig", b"hlig", b"calt"];

/// The features to shape a run of this script with.
pub fn features_for(script: Script) -> Vec<Feature> {
    if needs_shaping(script) {
        return Vec::new();
    }
    REFINEMENTS
        .iter()
        .map(|tag| Feature::new(Tag::from_bytes(tag), 0, ..))
        .collect()
}

/// Whether a script is one MapLibre cannot draw a codepoint at a time.
///
/// The list runs the other way round -- the scripts that *are* fine are named, and everything else
/// is assumed to need shaping -- because being wrong in that direction only costs a codepoint from
/// the pool, while being wrong in the other direction would draw the text incorrectly.
fn needs_shaping(script: Script) -> bool {
    !matches!(
        script,
        Script::Common
            | Script::Inherited
            | Script::Unknown
            | Script::Latin
            | Script::Greek
            | Script::Cyrillic
            | Script::Armenian
            | Script::Georgian
            | Script::Hebrew
            | Script::Han
            | Script::Hiragana
            | Script::Katakana
            | Script::Bopomofo
            | Script::Hangul
            | Script::Cherokee
            | Script::Ethiopic
            | Script::Canadian_Aboriginal
            | Script::Coptic
            | Script::Deseret
            | Script::Gothic
            | Script::Ogham
            | Script::Old_Italic
            | Script::Osage
            | Script::Runic
            | Script::Vai
            | Script::Yi
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_a_complex_script_alone() {
        assert!(features_for(Script::Devanagari).is_empty());
        assert!(features_for(Script::Arabic).is_empty());
        assert!(features_for(Script::Khmer).is_empty());
    }

    #[test]
    fn turns_refinements_off_for_a_simple_script() {
        assert_eq!(features_for(Script::Latin).len(), REFINEMENTS.len());
        assert!(features_for(Script::Latin).iter().all(|feature| feature.value == 0));
    }

    #[test]
    fn treats_hebrew_as_simple_so_that_unpointed_hebrew_stays_characters() {
        // Its vowel points are positioned by `mark`, which is not on the list, so pointed Hebrew is
        // still shaped.
        assert!(!features_for(Script::Hebrew).is_empty());
    }
}
