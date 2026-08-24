/**
 * The contract between the two halves of the plugin.
 *
 * Shaping happens in MapLibre's web workers, because that is where the text plugin interface lives.
 * Drawing happens on the main thread, because that is where glyph requests are made. Neither half
 * can call the other: MapLibre's own worker channel is not ours to use, and the two run in
 * different realms.
 *
 * So they talk over a `BroadcastChannel`, and this package is the only place that says what they
 * say to each other. It depends on nothing -- not on MapLibre, not on the DOM, not on the
 * WebAssembly module -- so that both halves can be read against one description of the protocol.
 */

export * from './messages.ts';
export * from './pool.ts';
export * from './glyphs-url.ts';
