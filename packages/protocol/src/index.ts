/**
 * The contract between the two halves of the plugin.
 *
 * Shaping happens in MapLibre's web workers, because that is where the text plugin interface lives.
 * Drawing happens on the main thread, because that is where glyph requests are answered. They talk
 * over MapLibre's own worker protocol, which both halves can reach, and this package is the only
 * place that says what they say to each other and the only place that admits to borrowing that
 * protocol.
 *
 * It depends on nothing -- not on MapLibre, not on the DOM, not on the WebAssembly module -- so that
 * both halves can be read against one description of the exchange.
 */

export * from './actor.ts';
export * from './messages.ts';
export * from './pool.ts';
export * from './glyphs-url.ts';
