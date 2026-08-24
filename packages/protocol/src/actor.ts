/**
 * The slice of MapLibre's worker protocol this plugin borrows, and the one place the borrowing is
 * admitted to.
 *
 * MapLibre already runs a message channel between the page and its workers, and both ends of it are
 * reachable from a plugin: `getGlobalDispatcher()` on the page, `self.worker.actor` inside a worker.
 * What is not reachable is the *type*: `MessageType` is a `const enum` and `RequestResponseMessageMap`
 * is a closed mapped type, so a message MapLibre does not already know about cannot be named without
 * widening them.
 *
 * These two types are that widening, written down once so that the casts live here rather than at
 * every call. An unregistered message type is answered with `null` rather than an error
 * (`Actor.processTask`), so borrowing the channel this way cannot break MapLibre's own traffic.
 *
 * If MapLibre gains a general-purpose plugin message, this file is the only thing that has to change.
 */

/** MapLibre's id for the dispatcher shared by every map on the page. */
export const GLOBAL_DISPATCHER_ID = 'global-dispatcher';

/** What a plugin needs of the worker-side `Actor`. */
export type WorkerActor = {
    sendAsync(
        message: {type: string; data: unknown; targetMapId?: string | number},
        abortController?: AbortController,
    ): Promise<unknown>;
    registerMessageHandler(
        type: string,
        handler: (mapId: string | number, params: never) => Promise<unknown>,
    ): void;
};

/** What a plugin needs of the main-thread `Dispatcher`. */
export type MainThreadDispatcher = {
    /** Sends to every worker and waits for all of them. Unregistered types answer with `null`. */
    broadcast(type: string, data: unknown): Promise<unknown[]>;
    registerMessageHandler(
        type: string,
        handler: (mapId: string | number, params: never) => Promise<unknown>,
    ): Promise<void>;
};

/** MapLibre's worker global, which carries the worker's own actor. */
export type WorkerGlobal = {
    worker?: {actor?: WorkerActor};
};
