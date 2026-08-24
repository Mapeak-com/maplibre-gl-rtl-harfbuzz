/**
 * Keeps track of the shaping workers, and closes a block of codepoints before it is drawn.
 *
 * MapLibre asks for a block of glyphs once and remembers that it has: a glyph that appears in a
 * block after it has been answered would be asked for never, and would silently not draw. So before
 * a block is drawn, every worker is asked to seal it -- to stop allocating into it, and to hand over
 * anything it has allocated there but not yet reported. Only once they have all answered is the
 * block complete, and it stays complete because nothing may be added to a sealed block.
 */

import {
    codepointRangeForWorker,
    JOIN,
    REPORT_GLYPHS,
    SEAL_RANGE,
    type Join,
    type MainThreadDispatcher,
    type ReportGlyphs,
    type Sealed,
    type SealRange,
    type Welcome,
} from '@maplibre-rtl-harfbuzz/protocol';

/** What the registry needs from the outside, so that it can be driven in a test without a map. */
export type RegistryPort = {
    /** Records what a worker's codepoints stand for. */
    register(entries: Int32Array): void;
    /** What a joining worker is given to shape with. */
    welcome(): Pick<Welcome, 'wasmUrl' | 'fonts'>;
};

export class WorkerRegistry {
    private readonly dispatcher: MainThreadDispatcher;
    private readonly port: RegistryPort;
    /** Which stretch of the pool each worker was given, so a second join gets the same answer. */
    private readonly slices = new Map<string, number>();

    constructor(dispatcher: MainThreadDispatcher, port: RegistryPort) {
        this.dispatcher = dispatcher;
        this.port = port;
    }

    get workerCount(): number {
        return this.slices.size;
    }

    /** Starts listening for the workers. Handlers are in place before any worker can ask. */
    async listen(): Promise<void> {
        await this.dispatcher.registerMessageHandler(JOIN, async (_mapId, {worker}: Join) =>
            this.welcome(worker),
        );
        await this.dispatcher.registerMessageHandler(
            REPORT_GLYPHS,
            async (_mapId, {entries}: ReportGlyphs) => {
                this.port.register(entries);
            },
        );
    }

    /**
     * Asks every worker to seal a block and waits for them to answer.
     *
     * A worker that has not loaded the plugin yet answers with `null`, which is MapLibre's own reply
     * to a message type nothing is listening for. There is nothing to wait for in that case: a
     * worker with no plugin has allocated nothing.
     */
    async seal(range: number): Promise<void> {
        if (this.slices.size === 0) return;

        const answers = await this.dispatcher.broadcast(SEAL_RANGE, {range} satisfies SealRange);
        for (const answer of answers) {
            const entries = (answer as Sealed | null)?.entries;
            if (entries?.length) this.port.register(entries);
        }
    }

    private welcome(worker: string): Welcome {
        let slice = this.slices.get(worker);
        if (slice === undefined) {
            slice = this.slices.size;
            this.slices.set(worker, slice);
        }

        const {wasmUrl, fonts} = this.port.welcome();
        return {
            wasmUrl,
            // Copied, because MapLibre's worker protocol *transfers* array buffers rather than
            // cloning them. Sending the originals would empty them, leaving the next worker -- and
            // the page's own copy -- with nothing to read.
            fonts: fonts.map((font) => ({...font, bytes: font.bytes.slice(0)})),
            ...codepointRangeForWorker(slice),
        };
    }
}
