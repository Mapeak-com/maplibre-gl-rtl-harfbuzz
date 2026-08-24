/**
 * The `glyphs` URL a style points at the plugin with, and how to read one back.
 *
 * MapLibre fills in `{fontstack}` and `{range}` and asks for the result through whatever protocol
 * the URL names, which is how the plugin gets to answer glyph requests without a server.
 */

import {RANGE_SIZE} from './pool.ts';

/** The protocol the plugin registers itself under unless told otherwise. */
export const DEFAULT_PROTOCOL = 'harfbuzz';

/** The value a style's `glyphs` property needs, for a plugin registered under `protocol`. */
export function glyphsUrl(protocol: string = DEFAULT_PROTOCOL): string {
    return `${protocol}://{fontstack}/{range}.pbf`;
}

export type GlyphRequest = {
    /** The `text-font` stack, as the style wrote it. */
    fontstack: string;
    /** The block asked for, as a codepoint divided by `RANGE_SIZE`. */
    range: number;
};

/**
 * Reads back a URL that {@link glyphsUrl} produced and MapLibre filled in.
 *
 * Returns `null` for anything that is not one, so that a handler can decline rather than guess.
 */
export function parseGlyphsUrl(url: string): GlyphRequest | null {
    const afterProtocol = url.slice(url.indexOf('://') + 3);
    const lastSlash = afterProtocol.lastIndexOf('/');
    if (lastSlash < 0) return null;

    const fontstack = decodeURIComponent(afterProtocol.slice(0, lastSlash));
    const start = /^(\d+)-\d+(?:\.pbf)?$/.exec(afterProtocol.slice(lastSlash + 1))?.[1];
    if (start === undefined) return null;

    return {fontstack, range: Number(start) / RANGE_SIZE};
}
