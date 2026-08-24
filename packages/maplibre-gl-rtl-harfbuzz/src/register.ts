/**
 * Wiring the two halves of the plugin into a map.
 */

import {GlyphProvider, type GlyphProviderOptions} from '@maplibre-rtl-harfbuzz/glyph-provider';

/** The parts of MapLibre's entry point this needs, so that the package does not depend on it. */
export type MapLibreLike = {
    addProtocol: (protocol: string, load: (parameters: {url: string}) => Promise<{data: ArrayBuffer}>) => void;
    removeProtocol: (protocol: string) => void;
    setRTLTextPlugin: (url: string, deferred?: boolean) => Promise<void>;
    getRTLTextPluginStatus: () => string;
    /** The channel the two halves of the plugin reach each other over. */
    getGlobalDispatcher: () => unknown;
};

export type HarfBuzzTextPluginOptions = Omit<GlyphProviderOptions, 'dispatcher'> & {
    /**
     * Where the worker half of the plugin is, if not next to this module.
     *
     * It has to keep its `.mjs` extension. MapLibre loads a plugin whose URL ends in `.mjs` with a
     * dynamic import, and anything else by fetching the source and running it, which would lose the
     * module context this half needs.
     */
    pluginUrl?: string | URL;
};

/**
 * Starts the glyph provider, registers it as a protocol, and hands MapLibre the shaping half.
 *
 * The plugin is loaded eagerly rather than deferred. MapLibre only fetches a deferred text plugin
 * once it meets right-to-left text, and complex scripts that are not right to left -- Devanagari,
 * Tamil, Khmer -- would never trigger it.
 */
export async function registerHarfBuzzTextPlugin(
    maplibregl: MapLibreLike,
    options: HarfBuzzTextPluginOptions,
): Promise<GlyphProvider> {
    if (maplibregl.getRTLTextPluginStatus() !== 'unavailable') {
        throw new Error(
            'maplibre-gl-rtl-harfbuzz: a text plugin has already been set on this page. MapLibre ' +
                'allows only one, and it cannot be replaced once set.',
        );
    }

    const provider = await GlyphProvider.create({
        ...options,
        dispatcher: maplibregl.getGlobalDispatcher() as never,
    });
    maplibregl.addProtocol(provider.protocol, provider.handleRequest);

    try {
        await maplibregl.setRTLTextPlugin(resolvePluginUrl(options.pluginUrl), false);
    } catch (error) {
        maplibregl.removeProtocol(provider.protocol);
        provider.destroy();
        throw error;
    }

    return provider;
}

function resolvePluginUrl(pluginUrl: HarfBuzzTextPluginOptions['pluginUrl']): string {
    const url = new URL(pluginUrl ?? 'text-plugin.mjs', import.meta.url).href;
    if (!new URL(url).pathname.endsWith('.mjs')) {
        throw new Error(
            `maplibre-gl-rtl-harfbuzz: the worker half must be served with a .mjs extension, but ${url} is not. ` +
                'MapLibre decides how to load a text plugin from its extension.',
        );
    }
    return url;
}
