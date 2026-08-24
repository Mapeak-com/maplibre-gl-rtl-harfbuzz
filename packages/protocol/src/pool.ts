/**
 * The stretch of Unicode set aside for shaped glyphs, and how it is divided between workers.
 *
 * These numbers mirror `crates/text-domain/src/constants.rs`; the WebAssembly module reports its own
 * through `codepointPool()`, and the drawing half checks the two agree at startup.
 */

/** The first codepoint of plane 15, where the supplementary private use areas begin. */
export const POOL_START = 0xf0000;

/** The last codepoint of plane 16 that a string may contain. */
export const POOL_END = 0x10fffd;

/** Glyphs are fetched in blocks of this many codepoints: the `{range}` of a `glyphs` URL. */
export const RANGE_SIZE = 256;

/**
 * How many workers the pool is divided between.
 *
 * MapLibre runs one worker outside Safari and up to three inside it, and every map on the page
 * shares that pool, so this has room to spare. Each share is large enough that a map would have to
 * put tens of thousands of distinct shaped glyphs on screen to use one up.
 */
export const MAX_WORKERS = 8;

/** How many codepoints each worker gets. */
export const CODEPOINTS_PER_WORKER = Math.floor((POOL_END - POOL_START + 1) / MAX_WORKERS);

/** Whether a codepoint stands for a shaped glyph rather than for a character. */
export function isPoolCodepoint(codepoint: number): boolean {
    return codepoint >= POOL_START && codepoint <= POOL_END;
}

/** Whether a block of codepoints holds shaped glyphs rather than characters. */
export function isPoolRange(range: number): boolean {
    return isPoolCodepoint(range * RANGE_SIZE);
}

/**
 * The stretch of the pool a worker allocates from.
 *
 * Giving each worker its own stretch is what lets them allocate without asking each other anything:
 * shaping happens inside a synchronous call MapLibre makes, so there is no moment at which a worker
 * could wait for an answer. Two workers may end up drawing the same glyph twice under two
 * codepoints, which costs a little room in the glyph atlas and nothing else.
 */
export function codepointRangeForWorker(
    index: number,
): {codepointStart: number; codepointEnd: number} {
    const codepointStart = POOL_START + index * CODEPOINTS_PER_WORKER;
    return {
        codepointStart,
        codepointEnd: Math.min(codepointStart + CODEPOINTS_PER_WORKER - 1, POOL_END),
    };
}
