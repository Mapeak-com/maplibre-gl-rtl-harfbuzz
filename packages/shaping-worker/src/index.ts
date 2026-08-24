/**
 * The shaping half of the plugin, which runs inside MapLibre's web workers.
 *
 * MapLibre's text plugin interface is three functions that take a string and return strings. That
 * is a narrow opening for something as wide as text shaping, and the way through it is this: the
 * strings that come back are still strings, but each codepoint in them stands for a glyph rather
 * than for a character. MapLibre carries them through its pipeline exactly as before -- collecting
 * them as the glyphs a tile needs, measuring them for line breaking, laying them out -- and when it
 * asks what they look like, the other half of the plugin draws the glyphs they stand for.
 */

export {startShaping} from './plugin.ts';
export type {ShapingOptions, RTLTextPlugin} from './plugin.ts';
