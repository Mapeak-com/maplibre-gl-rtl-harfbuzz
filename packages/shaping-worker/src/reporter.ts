/**
 * Telling the drawing half what the codepoints this worker invented stand for.
 *
 * Reporting is batched rather than done per label: MapLibre shapes every label of a tile within one
 * task, so a report scheduled as a task of its own carries a whole tile's new glyphs in one message.
 *
 * Batching is only an optimization, never a correctness question. Before a block of codepoints is
 * drawn the page asks for it to be sealed, and the answer to that carries everything not yet
 * reported -- so a glyph allocated a moment ago is never missed, however the batching fell.
 */

import type {Shaper} from '@maplibre-rtl-harfbuzz/wasm';
import {
    GLOBAL_DISPATCHER_ID,
    REPORT_GLYPHS,
    SEAL_RANGE,
    type ReportGlyphs,
    type Sealed,
    type SealRange,
    type WorkerActor,
} from '@maplibre-rtl-harfbuzz/protocol';

export class GlyphReporter {
    private readonly shaper: Shaper;
    private readonly actor: WorkerActor;
    private scheduled = false;

    constructor(shaper: Shaper, actor: WorkerActor) {
        this.shaper = shaper;
        this.actor = actor;
    }

    /** Answers the page when it closes a block of codepoints, and starts listening for that. */
    listen(): void {
        this.actor.registerMessageHandler(SEAL_RANGE, async (_mapId, {range}: SealRange) => {
            this.shaper.sealRange(range);
            return {entries: this.shaper.takeNewGlyphs()} satisfies Sealed;
        });
    }

    /** Reports whatever has been allocated, once the current task is over. */
    schedule(): void {
        if (this.scheduled) return;
        this.scheduled = true;
        // A macrotask rather than a microtask: microtasks run before MapLibre has finished the tile,
        // which would send one message per label instead of one per tile.
        setTimeout(() => {
            this.scheduled = false;
            const entries = this.shaper.takeNewGlyphs();
            if (!entries.length) return;
            void this.actor.sendAsync({
                type: REPORT_GLYPHS,
                data: {entries} satisfies ReportGlyphs,
                targetMapId: GLOBAL_DISPATCHER_ID,
            });
        }, 0);
    }
}
