/**
 * The entry point MapLibre loads into its workers, built as `text-plugin.mjs`.
 *
 * MapLibre fetches this URL inside each of its workers, waits up to five seconds for
 * `registerRTLTextPlugin` to be called, and only then lets tiles with text be built. Everything
 * this plugin needs -- the WebAssembly module, the font files, its share of the codepoint pool --
 * is fetched in that window, so that no label is ever shaped by a half-ready plugin.
 */

import {startShaping} from '@maplibre-rtl-harfbuzz/shaping-worker';

declare const self: {registerRTLTextPlugin?: (plugin: unknown) => void};

const plugin = await startShaping();

if (typeof self.registerRTLTextPlugin !== 'function') {
    throw new Error(
        'maplibre-gl-rtl-harfbuzz: this file is MapLibre\'s worker half of the plugin. Pass its URL ' +
            'to `registerHarfBuzzTextPlugin` rather than importing it yourself.',
    );
}

self.registerRTLTextPlugin(plugin);
