//! Where shaped glyphs are given codepoints, and where those codepoints are turned back into
//! pictures.
//!
//! This crate holds the bookkeeping and none of the craft: it knows nothing about HarfBuzz, about
//! fonts, or about distance fields, and reaches both through the traits `maplibre-text-domain`
//! declares.

#![forbid(unsafe_code)]

mod allocator;
mod registry;

pub use allocator::CodepointAllocator;
pub use registry::GlyphRegistry;
