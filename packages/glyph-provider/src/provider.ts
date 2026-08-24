/**
 * Answers MapLibre's glyph requests, and tells the shaping workers what to shape with.
 */

import init, {Shaper, codepointPool} from '@maplibre-rtl-harfbuzz/wasm';
import {
    DEFAULT_PROTOCOL,
    glyphsUrl,
    isPoolRange,
    parseGlyphsUrl,
    POOL_END,
    POOL_START,
    type MainThreadDispatcher,
} from '@maplibre-rtl-harfbuzz/protocol';

import {loadFont, type FontSource, type LoadedFont} from './fonts.ts';
import {WorkerRegistry} from './worker-registry.ts';

export type GlyphProviderOptions = {
    /**
     * The font files to shape and draw with, in fallback order: the first file with a glyph for a
     * character is the one that character is drawn with.
     *
     * URLs are fetched; bytes are used as they are. Any format `ttf-parser` reads will do --
     * TrueType, OpenType, variable fonts -- but not WOFF or WOFF2, which are compressed.
     */
    fonts: FontSource[];
    /** Where the WebAssembly module is, if not next to this one. */
    wasmUrl?: string | URL;
    /** The protocol to answer glyph requests under. A style's `glyphs` must name the same one. */
    protocol?: string;
    /**
     * MapLibre's `getGlobalDispatcher()`, which is how the two halves of the plugin reach each
     * other: the page registers handlers on it, and each worker answers through its own actor.
     */
    dispatcher: MainThreadDispatcher;
};

/** What MapLibre passes a protocol handler, of which only the URL matters here. */
type RequestParameters = {url: string};

export class GlyphProvider {
    /** The value a style's `glyphs` property needs. */
    readonly glyphsUrl: string;
    readonly protocol: string;

    private readonly shaper: Shaper;
    private readonly fonts: LoadedFont[];
    private readonly registry: WorkerRegistry;
    private inspectionShaper: Shaper | null = null;

    private constructor(shaper: Shaper, options: GlyphProviderOptions, fonts: LoadedFont[], wasmUrl: string) {
        this.shaper = shaper;
        this.fonts = fonts;
        this.protocol = options.protocol ?? DEFAULT_PROTOCOL;
        this.glyphsUrl = glyphsUrl(this.protocol);

        this.registry = new WorkerRegistry(options.dispatcher, {
            register: (entries) => this.shaper.registerGlyphs(entries),
            welcome: () => ({wasmUrl, fonts: fonts.map(({bytes, weight, width}) => ({bytes, weight, width}))}),
        });
    }

    /**
     * Loads the WebAssembly module and the fonts, and starts listening for shaping workers.
     *
     * The workers are told what to shape with rather than fetching it themselves, so that both
     * halves are certain to be working from the same font files -- a glyph index means nothing
     * unless both halves resolved it against the same file. Listening starts before this returns,
     * so that a worker loading the plugin can never ask before anything is there to answer.
     */
    static async create(options: GlyphProviderOptions): Promise<GlyphProvider> {
        const wasmUrl = new URL(options.wasmUrl ?? 'shaper_bg.wasm', import.meta.url).href;
        await init({module_or_path: wasmUrl});

        const [pool, fonts] = [codepointPool(), await Promise.all(options.fonts.map(loadFont))];
        if (pool[0] !== POOL_START || pool[1] !== POOL_END) {
            throw new Error(
                'maplibre-gl-rtl-harfbuzz: the WebAssembly module and this package disagree about ' +
                    'the codepoint pool, which means they were built from different sources',
            );
        }

        const shaper = new Shaper();
        fonts.forEach((font, index) => {
            if (shaper.addFont(new Uint8Array(font.bytes), font.weight, font.width) < 0) {
                throw new Error(
                    `maplibre-gl-rtl-harfbuzz: could not read the font at position ${index}. ` +
                        'WOFF and WOFF2 are compressed and cannot be read; use the TrueType or OpenType file.',
                );
            }
        });

        const provider = new GlyphProvider(shaper, options, fonts, wasmUrl);
        await provider.registry.listen();
        return provider;
    }

    /**
     * Draws one block of glyphs. This is what gets registered with `addProtocol`.
     */
    handleRequest = async (parameters: RequestParameters): Promise<{data: ArrayBuffer}> => {
        const request = parseGlyphsUrl(parameters.url);
        if (!request || !Number.isInteger(request.range)) {
            throw new Error(`maplibre-gl-rtl-harfbuzz: ${parameters.url} is not a glyph range URL`);
        }

        // A block of shaped glyphs has to be closed to further allocation before it is answered,
        // because MapLibre will not ask for it a second time.
        if (isPoolRange(request.range)) {
            await this.registry.seal(request.range);
        }

        const data = this.shaper.glyphPbf(request.fontstack, request.range);
        return {data: data.buffer as ArrayBuffer};
    };

    /**
     * A shaper for tools that want to look at what shaping produced.
     *
     * It is a second one, built on demand from the same font files, because looking at text the way
     * the map lays it out means running the same path the map runs -- shaping *and* reordering --
     * and that allocates codepoints. Those allocations would otherwise land in the same registry
     * that answers MapLibre's glyph requests, where they would mean something different from what
     * the workers meant by them.
     */
    get inspector(): Shaper {
        if (!this.inspectionShaper) {
            this.inspectionShaper = new Shaper();
            for (const font of this.fonts) {
                this.inspectionShaper.addFont(new Uint8Array(font.bytes), font.weight, font.width);
            }
        }
        return this.inspectionShaper;
    }

    destroy(): void {
        this.shaper.free();
        this.inspectionShaper?.free();
        this.inspectionShaper = null;
    }
}
