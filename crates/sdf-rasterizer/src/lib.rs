//! Rasterizes a glyph outline into the signed distance field MapLibre's symbol shader samples.
//!
//! The encoding is the one shared by `sdf-glyph-foundry` (what a glyph server produces) and
//! `@mapbox/tiny-sdf` (what MapLibre draws locally): a distance of zero maps to 191, the field
//! grows darker outwards over `SDF_RADIUS` pixels, and it is cut off a quarter of the radius inside
//! the outline. `symbol_sdf.fragment.glsl` finds the contour again at `(256 - 64) / 256`, which is
//! that same 191.
//!
//! Distances are measured exactly, against the flattened outline, rather than approximated from a
//! coverage bitmap the way a glyph server does it. The glyphs are small enough that it costs little,
//! and it is what lets a glyph be drawn at a fraction of a pixel from where the grid would put it,
//! which is how a mark ends up exactly where the font asks for it.

#![forbid(unsafe_code)]

mod font_set;

pub use font_set::FontSetRasterizer;

use maplibre_text_domain::{RasterGlyph, GLYPH_BORDER, SDF_CUTOFF, SDF_RADIUS};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// The largest distance the encoding can still tell apart, outside the outline.
const MAX_OUTER_DISTANCE: f32 = SDF_RADIUS * (1.0 - SDF_CUTOFF);

/// A line segment of a flattened outline, in pixels, y growing downwards.
#[derive(Clone, Copy)]
struct Segment {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Segment {
    /// The squared distance from a point to this segment.
    fn distance_squared(&self, x: f32, y: f32) -> f32 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        let length_squared = dx * dx + dy * dy;
        let t = if length_squared <= f32::EPSILON {
            0.0
        } else {
            (((x - self.x0) * dx + (y - self.y0) * dy) / length_squared).clamp(0.0, 1.0)
        };
        let ex = x - (self.x0 + t * dx);
        let ey = y - (self.y0 + t * dy);
        ex * ex + ey * ey
    }
}

/// Collects a glyph outline as line segments in pixel space, flattening curves as it goes.
struct Flattener {
    segments: Vec<Segment>,
    /// Font units to pixels.
    scale: f32,
    /// Sub-pixel translation, applied so that a glyph positioned at a fraction of a pixel can be
    /// rasterized where it actually sits rather than snapped to the grid.
    offset_x: f32,
    offset_y: f32,
    start_x: f32,
    start_y: f32,
    x: f32,
    y: f32,
}

/// How far a flattened curve may stray from the true one, in pixels.
const FLATTEN_TOLERANCE: f32 = 0.05;

impl Flattener {
    fn new(scale: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            segments: Vec::new(),
            scale,
            offset_x,
            offset_y,
            start_x: 0.0,
            start_y: 0.0,
            x: 0.0,
            y: 0.0,
        }
    }

    /// Font coordinates have y growing upwards; pixel coordinates have it growing downwards.
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x * self.scale + self.offset_x,
            -y * self.scale + self.offset_y,
        )
    }

    fn line(&mut self, x: f32, y: f32) {
        self.segments.push(Segment {
            x0: self.x,
            y0: self.y,
            x1: x,
            y1: y,
        });
        self.x = x;
        self.y = y;
    }

    /// The number of straight pieces a curve needs, from how far its control points sit off the
    /// chord between its ends.
    fn steps(&self, points: &[(f32, f32)]) -> usize {
        let (first, last) = (points[0], points[points.len() - 1]);
        let mut deviation: f32 = 0.0;
        for &(x, y) in &points[1..points.len() - 1] {
            let dx = x - (first.0 + last.0) * 0.5;
            let dy = y - (first.1 + last.1) * 0.5;
            deviation = deviation.max((dx * dx + dy * dy).sqrt());
        }
        ((deviation / FLATTEN_TOLERANCE).sqrt().ceil() as usize).clamp(1, 32)
    }
}

impl OutlineBuilder for Flattener {
    fn move_to(&mut self, x: f32, y: f32) {
        // An unclosed contour is still a closed region as far as filling goes.
        self.close();
        let (x, y) = self.point(x, y);
        self.x = x;
        self.y = y;
        self.start_x = x;
        self.start_y = y;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.line(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = (self.x, self.y);
        let p1 = self.point(x1, y1);
        let p2 = self.point(x, y);
        let steps = self.steps(&[p0, p1, p2]);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let u = 1.0 - t;
            self.line(
                u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
                u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
            );
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = (self.x, self.y);
        let p1 = self.point(x1, y1);
        let p2 = self.point(x2, y2);
        let p3 = self.point(x, y);
        let steps = self.steps(&[p0, p1, p2, p3]);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let u = 1.0 - t;
            self.line(
                u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
                u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
            );
        }
    }

    fn close(&mut self) {
        if self.x != self.start_x || self.y != self.start_y {
            let (x, y) = (self.start_x, self.start_y);
            self.line(x, y);
        }
    }
}

