/**
 * Loading the built plugin the way a page would.
 *
 * The import is deliberately a runtime one against a URL, and deliberately hidden from the bundler:
 * both halves of the plugin work out where the WebAssembly module and the worker script are by
 * resolving them against their own URL, so bundling them into the page would break exactly the
 * thing this page exists to exercise.
 */

/** The font files, in fallback order. Latin first, so every other script falls through to its own. */
export const FONTS = [
    '/fonts/NotoSans.ttf',
    '/fonts/NotoSansHebrew.ttf',
    '/fonts/NotoSansArabic.ttf',
    '/fonts/NotoSansDevanagari.ttf',
    '/fonts/NotoSansBengali.ttf',
    '/fonts/NotoSansTamil.ttf',
    '/fonts/NotoSansThai.ttf',
    '/fonts/NotoSansKhmer.ttf',
];

/** What the plugin's shaper offers a tool that wants to look at shaping rather than draw a map. */
export type Inspector = {
    /** The glyphs shaping produces, in the order they were written. */
    inspect(text: string): Int32Array;
    /** The same glyphs in the order they are drawn, which for right-to-left text is not the same. */
    inspectVisual(text: string): Int32Array;
    /** The glyphs MapLibre would draw with no plugin at all, for comparison. */
    inspectUnshaped(text: string): Int32Array;
    glyphImage(font: number, glyph: number, dx: number, dy: number): Uint8Array;
};

export type Plugin = {
    glyphsUrl(protocol?: string): string;
    registerHarfBuzzTextPlugin(
        maplibregl: unknown,
        options: {fonts: string[]},
    ): Promise<{inspector: Inspector; glyphsUrl: string}>;
};

export async function loadPlugin(): Promise<Plugin> {
    const url = new URL('/plugin/index.mjs', location.href).href;
    return (await import(/* @vite-ignore */ url)) as Plugin;
}
