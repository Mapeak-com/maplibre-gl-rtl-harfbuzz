/**
 * The debug page.
 *
 * Two things are going on at once. The map is the real test: MapLibre, unmodified, drawing labels
 * through the plugin. The panel beside it is the explanation: the same text drawn three ways --
 * as MapLibre would draw it with no plugin, as the plugin draws it, and as the browser's own text
 * engine draws it -- with the shaped glyphs listed underneath.
 */

// MapLibre GL JS 6 has no default export; everything it offers is a named one.
import * as maplibregl from 'maplibre-gl';

import {decodeInspection, renderShapedText, type ShapedGlyph} from './inspector.ts';
import {FONTS, loadPlugin, type Inspector} from './plugin.ts';
import {DEFAULT_INSPECTION, SAMPLES} from './samples.ts';
import {buildStyle, type LabelSettings} from './style.ts';

const TEXT_COLOR = '#e7eaf0';

const status = element<HTMLParagraphElement>('status');

try {
    await main();
} catch (error) {
    status.className = 'error';
    status.textContent = `${error}`;
    throw error;
}

async function main(): Promise<void> {
    const {registerHarfBuzzTextPlugin, glyphsUrl} = await loadPlugin();
    const provider = await registerHarfBuzzTextPlugin(maplibregl, {fonts: FONTS});
    status.textContent = `Shaping with ${FONTS.length} font files. The map below is unmodified MapLibre GL JS.`;

    await registerReferenceFonts();

    const settings: LabelSettings = {size: 28, halo: true, alongLine: false, custom: ''};
    const map = new maplibregl.Map({
        container: 'map',
        style: buildStyle(glyphsUrl(), settings) as never,
        center: [55, 24],
        zoom: 2.1,
        attributionControl: false,
    });
    map.addControl(new maplibregl.NavigationControl(), 'top-left');
    // Handy from the console, and from the screenshots this page is checked with.
    (globalThis as {debugMap?: unknown}).debugMap = map;

    const restyle = () => map.setStyle(buildStyle(glyphsUrl(), settings) as never);
    const inspect = makeInspectorPanel(provider.inspector, settings);

    const text = element<HTMLInputElement>('text');
    text.value = DEFAULT_INSPECTION;
    text.addEventListener('input', () => {
        settings.custom = text.value;
        inspect(text.value, noteFor(text.value));
        restyle();
    });

    bindRange('size', (value) => {
        settings.size = value;
        inspect(text.value, noteFor(text.value));
        restyle();
    });
    bindCheckbox('halo', (on) => {
        settings.halo = on;
        restyle();
    });
    bindCheckbox('along-line', (on) => {
        settings.alongLine = on;
        restyle();
    });

    buildSampleButtons((sample) => {
        text.value = sample.text;
        settings.custom = sample.text;
        inspect(sample.text, sample.without);
        restyle();
        map.easeTo({center: sample.at, zoom: 4});
    });

    inspect(text.value, noteFor(text.value));
}

/** Draws one string three ways and lists what shaping made of it. */
function makeInspectorPanel(inspector: Inspector, settings: LabelSettings) {
    const plain = element<HTMLCanvasElement>('canvas-plain');
    const shaped = element<HTMLCanvasElement>('canvas-shaped');
    const reference = element<HTMLDivElement>('reference');
    const withoutNote = element<HTMLParagraphElement>('without-note');
    const summary = element<HTMLParagraphElement>('glyph-summary');
    const rows = element<HTMLTableSectionElement>('glyphs');

    return (text: string, note: string) => {
        const options = {size: settings.size, color: TEXT_COLOR};
        // The table lists the glyphs in the order they were written, which is the order shaping
        // works in; the canvas draws them in the order they end up on screen.
        const shapedGlyphs = decodeInspection(inspector.inspect(text));
        // Without a plugin MapLibre draws codepoints in the order they were written, left to right,
        // whichever direction the script runs -- so this one is not reordered either.
        const plainGlyphs = decodeInspection(inspector.inspectUnshaped(text));

        renderShapedText(plain, inspector, plainGlyphs, options);
        renderShapedText(shaped, inspector, decodeInspection(inspector.inspectVisual(text)), options);

        withoutNote.textContent = note;
        reference.style.font = `${settings.size}px ${referenceFontStack()}`;
        reference.style.direction = paragraphDirection(text);
        reference.textContent = text;

        const replaced = shapedGlyphs.filter((glyph) => glyph.codepoint === null).length;
        summary.textContent =
            `${[...text].length} characters became ${shapedGlyphs.length} glyphs, ` +
            `${replaced} of which no codepoint stands for any more.`;

        rows.replaceChildren(...shapedGlyphs.map(row));
    };
}

