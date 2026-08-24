/**
 * Finding the drawing half.
 *
 * MapLibre gives every worker an actor of its own, and the page registers its handlers before it
 * hands MapLibre the plugin URL — so by the time this runs there is always something listening, and
 * asking once is enough.
 */

import {
    GLOBAL_DISPATCHER_ID,
    JOIN,
    type Join,
    type Welcome,
    type WorkerActor,
    type WorkerGlobal,
} from '@maplibre-rtl-harfbuzz/protocol';

declare const self: WorkerGlobal;

export type Joined = {
    actor: WorkerActor;
    welcome: Welcome;
    worker: string;
};

export async function join(): Promise<Joined> {
    const actor = self.worker?.actor;
    if (!actor) {
        throw new Error(
            'maplibre-gl-rtl-harfbuzz: this file is MapLibre\'s worker half of the plugin, and it ' +
                'found no MapLibre worker around it. Pass its URL to `registerHarfBuzzTextPlugin` ' +
                'rather than loading it yourself.',
        );
    }

    const worker = workerId();
    const welcome = (await actor.sendAsync({
        type: JOIN,
        data: {worker} satisfies Join,
        targetMapId: GLOBAL_DISPATCHER_ID,
    })) as Welcome | null;

    if (!welcome) {
        throw new Error(
            'maplibre-gl-rtl-harfbuzz: no glyph provider answered. Call ' +
                '`registerHarfBuzzTextPlugin` before the map starts loading tiles.',
        );
    }

    return {actor, welcome, worker};
}

function workerId(): string {
    return globalThis.crypto?.randomUUID?.() ?? `worker-${Math.random().toString(36).slice(2)}`;
}
