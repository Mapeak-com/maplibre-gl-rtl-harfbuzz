//! Writes a glyph range as this crate rasterizes it, for comparison against the same range served
//! by a real glyph server.
//!
//! ```sh
//! cargo run --release --example calibrate -- NotoSans.ttf 0 out.pbf
//! ```
//!
//! The point of the comparison is that the glyph PBF format has conventions no document states --
//! where `top` is measured from, whether bounds are rounded or truncated -- and the way to be sure
//! of them is to check against glyphs a server produced.

use maplibre_shaper_wasm::Shaper;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let font_path = arguments.next().expect("usage: calibrate <font> <range> <out>");
    let range: u32 = arguments.next().expect("a range").parse().expect("a number");
    let out_path = arguments.next().expect("an output path");

    let mut shaper = Shaper::new();
    let data = std::fs::read(&font_path).expect("the font file");
    assert!(shaper.add_font(data, 400.0, 100.0) >= 0, "{font_path} is not a font this can read");

    let pbf = shaper.glyph_pbf("Calibration", range);
    std::fs::write(&out_path, &pbf).expect("to write the output");
    eprintln!("wrote {} bytes to {out_path}", pbf.len());
}