function row(glyph: ShapedGlyph, index: number): HTMLTableRowElement {
    const tr = document.createElement('tr');
    if (glyph.codepoint === null) tr.className = 'shaped';
    const cells = [
        String(index),
        String(glyph.font),
        String(glyph.glyph),
        String(glyph.dx),
        String(glyph.dy),
        String(glyph.advance),
        glyph.codepoint === null ? '—' : `U+${glyph.codepoint.toString(16).toUpperCase().padStart(4, '0')}`,
    ];
    tr.append(
        ...cells.map((value) => {
            const td = document.createElement('td');
            td.textContent = value;
            td.className = 'mono';
            return td;
        }),
    );
    return tr;
}

function buildSampleButtons(onPick: (sample: (typeof SAMPLES)[number]) => void): void {
    const container = element<HTMLDivElement>('samples');
    container.replaceChildren(
        ...SAMPLES.map((sample) => {
            const button = document.createElement('button');
            button.textContent = sample.text;
            const label = document.createElement('small');
            label.textContent = sample.language;
            button.append(label);
            button.addEventListener('click', () => onPick(sample));
            return button;
        }),
    );
}

/**
 * Which way the line runs, by the same rule the algorithm uses: the direction of the first strong
 * character, and left to right if there is none. Getting this from "does it contain any Hebrew"
 * would align the reference differently from the paragraph it is standing in for.
 */
function paragraphDirection(text: string): 'ltr' | 'rtl' {
    const strong = /[\p{Script=Hebrew}\p{Script=Arabic}\p{Script=Syriac}\p{Script=Thaana}\p{Script=Nko}\p{Script=Adlam}]|[A-Za-z\u00C0-\u02AF\u0370-\u052F\u0900-\u1FFF\u2C00-\uD7FF]/u;
    const match = strong.exec(text);
    if (!match) return 'ltr';
    return /[\p{Script=Hebrew}\p{Script=Arabic}\p{Script=Syriac}\p{Script=Thaana}\p{Script=Nko}\p{Script=Adlam}]/u.test(match[0])
        ? 'rtl'
        : 'ltr';
}

function noteFor(text: string): string {
    return (
        SAMPLES.find((sample) => sample.text === text)?.without ??
        'Every codepoint is laid out as its own glyph at its own advance, with nothing reordered, ' +
            'joined, or moved.'
    );
}

/**
 * Registers the same font files with the browser, so the reference line is drawn from the very
 * files the plugin shapes with rather than from whatever the system happens to have.
 */
async function registerReferenceFonts(): Promise<void> {
    // One family per file, rather than one family for all of them: a CSS font stack falls through
    // file by file in the order it is written, which is the same rule the plugin follows, while
    // several files sharing a family name leaves the choice to the browser.
    await Promise.all(
        FONTS.map(async (url, index) => {
            const face = new FontFace(referenceFamily(index), `url(${url})`);
            document.fonts.add(await face.load());
        }),
    );
}

function referenceFamily(index: number): string {
    return `Debug Reference ${index}`;
}

function referenceFontStack(): string {
    return FONTS.map((_, index) => `"${referenceFamily(index)}"`).join(', ');
}

function bindRange(id: string, onChange: (value: number) => void): void {
    const input = element<HTMLInputElement>(id);
    const output = element<HTMLSpanElement>(`${id}-value`);
    input.addEventListener('input', () => {
        output.textContent = input.value;
        onChange(Number(input.value));
    });
}

function bindCheckbox(id: string, onChange: (checked: boolean) => void): void {
    const input = element<HTMLInputElement>(id);
    input.addEventListener('change', () => onChange(input.checked));
}

function element<T extends HTMLElement>(id: string): T {
    const found = document.getElementById(id);
    if (!found) throw new Error(`the page has no #${id}`);
    return found as T;
}
