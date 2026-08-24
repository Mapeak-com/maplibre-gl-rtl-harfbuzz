/**
 * Complex-script and right-to-left text for MapLibre GL JS.
 *
 * ```ts
 * import maplibregl from 'maplibre-gl';
 * import {registerHarfBuzzTextPlugin, glyphsUrl} from 'maplibre-gl-rtl-harfbuzz';
 *
 * await registerHarfBuzzTextPlugin(maplibregl, {
 *     fonts: ['/fonts/NotoSans-Regular.ttf', '/fonts/NotoSansHebrew-Regular.ttf'],
 * });
 *
 * const map = new maplibregl.Map({
 *     container: 'map',
 *     style: {...style, glyphs: glyphsUrl()},
 * });
 * ```
 *
 * The plugin draws every glyph the style asks for, out of the font files given here, so the style's
 * `glyphs` has to point at it rather than at a glyph server. That is not a limitation of the
 * approach so much as the point of it: shaping and drawing have to agree on the font, because a
 * glyph index means nothing without one.
 */

export {registerHarfBuzzTextPlugin, type HarfBuzzTextPluginOptions} from './register.ts';
export {glyphsUrl, DEFAULT_PROTOCOL} from '@maplibre-rtl-harfbuzz/protocol';
export type {FontSource, FontInstance} from '@maplibre-rtl-harfbuzz/glyph-provider';
export {GlyphProvider} from '@maplibre-rtl-harfbuzz/glyph-provider';
