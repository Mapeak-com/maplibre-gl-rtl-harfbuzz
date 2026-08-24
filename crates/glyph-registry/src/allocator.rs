//! Hands out the codepoints that stand for shaped glyphs.
//!
//! MapLibre asks for glyphs in blocks of 256 codepoints, and it asks for each block once: a glyph
//! added to a block that has already been drawn and handed over would be asked for never. So a
//! block can be *sealed*, after which nothing new is allocated into it, and whoever draws the
//! blocks seals one before drawing it.

use maplibre_text_domain::{NONCHARACTERS, POOL_END, POOL_START, RANGE_SIZE};
use std::collections::HashSet;

pub struct CodepointAllocator {
    next: u32,
    end: u32,
    sealed: HashSet<u32>,
}

impl Default for CodepointAllocator {
    fn default() -> Self {
        Self {
            next: POOL_START,
            end: POOL_END,
            sealed: HashSet::new(),
        }
    }
}

impl CodepointAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Narrows this allocator to one stretch of the pool.
    ///
    /// A map runs more than one worker, each shaping its own tiles, and none of them can ask the
    /// others what a codepoint means without stopping to wait. Giving each its own stretch means
    /// they never have to: two workers may end up drawing the same glyph twice under two
    /// codepoints, which costs a little room in the glyph atlas and nothing else.
    pub fn restrict(&mut self, start: u32, end: u32) {
        self.next = start.max(POOL_START);
        self.end = end.min(POOL_END);
    }

    /// Stops anything new from being allocated into a block.
    pub fn seal(&mut self, range: u32) {
        self.sealed.insert(range);
        if self.next / RANGE_SIZE == range {
            self.next = (range + 1) * RANGE_SIZE;
        }
    }

    /// The next codepoint, or `None` once the stretch is used up.
    pub fn next(&mut self) -> Option<u32> {
        while self.next <= self.end {
            let codepoint = self.next;
            if self.sealed.contains(&(codepoint / RANGE_SIZE)) {
                self.next = (codepoint / RANGE_SIZE + 1) * RANGE_SIZE;
                continue;
            }
            self.next += 1;
            if NONCHARACTERS.contains(&codepoint) {
                continue;
            }
            return Some(codepoint);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_out_the_pool_in_order() {
        let mut allocator = CodepointAllocator::new();
        assert_eq!(allocator.next(), Some(POOL_START));
        assert_eq!(allocator.next(), Some(POOL_START + 1));
    }

    #[test]
    fn skips_a_sealed_block() {
        let mut allocator = CodepointAllocator::new();
        allocator.seal(POOL_START / RANGE_SIZE);
        assert_eq!(allocator.next(), Some(POOL_START + RANGE_SIZE));
    }

    #[test]
    fn skips_noncharacters() {
        let mut allocator = CodepointAllocator::new();
        allocator.restrict(NONCHARACTERS[0] - 1, NONCHARACTERS[1] + 1);
        assert_eq!(allocator.next(), Some(NONCHARACTERS[0] - 1));
        assert_eq!(allocator.next(), Some(NONCHARACTERS[1] + 1));
        assert_eq!(allocator.next(), None);
    }

    #[test]
    fn runs_out_at_the_end_of_its_stretch() {
        let mut allocator = CodepointAllocator::new();
        allocator.restrict(POOL_START, POOL_START);
        assert_eq!(allocator.next(), Some(POOL_START));
        assert_eq!(allocator.next(), None);
    }
}
