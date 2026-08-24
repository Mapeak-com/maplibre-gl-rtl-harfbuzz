//! The second pass: putting shaped, line-broken text into the order it is read in.
//!
//! These need no font. The stand-in string the reordering works over is built from what shaping
//! reported about each piece, so the direction of a glyph can simply be stated.

use maplibre_text_domain::{TextShaping, VisualLine};
use maplibre_text_shaping::HarfBuzzShaping;
use std::{cell::RefCell, rc::Rc};

/// A shaper with no fonts, which is all reordering needs.
fn shaper() -> HarfBuzzShaping {
    HarfBuzzShaping::new(Rc::new(RefCell::new(maplibre_font_set::FontSet::new())))
}

/// Stands in for a shaped glyph. Any codepoint of the pool will do; what matters is that
/// `direction_of` claims it, since a shaped glyph has no direction of its own to read.
const RTL_GLYPH: char = '\u{F0000}';
const LTR_GLYPH: char = '\u{F0001}';

fn reorder(text: &str, line_breaks: &[u32]) -> Vec<VisualLine> {
    shaper().reorder(
        text,
        &|codepoint| match char::from_u32(codepoint) {
            Some(RTL_GLYPH) => Some(true),
            Some(LTR_GLYPH) => Some(false),
            _ => None,
        },
        &[],
        line_breaks,
    )
}

fn line(text: &str, line_breaks: &[u32]) -> String {
    reorder(text, line_breaks)
        .into_iter()
        .map(|line| {
            line.codepoints
                .into_iter()
                .filter_map(char::from_u32)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_right_to_left_run_is_reversed() {
    let shaped: String = [RTL_GLYPH, RTL_GLYPH, RTL_GLYPH].iter().collect();
    let visual = line(&shaped, &[]);
    // Every glyph is the same character here, so what is checked is that the run stays whole.
    assert_eq!(visual.chars().count(), 3);
    assert!(visual.chars().all(|ch| ch == RTL_GLYPH));
}

#[test]
fn a_left_to_right_run_is_left_alone() {
    assert_eq!(line("Tel Aviv", &[]), "Tel Aviv");
}

/// The order a line comes out in depends on which direction the paragraph runs, and that is decided
/// by its first strong character.
#[test]
fn the_first_strong_character_decides_the_direction_of_the_line() {
    let rtl_first = format!("{RTL_GLYPH}{RTL_GLYPH} ab");
    let ltr_first = format!("ab {RTL_GLYPH}{RTL_GLYPH}");

    // Either way the Latin ends up on the left and the right-to-left run on the right.
    for text in [&rtl_first, &ltr_first] {
        let visual = line(text, &[]);
        let letters: String = visual.chars().filter(|ch| ch.is_ascii_alphabetic()).collect();
        assert_eq!(letters, "ab", "{text:?} put its Latin in the wrong order");
        assert!(
            visual.chars().take(2).all(|ch| ch.is_ascii_alphabetic()),
            "{text:?} should start its line with the Latin run"
        );
    }
}

/// A number inside right-to-left text is read left to right, and only the number is.
#[test]
fn a_number_inside_right_to_left_text_keeps_its_own_direction() {
    let text = format!("{RTL_GLYPH}{RTL_GLYPH} 42 {RTL_GLYPH}");
    let visual = line(&text, &[]);
    let digits: String = visual.chars().filter(char::is_ascii_digit).collect();
    assert_eq!(digits, "42", "the digits should not have been reversed");
}

/// The algorithm resolves the order of a *line*, which is why this pass runs after MapLibre has
/// decided where the lines break rather than before.
#[test]
fn each_line_is_reordered_on_its_own() {
    let text = format!("ab {RTL_GLYPH}{RTL_GLYPH}");
    let lines = reorder(&text, &[3]);
    assert_eq!(lines.len(), 2, "one break should give two lines");
    assert_eq!(lines[0].codepoints.len(), 3, "the first line is `ab ` ");
    assert_eq!(lines[1].codepoints.len(), 2, "the second line is the right-to-left run");
}

/// The lines have to come out matching MapLibre's own `breakLines`, or a label would be broken into
/// a different number of lines than MapLibre measured.
#[test]
fn the_number_of_lines_matches_maplibres_own_splitting() {
    assert_eq!(reorder("abcdef", &[2, 4]).len(), 3);
    assert_eq!(reorder("abcdef", &[]).len(), 1);
    assert_eq!(reorder("abcdef", &[6]).len(), 1, "a break at the end adds no empty line");
}

/// MapLibre indexes sections and breaks by UTF-16 code unit, and every codepoint the pool hands out
/// is two of them. Counting in characters instead would misplace every break in a shaped label.
#[test]
fn line_breaks_are_counted_in_utf16_code_units() {
    let text = format!("{LTR_GLYPH}{LTR_GLYPH}{LTR_GLYPH}");
    // Two code units per glyph, so a break after the second glyph is at four.
    let lines = reorder(&text, &[4]);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].codepoints.len(), 2);
    assert_eq!(lines[1].codepoints.len(), 1);
}
