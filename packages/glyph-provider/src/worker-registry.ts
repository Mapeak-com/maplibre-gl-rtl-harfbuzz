/**
 * Keeps track of the shaping workers, and runs the exchange that makes a block of codepoints safe
 * to draw.
 *
 * MapLibre asks for a block of glyphs once and remembers that it has: a glyph that appears in a
 * block after it has been answered would be asked for never, and would silently not draw. So before
 * a block is drawn, every worker is asked to seal it -- to stop allocating into it, and to hand over
 * anything it has allocated there but not yet reported. Only once they have all answered is the
 * block complete, and it stays complete because nothing may be added to a sealed block.
 */

import {
    codepointRangeForWorker,
    SEAL_TIMEOUT_MS,
    type ChannelMessage,
    type SealedMessage,
    type WelcomeMessage,
} from '@maplibre-rtl-harfbuzz/protocol';

/** What the registry needs from the outside, so that it can be driven in a test without a channel. */
export type RegistryPort = {
    post(message: ChannelMessage): void;
    /** Records what a worker's codepoints stand for. */
    register(entries: Int32Array): void;
    /** What a joining worker is given to shape with. */
    welcome(): Pick<WelcomeMessage, 'wasmUrl' | 'fonts'>;
    warn(message: string): void;
};

type PendingSeal = {
    outstanding: Set<string>;
    resolve: () => void;
    timer: ReturnType<typeof setTimeout>;
};

export class WorkerRegistry {
    private readonly port: RegistryPort;
    /** Which stretch of the pool each worker was given, so a repeated hello gets the same answer. */
    private readonly slices = new Map<string, number>();
    private readonly pending = new Map<number, PendingSeal>();
    private nextRequest = 0;

    constructor(port: RegistryPort) {
        this.port = port;
    }

    get workerCount(): number {
        return this.slices.size;
    }

    /** Handles anything a worker says. Messages from the drawing half itself are ignored. */
    receive(message: ChannelMessage): void {
        switch (message.type) {
            case 'hello':
                this.port.post({
                    type: 'welcome',
                    worker: message.worker,
                    ...this.port.welcome(),
                    ...codepointRangeForWorker(this.sliceFor(message.worker)),
                });
                break;
            case 'glyphs':
                this.port.register(message.entries);
                break;
            case 'sealed':
                this.sealed(message);
                break;
        }
    }

    /**
     * Asks every worker to seal a block and waits for them to answer.
     *
     * A worker that never answers is given up on rather than allowed to stall every glyph request
     * behind it; the block is then drawn with what is known, which can leave a glyph missing but
     * cannot leave the map hanging.
     */
    async seal(range: number): Promise<void> {
        if (this.slices.size === 0) return;

        const request = this.nextRequest++;
        const outstanding = new Set(this.slices.keys());

        await new Promise<void>((resolve) => {
            const timer = setTimeout(() => {
                this.pending.delete(request);
                this.port.warn(
                    `gave up waiting for ${[...outstanding].length} worker(s) to seal codepoint block ${range}; ` +
                        'some glyphs in it may not draw',
                );
                resolve();
            }, SEAL_TIMEOUT_MS);

            this.pending.set(request, {outstanding, resolve, timer});
            this.port.post({type: 'seal', request, range});
        });
    }

    /** Forgets every worker, so that a rebuilt provider is not waited on by the old ones. */
    clear(): void {
        for (const pending of this.pending.values()) {
            clearTimeout(pending.timer);
            pending.resolve();
        }
        this.pending.clear();
        this.slices.clear();
    }

    private sealed(message: SealedMessage): void {
        if (message.entries.length) {
            this.port.register(message.entries);
        }
        const pending = this.pending.get(message.request);
        if (!pending) return;

        pending.outstanding.delete(message.worker);
        if (pending.outstanding.size > 0) return;

        clearTimeout(pending.timer);
        this.pending.delete(message.request);
        pending.resolve();
    }

    private sliceFor(worker: string): number {
        let slice = this.slices.get(worker);
        if (slice === undefined) {
            slice = this.slices.size;
            this.slices.set(worker, slice);
        }
        return slice;
    }
}
