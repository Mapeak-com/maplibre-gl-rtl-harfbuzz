//! What shaping has to get right, stated as text going in and glyphs coming out.
//!
//! The fixtures are fetched by `npm run fetch-fonts`; without them the tests skip themselves rather
//! than fail, so that a fresh checkout still builds and tests.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use maplibre_font_set::{FontInstance, FontSet};
use maplibre_text_domain::{Piece, TextShaping};
use maplibre_text_shaping::HarfBuzzShaping;

/// Latin first, so that every other script falls through to its own file, as a style would set it.
const FIXTURE_FONTS: [&str; 5] = [
    "NotoSans.ttf",
    "NotoSansHebrew.ttf",
    "NotoSansArabic.ttf",
    "NotoSansDevanagari.ttf",
    "NotoSansTamil.ttf",
];

fn shaper() -> Option<HarfBuzzShaping> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../debug/public/fonts");
    let mut fonts = FontSet::new();
    for name in FIXTURE_FONTS {
        fonts.add(std::fs::read(directory.join(name)).ok()?, FontInstance::default())?;
    }
    Some(HarfBuzzShaping::new(Rc::new(RefCell::new(fonts))))
}

macro_rules! shaper_or_skip {
    () => {
        match shaper() {
            Some(shaper) => shaper,
            None => {
                eprintln!("skipped: run `npm run fetch-fonts` to fetch the fixture fonts");
                return;
            }
        }
    };
}

/// What the pieces are, as a string: a character shaping left alone, or `.` for a glyph.
fn outline(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Verbatim(ch) => *ch,
            Piece::Glyph(_) => '.',
        })
        .collect()
}

fn glyphs(pieces: &[Piece]) -> Vec<maplibre_text_domain::ShapedGlyph> {
    pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Glyph(glyph) => Some(*glyph),
            Piece::Verbatim(_) => None,
        })
        .collect()
}

/// Text that shaping does not change has to come back as text, or the plugin would spend a
/// codepoint on every letter of every Latin label, and MapLibre's line breaking would stop being
/// able to see the spaces and hyphens it breaks on.
#[test]
fn text_that_needs_no_shaping_stays_text() {
    let shaper = shaper_or_skip!();
    for text in ["Tel Aviv-Yafo", "New Delhi 110001", "רחוב הרצל 12", "תל אביב Tel Aviv"] {
        assert_eq!(outline(&shaper.shape(text)), text, "{text:?} should have been left alone");
    }
}

/// Hebrew vowel points are the case MapLibre cannot express at all: they have to be hung under the
/// letter they belong to rather than placed after it.
#[test]
fn hebrew_vowel_points_are_hung_on_their_letters() {
    let shaper = shaper_or_skip!();
    let pieces = shaper.shape("שְׁדֵרוֹת");
    let glyphs = glyphs(&pieces);

    assert_eq!(glyphs.len(), pieces.len(), "pointed Hebrew has to be shaped, not passed through");
    assert!(glyphs.iter().all(|glyph| glyph.rtl), "Hebrew is a right-to-left script");

    let marks: Vec<_> = glyphs.iter().filter(|glyph| glyph.advance == 0.0).collect();
    assert!(marks.len() >= 4, "expected the vowel points to take no width of their own");
    assert!(
        marks.iter().any(|mark| mark.dx != 0.0),
        "a vowel point has to be moved to sit under its letter; none of them were"
    );
}

/// Arabic asks for the same thing on top of letters that have already changed shape to join.
#[test]
fn arabic_joins_and_then_carries_its_vowel_marks() {
    let shaper = shaper_or_skip!();
    let joined = glyphs(&shaper.shape("القاهرة"));
    let pointed = glyphs(&shaper.shape("مَدِينَةُ"));

    assert!(!joined.is_empty(), "Arabic always needs shaping");
    assert!(
        pointed.iter().any(|glyph| glyph.advance == 0.0),
        "the vowel marks should take no width of their own"
    );
}

/// A Devanagari cluster reorders and fuses: the written form is not the order it is stored in, and
/// the conjunct is a glyph no codepoint stands for.
#[test]
fn devanagari_reorders_and_fuses() {
    let shaper = shaper_or_skip!();
    let pieces = shaper.shape("दिल्ली");
    let glyphs = glyphs(&pieces);

    assert_eq!(glyphs.len(), pieces.len(), "Devanagari always needs shaping");
    assert!(
        glyphs.len() < "दिल्ली".chars().count(),
        "the conjunct ल्ली should have fused several characters into one glyph"
    );
    assert!(glyphs.iter().all(|glyph| !glyph.rtl), "Devanagari runs left to right");
}

/// Tamil fuses too, which is worth its own case because it does so through a different mechanism.
#[test]
fn tamil_fuses() {
    let shaper = shaper_or_skip!();
    let text = "சென்னை";
    let pieces = shaper.shape(text);
    assert!(
        pieces.len() < text.chars().count(),
        "expected Tamil to produce fewer glyphs than characters"
    );
}

/// Shaping runs before line breaking, so it has to leave the pieces in the order they were written.
/// Putting them into the order they are read in is the second pass, and it is the second pass that
/// has to be told where the lines break.
#[test]
fn shaping_leaves_the_pieces_in_the_order_they_were_written() {
    let shaper = shaper_or_skip!();
    let pieces = shaper.shape("שלום Hello");
    assert_eq!(
        outline(&pieces),
        "שלום Hello",
        "neither run should have been reversed yet"
    );
}
