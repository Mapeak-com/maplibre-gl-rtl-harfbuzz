/**
 * What the shaping half and the drawing half say to each other.
 *
 * Shaping runs in MapLibre's workers, because that is where the text plugin interface lives.
 * Drawing runs on the main thread, because that is where glyph requests are answered. Three
 * exchanges connect them, and only three:
 *
 * 1. **Joining.** A worker announces itself and is given the fonts to shape with and its own stretch
 *    of codepoints to allocate from.
 * 2. **Reporting.** A worker says what codepoints it has allocated and what they stand for.
 * 3. **Sealing.** Before a block of codepoints is drawn, the workers are asked to stop allocating
 *    into it and to hand over anything they have not reported yet. This is what makes the whole
 *    scheme sound: MapLibre asks for a block of glyphs once and never again, so a block must be
 *    complete before it is answered, and nothing may be added to it afterwards.
 *
 * Each is a request with an answer, which is what MapLibre's own worker protocol offers -- so the
 * sealing exchange needs no bookkeeping of its own beyond awaiting the answers.
 */

/** A worker asking to be let in. Answered with {@link Welcome}. */
export const JOIN = 'maplibre-gl-rtl-harfbuzz/join';

/** A worker saying what it has allocated. Answered with nothing. */
export const REPORT_GLYPHS = 'maplibre-gl-rtl-harfbuzz/glyphs';

/** The page closing a block of codepoints. Answered with {@link Sealed}. */
export const SEAL_RANGE = 'maplibre-gl-rtl-harfbuzz/seal';

export type Join = {
    /** Chosen by the worker, and stable for its lifetime, so that a second join gets a second answer. */
    worker: string;
};

/** Everything a worker needs before it can shape anything. */
export type Welcome = {
    /** Where to fetch the WebAssembly module from, absolute. */
    wasmUrl: string;
    /**
     * The font files, in fallback order, each with the instance of it to read. Sent rather than
     * fetched again so that both halves are certain to be reading the very same bytes at the very
     * same instance -- a glyph index means nothing otherwise.
     */
    fonts: Array<{bytes: ArrayBuffer; weight: number; width: number}>;
    /** The stretch of the codepoint pool this worker may allocate from, inclusive. */
    codepointStart: number;
    codepointEnd: number;
};

/** Codepoints a worker has allocated, in the flat encoding the WebAssembly module uses. */
export type ReportGlyphs = {
    entries: Int32Array;
};

export type SealRange = {
    /** The block, as a codepoint divided by `RANGE_SIZE`. */
    range: number;
};

/** A worker confirming it will not allocate into that block again, with anything left to report. */
export type Sealed = {
    entries: Int32Array;
};

/** How many 32 bit integers one glyph entry takes: codepoint, font, glyph, dx, dy, advance, rtl. */
export const GLYPH_ENTRY_WIDTH = 7;