/// Rasterizes one glyph of a face.
///
/// `offset_x` and `offset_y` shift the glyph within its own pixel grid, in pixels with y growing
/// downwards; they carry the sub-pixel part of a shaped glyph's position, so that a mark landing a
/// third of a pixel below a letter is drawn a third of a pixel lower rather than rounded onto it.
pub fn rasterize(face: &Face, glyph_id: GlyphId, offset_x: f32, offset_y: f32) -> RasterGlyph {
    let scale = maplibre_text_domain::EM_PX / face.units_per_em() as f32;
    let mut flattener = Flattener::new(scale, offset_x, offset_y);
    let bounding_box = face.outline_glyph(glyph_id, &mut flattener);
    flattener.close();
    let segments = flattener.segments;

    if bounding_box.is_none() || segments.is_empty() {
        return RasterGlyph::blank();
    }

    // The bounds of the flattened outline rather than of the reported bounding box: a font's box is
    // allowed to be conservative, and a too-wide box would pad the atlas with empty pixels.
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for segment in &segments {
        min_x = min_x.min(segment.x0).min(segment.x1);
        min_y = min_y.min(segment.y0).min(segment.y1);
        max_x = max_x.max(segment.x0).max(segment.x1);
        max_y = max_y.max(segment.y0).max(segment.y1);
    }

    // Rounded rather than floored and ceiled, which is what a glyph server's own rasterizer does:
    // a hairline rule that spans 5.50 to 7.37 pixels is one pixel tall there, not three.
    let left = min_x.round() as i32;
    let top = min_y.round() as i32;
    let width = (max_x.round() as i32 - left).max(0) as u32;
    let height = (max_y.round() as i32 - top).max(0) as u32;

    if width == 0 || height == 0 {
        return RasterGlyph::blank();
    }

    let bitmap = render(&segments, left, top, width, height);

    RasterGlyph {
        width,
        height,
        left,
        // `top` is measured up from the baseline, and pixel y grows down from it.
        bearing_y: -top,
        bitmap,
    }
}

/// Fills the padded bitmap with the encoded distance to the outline.
fn render(segments: &[Segment], left: i32, top: i32, width: u32, height: u32) -> Vec<u8> {
    let padded_width = width as i32 + 2 * GLYPH_BORDER;
    let padded_height = height as i32 + 2 * GLYPH_BORDER;
    let mut bitmap = vec![0u8; (padded_width * padded_height) as usize];

    let bounds: Vec<Bounds> = segments.iter().map(Bounds::of).collect();
    // Which segments could possibly be the nearest one to a pixel on each row. Without this every
    // pixel measures against every segment, and a glyph with a few hundred of them spends most of
    // its time on parts of the outline that are nowhere near.
    let rows = row_buckets(&bounds, top, padded_height);

    let mut crossings: Vec<(f32, i32)> = Vec::new();

    for row in 0..padded_height {
        let y = (top - GLYPH_BORDER + row) as f32 + 0.5;
        let nearby = &rows[row as usize];

        // Where the scanline crosses the outline, with the direction of each crossing, so that the
        // non-zero winding rule can say which stretches of the row are inside the glyph. Every
        // segment is considered here: a crossing far to the left still flips the winding.
        crossings.clear();
        for segment in segments {
            let (y0, y1) = (segment.y0, segment.y1);
            if (y0 > y) == (y1 > y) {
                continue;
            }
            let t = (y - y0) / (y1 - y0);
            crossings.push((
                segment.x0 + t * (segment.x1 - segment.x0),
                if y1 > y0 { 1 } else { -1 },
            ));
        }
        crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));

        let mut crossing_index = 0;
        let mut winding = 0;

        for column in 0..padded_width {
            let x = (left - GLYPH_BORDER + column) as f32 + 0.5;

            while crossing_index < crossings.len() && crossings[crossing_index].0 <= x {
                winding += crossings[crossing_index].1;
                crossing_index += 1;
            }
            let inside = winding != 0;

            let mut best = f32::MAX;
            for &index in nearby {
                // A segment whose box is already farther away than the best so far cannot improve
                // on it, and the box test is much cheaper than the distance.
                if bounds[index].distance_squared(x, y) >= best {
                    continue;
                }
                best = best.min(segments[index].distance_squared(x, y));
            }

            let distance = best.sqrt();
            let signed = if inside { -distance } else { distance };
            bitmap[(row * padded_width + column) as usize] = encode(signed);
        }
    }

    bitmap
}

/// A segment's bounding box, kept alongside the segments rather than inside them so that the
/// distance loop reads them one after another.
struct Bounds {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Bounds {
    fn of(segment: &Segment) -> Self {
        Self {
            x0: segment.x0.min(segment.x1),
            y0: segment.y0.min(segment.y1),
            x1: segment.x0.max(segment.x1),
            y1: segment.y0.max(segment.y1),
        }
    }

    fn distance_squared(&self, x: f32, y: f32) -> f32 {
        let dx = (self.x0 - x).max(0.0).max(x - self.x1);
        let dy = (self.y0 - y).max(0.0).max(y - self.y1);
        dx * dx + dy * dy
    }
}

/// For each row of the bitmap, the segments near enough to matter.
///
/// Anything further away than the field reaches is left out: past that the encoding saturates, so a
/// segment beyond it cannot change a single value.
fn row_buckets(bounds: &[Bounds], top: i32, padded_height: i32) -> Vec<Vec<usize>> {
    let reach = MAX_OUTER_DISTANCE + 1.0;
    let mut rows = vec![Vec::new(); padded_height as usize];

    for (index, box_) in bounds.iter().enumerate() {
        let from = ((box_.y0 - reach) - (top - GLYPH_BORDER) as f32).floor().max(0.0) as i32;
        let to = ((box_.y1 + reach) - (top - GLYPH_BORDER) as f32).ceil().min(padded_height as f32) as i32;
        for row in from..to {
            rows[row as usize].push(index);
        }
    }

    rows
}

/// Maps a signed distance in pixels, positive outside the outline, onto the shader's 0..255 range.
fn encode(distance: f32) -> u8 {
    if distance > MAX_OUTER_DISTANCE {
        return 0;
    }
    (255.0 - 255.0 * (distance / SDF_RADIUS + SDF_CUTOFF)).round().clamp(0.0, 255.0) as u8
}
