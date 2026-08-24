/**
 * The drawing half of the plugin, which runs on the main thread.
 *
 * It answers MapLibre's glyph requests out of the font files themselves, rasterizing each glyph into
 * the same signed distance field a glyph server would have sent -- so a style needs no glyph server
 * at all. For the codepoints the shaping half invented, it draws the glyphs those stand for, with
 * the offsets the font asked for baked into the metrics MapLibre reads.
 */

export {GlyphProvider} from './provider.ts';
export type {GlyphProviderOptions} from './provider.ts';
export {WorkerRegistry} from './worker-registry.ts';
export type {FontSource, FontInstance, LoadedFont} from './fonts.ts';
