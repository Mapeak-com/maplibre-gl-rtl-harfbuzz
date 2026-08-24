//! Choosing an instance of a variable font.
//!
//! A variable font file is a whole family: one set of outlines plus the deltas that carry it from
//! one weight or width to another. The outlines in the file are the *default* instance, and for
//! several of Google's Noto families that default is the thin master, not the regular one. Read
//! without asking for anything, such a file draws hairlines -- correctly, and not at all what
//! anyone meant by it.
//!
//! So an instance is always asked for, defaulting to the one a browser would pick for
//! `font-weight: normal; font-stretch: normal`, and each axis is clamped to what the file offers so
//! that asking for a weight a family does not have gives its nearest instead of nothing.

use rustybuzz::{Face, Variation};
use ttf_parser::Tag;

/// The instance to read a variable font at. Ignored by a font that is not variable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontInstance {
    /// The `wght` axis, on the usual 100 (thin) to 900 (black) scale.
    pub weight: f32,
    /// The `wdth` axis, as a percentage of normal.
    pub width: f32,
}

impl Default for FontInstance {
    fn default() -> Self {
        // What `font-weight: normal` and `font-stretch: normal` mean in CSS.
        Self {
            weight: 400.0,
            width: 100.0,
        }
    }
}

const WEIGHT: Tag = Tag::from_bytes(b"wght");
const WIDTH: Tag = Tag::from_bytes(b"wdth");

/// Sets a face to an instance, doing nothing to a font that has no such axes.
pub fn apply(face: &mut Face<'_>, instance: FontInstance) {
    let wanted = [(WEIGHT, instance.weight), (WIDTH, instance.width)];

    let variations: Vec<Variation> = face
        .variation_axes()
        .into_iter()
        .filter_map(|axis| {
            let (tag, value) = wanted.iter().find(|(tag, _)| *tag == axis.tag)?;
            Some(Variation {
                tag: *tag,
                value: value.clamp(axis.min_value, axis.max_value),
            })
        })
        .collect();

    if !variations.is_empty() {
        face.set_variations(&variations);
    }
}
