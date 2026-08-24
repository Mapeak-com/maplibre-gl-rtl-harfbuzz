/**
 * Telling the drawing half what the codepoints this worker invented stand for.
 *
 * Reporting is batched rather than done per label: MapLibre shapes every label of a tile within one
 * task, so a report scheduled as a task of its own carries a whole tile's new glyphs in one message.
 *
 * Batching is only an optimization, never a correctness question. Before a block of codepoints is
 * drawn the drawing half asks for it to be sealed, and the answer to that carries everything not
 * yet reported -- so a glyph allocated a moment ago is never missed, however the batching fell.
 */

import type {Shaper} from '@maplibre-rtl-harfbuzz/wasm';
import type {ChannelMessage} from '@maplibre-rtl-harfbuzz/protocol';

export class GlyphReporter {
    private readonly shaper: Shaper;
    private readonly channel: {postMessage(message: ChannelMessage): void};
    private readonly worker: string;
    private scheduled = false;

    constructor(shaper: Shaper, channel: {postMessage(message: ChannelMessage): void}, worker: string) {
        this.shaper = shaper;
        this.channel = channel;
        this.worker = worker;
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
            if (entries.length) {
                this.channel.postMessage({type: 'glyphs', worker: this.worker, entries});
            }
        }, 0);
    }

    /**
     * Closes a block of codepoints to further allocation and hands over everything allocated in it
     * so far, so that the block can be drawn complete.
     */
    seal(range: number, request: number): void {
        this.shaper.sealRange(range);
        this.channel.postMessage({
            type: 'sealed',
            worker: this.worker,
            request,
            entries: this.shaper.takeNewGlyphs(),
        });
    }
}
