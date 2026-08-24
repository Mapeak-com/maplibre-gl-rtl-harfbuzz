//! Checks the rasterizer against glyphs a real glyph server produced.
//!
//! The glyph protocol buffer has conventions that no document states: where `top` is measured from,
//! whether a glyph's bounds are rounded or truncated, whether an advance is rounded or floored, and
//! exactly how a distance maps onto a byte. Getting any of them wrong shifts every label on the map
//! by a pixel or two, or makes text a shade too heavy, in ways that are easy to look at and not
//! notice.
//!
//! So the test is against the thing itself: the same font a public glyph server draws with, and the
//! block of glyphs it serves for it. The fixtures are fetched by `npm run fetch-fonts`, and the test
//! skips itself when they are not there, so that a checkout without them still builds and tests.

use std::path::PathBuf;

use maplibre_font_set::{FontInstance, FontSet};
use maplibre_sdf_rasterizer::rasterize;
use maplibre_text_domain::{GLYPH_BORDER, TOP_ORIGIN};

mod pbf;

/// A glyph as the reference file has it.
use pbf::ReferenceGlyph;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

/// The fixtures, or `None` with a note, so that a checkout without them is not a failure.
fn fixtures() -> Option<(FontSet, Vec<ReferenceGlyph>)> {
    let font = std::fs::read(fixture("NotoSans-Regular.ttf")).ok()?;
    let reference = std::fs::read(fixture("noto-sans-regular-0-255.pbf")).ok()?;

    let mut fonts = FontSet::new();
    fonts.add(font, FontInstance::default())?;
    Some((fonts, pbf::read(&reference)))
}

macro_rules! fixtures_or_skip {
    () => {
        match fixtures() {
            Some(fixtures) => fixtures,
            None => {
                eprintln!("skipped: run `npm run fetch-fonts` to fetch the glyph server fixtures");
                return;
            }
        }
    };
}

/// Every glyph of the block, drawn here and drawn by the server, compared metric by metric.
///
/// A handful of glyphs land a pixel apart, because the two rasterizers round a bound that falls
/// exactly halfway differently. That is allowed for, and the allowance is small enough that a real
/// mistake -- a wrong origin, a wrong scale -- could not hide inside it.
#[test]
fn metrics_match_a_glyph_server() {
    let (fonts, reference) = fixtures_or_skip!();
    let font = fonts.get(0).expect("the fixture font");

    let (mut compared, mut off_by_one) = (0, 0);
    for glyph in &reference {
        let Some(ch) = char::from_u32(glyph.id) else {
            continue;
        };
        let Some((_, index)) = fonts.lookup(ch) else {
            continue;
        };

        let drawn = rasterize(font.face(), ttf_parser::GlyphId(index), 0.0, 0.0);
        compared += 1;

        assert_eq!(
            font.advance(index) as i64,
            glyph.advance as i64,
            "advance for U+{:04X}: a glyph server truncates it rather than rounding",
            glyph.id
        );

        let differences = [
            (drawn.width as i64 - glyph.width as i64),
            (drawn.height as i64 - glyph.height as i64),
            (drawn.left as i64 - glyph.left as i64),
            ((drawn.bearing_y - TOP_ORIGIN) as i64 - glyph.top as i64),
        ];
        for difference in differences {
            assert!(
                difference.abs() <= 1,
                "U+{:04X} is {difference} pixels away from the glyph server's, which is more than \
                 rounding can explain",
                glyph.id
            );
        }
        if differences.iter().any(|difference| *difference != 0) {
            off_by_one += 1;
        }
    }

    assert!(compared > 150, "expected a full block of Latin glyphs, got {compared}");
    // About one glyph in fifteen lands a pixel apart, on bounds that fall almost exactly halfway.
    // The limit is here to catch a systematic shift, which would move most of them at once.
    assert!(
        off_by_one * 10 < compared,
        "{off_by_one} of {compared} glyphs differ by a pixel, which is more than rounding explains"
    );
}

/// The distance field itself, which is what decides how heavy the text looks.
#[test]
fn distance_fields_match_a_glyph_server() {
    let (fonts, reference) = fixtures_or_skip!();
    let font = fonts.get(0).expect("the fixture font");

    let (mut total, mut count, mut worst) = (0u64, 0u64, 0u8);
    let mut glyphs = 0;

    for glyph in &reference {
        let Some(ch) = char::from_u32(glyph.id) else {
            continue;
        };
        let Some((_, index)) = fonts.lookup(ch) else {
            continue;
        };

        let drawn = rasterize(font.face(), ttf_parser::GlyphId(index), 0.0, 0.0);
        // Only the glyphs whose bounds agree can be compared pixel for pixel.
        if drawn.width != glyph.width || drawn.height != glyph.height || glyph.bitmap.is_empty() {
            continue;
        }
        assert_eq!(
            drawn.bitmap.len(),
            ((glyph.width + 2 * GLYPH_BORDER as u32) * (glyph.height + 2 * GLYPH_BORDER as u32))
                as usize,
            "the padded bitmap of U+{:04X} is not the size the format says",
            glyph.id
        );

        glyphs += 1;
        for (drawn, expected) in drawn.bitmap.iter().zip(&glyph.bitmap) {
            let difference = drawn.abs_diff(*expected);
            total += difference as u64;
            count += 1;
            worst = worst.max(difference);
        }
    }

    assert!(glyphs > 100, "expected to compare a good number of glyphs, got {glyphs}");
    let mean = total as f64 / count as f64;
    assert!(
        mean < 3.0,
        "the distance field is off by {mean:.2} on average, which points at the encoding rather \
         than at the two rasterizers disagreeing"
    );
    assert!(worst < 48, "one value is off by {worst}, far more than antialiasing explains");
}
