/**
 * Finding the drawing half.
 *
 * A worker has no way to know whether the other half is listening yet -- MapLibre may have started
 * loading the plugin before the page finished setting it up -- so the announcement is repeated
 * until it is answered. `BroadcastChannel` delivers to whoever is listening at the time and keeps
 * nothing for anyone who arrives later, which is what makes the repetition necessary.
 */

import {CHANNEL_NAME, type ChannelMessage, type WelcomeMessage} from '@maplibre-rtl-harfbuzz/protocol';

/** How often to announce ourselves again while waiting to be answered. */
const RETRY_INTERVAL_MS = 100;

/**
 * How long to keep trying.
 *
 * MapLibre waits five seconds for a plugin to register itself before giving up on it, so there is
 * no point still trying after four.
 */
const GIVE_UP_MS = 4000;

export async function join(
    channelName: string = CHANNEL_NAME,
): Promise<{channel: BroadcastChannel; welcome: WelcomeMessage}> {
    const worker = workerId();
    const channel = new BroadcastChannel(channelName);

    return new Promise((resolve, reject) => {
        const announce = () => channel.postMessage({type: 'hello', worker});
        const retry = setInterval(announce, RETRY_INTERVAL_MS);
        const giveUp = setTimeout(() => {
            stop();
            channel.close();
            reject(
                new Error(
                    'maplibre-gl-rtl-harfbuzz: no glyph provider answered. Call ' +
                        '`registerHarfBuzzTextPlugin` before the map starts loading tiles, and check ' +
                        'that both halves are using the same channel name.',
                ),
            );
        }, GIVE_UP_MS);

        const stop = () => {
            clearInterval(retry);
            clearTimeout(giveUp);
        };

        channel.onmessage = (event: MessageEvent<ChannelMessage>) => {
            const message = event.data;
            if (message.type !== 'welcome' || message.worker !== worker) return;
            stop();
            resolve({channel, welcome: message});
        };

        announce();
    });
}

function workerId(): string {
    return globalThis.crypto?.randomUUID?.() ?? `worker-${Math.random().toString(36).slice(2)}`;
}
