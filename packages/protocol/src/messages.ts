/**
 * What the shaping half and the drawing half say to each other.
 *
 * Three exchanges, and only three:
 *
 * 1. **Joining.** A worker announces itself and is given the fonts to shape with and its own
 *    stretch of codepoints to allocate from.
 * 2. **Reporting.** A worker says what codepoints it has allocated and what they stand for.
 * 3. **Sealing.** Before a block of codepoints is drawn, the drawing half asks the workers to stop
 *    allocating into it and to hand over anything they have not reported yet. This is what makes
 *    the whole scheme sound: MapLibre asks for a block of glyphs once and never again, so a block
 *    must be complete before it is answered, and after it is answered nothing may be added to it.
 */

/** The default name of the channel the two halves meet on. */
export const CHANNEL_NAME = 'maplibre-gl-rtl-harfbuzz';

/** A worker announcing itself. Repeated until it is answered, in case the other half is not up yet. */
export type HelloMessage = {
    type: 'hello';
    /** Chosen by the worker, and stable for its lifetime. */
    worker: string;
};

/** The answer: everything a worker needs before it can shape anything. */
export type WelcomeMessage = {
    type: 'welcome';
    worker: string;
    /** Where to fetch the WebAssembly module from, absolute. */
    wasmUrl: string;
    /**
     * The font files, in fallback order, each with the instance of it to read. Already fetched, so
     * that no worker fetches them again and, more importantly, so that both halves are certain to
     * be reading the very same bytes at the very same instance -- a glyph index means nothing
     * otherwise.
     */
    fonts: Array<{bytes: ArrayBuffer; weight: number; width: number}>;
    /** The stretch of the codepoint pool this worker may allocate from, inclusive. */
    codepointStart: number;
    codepointEnd: number;
};

/** Codepoints a worker has allocated, in the flat encoding the WebAssembly module uses. */
export type GlyphsMessage = {
    type: 'glyphs';
    worker: string;
    entries: Int32Array;
};

/** A block of codepoints is about to be drawn. */
export type SealMessage = {
    type: 'seal';
    /** Matches the reply to the request. */
    request: number;
    /** The block, as a codepoint divided by `RANGE_SIZE`. */
    range: number;
};

/** A worker confirming it will not allocate into that block again, with anything left to report. */
export type SealedMessage = {
    type: 'sealed';
    worker: string;
    request: number;
    entries: Int32Array;
};

export type WorkerMessage = HelloMessage | GlyphsMessage | SealedMessage;
export type ProviderMessage = WelcomeMessage | SealMessage;
export type ChannelMessage = WorkerMessage | ProviderMessage;

/** How many 32 bit integers one glyph entry takes: codepoint, font, glyph, dx, dy, advance, rtl. */
export const GLYPH_ENTRY_WIDTH = 7;

/**
 * How long the drawing half waits for the workers to seal a block.
 *
 * A worker answers between tasks, so the wait is however long it has left of the tile it is parsing
 * -- tens of milliseconds, normally. The limit is only here so that a worker that has gone away
 * without saying so cannot stall every glyph request behind it.
 */
export const SEAL_TIMEOUT_MS = 5000;
