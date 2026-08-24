//! Shaping text with HarfBuzz, and putting shaped text back into reading order.
//!
//! Two passes, because that is the shape of the problem and, as it happens, also the shape of the
//! interface MapLibre offers a text plugin:
//!
//! 1. [`shape`](shape::shape) resolves the direction of every character, splits the text into runs
//!    of one direction, one script and one font, and shapes each run. What comes back is glyphs,
//!    in logical order.
//! 2. [`reorder`](reorder::reorder) runs once MapLibre has decided where the lines break, and puts
//!    each line into the order it is read in. It has to be a second pass because the Unicode
//!    Bidirectional Algorithm resolves the order of a *line*: the same words come out in a
//!    different order depending on where the line ends.

#![forbid(unsafe_code)]

mod features;
mod reorder;
mod shape;

use std::{cell::RefCell, rc::Rc};

use maplibre_font_set::FontSet;
use maplibre_text_domain::{Piece, TextShaping, VisualLine};

/// Shapes text with HarfBuzz, through the `rustybuzz` port of it.
pub struct HarfBuzzShaping {
    fonts: Rc<RefCell<FontSet>>,
}

impl HarfBuzzShaping {
    pub fn new(fonts: Rc<RefCell<FontSet>>) -> Self {
        Self { fonts }
    }
}

impl TextShaping for HarfBuzzShaping {
    fn shape(&self, text: &str) -> Vec<Piece> {
        shape::shape(&self.fonts.borrow(), text)
    }

    fn reorder(
        &self,
        text: &str,
        direction_of: &dyn Fn(u32) -> Option<bool>,
        sections: &[u32],
        line_breaks: &[u32],
    ) -> Vec<VisualLine> {
        reorder::reorder(text, direction_of, sections, line_breaks)
    }
}
