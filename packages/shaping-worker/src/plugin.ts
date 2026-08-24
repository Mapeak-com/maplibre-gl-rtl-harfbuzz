/**
 * The three functions MapLibre asks a text plugin for, and what this plugin does with them.
 */

import init, {Shaper} from '@maplibre-rtl-harfbuzz/wasm';

import {join} from './join.ts';
import {decodeLines} from './lines.ts';
import {GlyphReporter} from './reporter.ts';

/**
 * The interface MapLibre's `registerRTLTextPlugin` takes.
 *
 * The names are a legacy of the plugin this replaces, which only did Arabic shaping and
 * bidirectional ordering. What they are called matters less than when MapLibre calls them:
 * `applyArabicShaping` runs over a label's text *before* MapLibre works out which glyphs the tile
 * needs, which is the only point at which a shaped glyph can be introduced and still be fetched.
 */
export type RTLTextPlugin = {
    applyArabicShaping: (text: string) => string;
    processBidirectionalText: (text: string, lineBreaks: number[]) => string[];
    processStyledBidirectionalText: (
        text: string,
        sections: number[],
        lineBreaks: number[],
    ) => Array<[string, number[]]>;
};

/**
 * Joins the drawing half, loads what it sends, and returns the plugin MapLibre should register.
 *
 * MapLibre gives a plugin five seconds between fetching its script and registering itself, which is
 * time enough to fetch a WebAssembly module and parse a few font files. Registering only once
 * everything is ready is deliberate: a plugin that registered early would be asked to shape text it
 * could not yet shape, and MapLibre would carry the unshaped result into tiles it has already built.
 */
export async function startShaping(): Promise<RTLTextPlugin> {
    const {actor, welcome} = await join();
    await init({module_or_path: welcome.wasmUrl});

    const shaper = new Shaper();
    for (const font of welcome.fonts) {
        shaper.addFont(new Uint8Array(font.bytes), font.weight, font.width);
    }
    shaper.restrictCodepoints(welcome.codepointStart, welcome.codepointEnd);

    const reporter = new GlyphReporter(shaper, actor);
    reporter.listen();

    return createPlugin(shaper, reporter);
}

function createPlugin(shaper: Shaper, reporter: GlyphReporter): RTLTextPlugin {
    return {
        applyArabicShaping(text) {
            const shaped = shaper.applyShaping(text);
            reporter.schedule();
            return shaped;
        },

        processBidirectionalText(text, lineBreaks) {
            return decodeLines(
                shaper.processBidi(text, new Uint32Array(0), Uint32Array.from(lineBreaks)),
            ).map(([line]) => line);
        },

        processStyledBidirectionalText(text, sections, lineBreaks) {
            return decodeLines(
                shaper.processBidi(text, Uint32Array.from(sections), Uint32Array.from(lineBreaks)),
            );
        },
    };
}
