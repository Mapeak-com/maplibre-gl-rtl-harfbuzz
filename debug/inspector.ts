/**
 * Drawing shaped text the way the map draws it.
 *
 * This does not approximate the map: it asks the plugin for the very distance fields that go into
 * the glyph atlas, places them at the metrics that go into the glyph protocol buffer, and resolves
 * them with the same threshold `symbol_sdf.fragment.glsl` uses. If a mark is a pixel out of place
 * here, it is a pixel out of place on the map, which is what makes this worth looking at.
 */

import type {Inspector} from './plugin.ts';

/** One shaped glyph, as `Shaper::inspect` reports it. */
export type ShapedGlyph = {
    font: number;
    glyph: number;
    /** Offset from the pen position, in quarters of a pixel at a 24 pixel em, y upwards. */
    dx: number;
    dy: number;
    /** How far the pen moves after this glyph, in whole pixels at a 24 pixel em. */
    advance: number;
    rtl: boolean;
    /** The character this came through shaping unchanged as, or null if shaping replaced it. */
    codepoint: number | null;
};

/** MapLibre's em, in pixels: `src/symbol/one_em.ts`. */
const EM = 24;

/** The padding a glyph bitmap carries on every side: `GLYPH_PBF_BORDER`. */
const BORDER = 3;

/** Where the distance field's contour sits: `(256 - 64) / 256` in the symbol shader. */
const CONTOUR = 192 / 256;

/** `EDGE_GAMMA` in the symbol shader, at a device pixel ratio of one. */
const EDGE_GAMMA = 0.105;

/** The band drawn around the baseline, in em units, chosen to hold ascenders and descenders. */
const ABOVE_BASELINE = 26;
const BELOW_BASELINE = 10;

const ENTRY_WIDTH = 7;

export function decodeInspection(flat: Int32Array): ShapedGlyph[] {
    const glyphs: ShapedGlyph[] = [];
    for (let at = 0; at + ENTRY_WIDTH <= flat.length; at += ENTRY_WIDTH) {
        glyphs.push({
            font: flat[at],
            glyph: flat[at + 1],
            dx: flat[at + 2],
            dy: flat[at + 3],
            advance: flat[at + 4],
            rtl: flat[at + 5] !== 0,
            codepoint: flat[at + 6] < 0 ? null : flat[at + 6],
        });
    }
    return glyphs;
}

/** A glyph's distance field, unpacked from what `Shaper::glyphImage` returns. */
type GlyphImage = {
    width: number;
    height: number;
    left: number;
    bearingY: number;
    field: Uint8Array;
};

function glyphImage(inspector: Inspector, glyph: ShapedGlyph): GlyphImage {
    const packed = inspector.glyphImage(glyph.font, glyph.glyph, glyph.dx, glyph.dy);
    const header = new Int32Array(packed.buffer, packed.byteOffset, 4);
    return {
        width: header[0],
        height: header[1],
        left: header[2],
        bearingY: header[3],
        field: packed.subarray(16),
    };
}

export type RenderOptions = {
    /** The `text-size` to draw at, as a style would set it. */
    size: number;
    color: string;
};

/**
 * Draws a shaped line into a canvas, resizing it to fit.
 *
 * Returns the width of the line in pixels, which is also what MapLibre would lay it out to.
 */
export function renderShapedText(
    canvas: HTMLCanvasElement,
    inspector: Inspector,
    glyphs: ShapedGlyph[],
    {size, color}: RenderOptions,
): number {
    // The canvas is drawn at the screen's own resolution and shown at the requested size, so that
    // the comparison is against the browser's text rather than against a scaled-up picture of it.
    const pixelRatio = globalThis.devicePixelRatio || 1;
    const fontScale = size / EM;
    const scale = fontScale * pixelRatio;
    const advance = glyphs.reduce((total, glyph) => total + glyph.advance, 0);

    const width = Math.max(1, Math.ceil((advance + 2 * BORDER) * scale));
    const height = Math.ceil((ABOVE_BASELINE + BELOW_BASELINE) * scale);
    canvas.width = width;
    canvas.height = height;
    canvas.style.width = `${Math.round(width / pixelRatio)}px`;
    canvas.style.height = `${Math.round(height / pixelRatio)}px`;

    // Coverage is accumulated on its own before being coloured, so that a mark overlapping the
    // letter it sits on does not double up where the two meet.
    const coverage = new Float32Array(width * height);
    // Half the width of the band the contour is softened over, in field units. The shader works it
    // out the same way, from the font scale and the device pixel ratio.
    const gamma = EDGE_GAMMA / (pixelRatio * fontScale);

    let pen = BORDER;
    for (const glyph of glyphs) {
        drawGlyph(coverage, width, height, glyphImage(inspector, glyph), pen, scale, gamma);
        pen += glyph.advance;
    }

    paint(canvas, coverage, color);
    return width;
}

function drawGlyph(
    coverage: Float32Array,
    canvasWidth: number,
    canvasHeight: number,
    image: GlyphImage,
    pen: number,
    scale: number,
    gamma: number,
): void {
    if (image.width === 0 || image.height === 0) return;

    const fieldWidth = image.width + 2 * BORDER;
    const fieldHeight = image.height + 2 * BORDER;

    // Where the top left of the padded field lands, in em units from the pen and the baseline.
    const originX = pen + image.left - BORDER;
    const originY = ABOVE_BASELINE - image.bearingY - BORDER;

    const fromX = Math.max(0, Math.floor(originX * scale));
    const toX = Math.min(canvasWidth, Math.ceil((originX + fieldWidth) * scale));
    const fromY = Math.max(0, Math.floor(originY * scale));
    const toY = Math.min(canvasHeight, Math.ceil((originY + fieldHeight) * scale));

    for (let y = fromY; y < toY; y++) {
        // The centre of the output pixel, in the field's own coordinates.
        const fieldY = (y + 0.5) / scale - originY - 0.5;
        for (let x = fromX; x < toX; x++) {
            const fieldX = (x + 0.5) / scale - originX - 0.5;
            const distance = sample(image.field, fieldWidth, fieldHeight, fieldX, fieldY) / 255;
            const alpha = smoothstep(CONTOUR - gamma, CONTOUR + gamma, distance);
            const at = y * canvasWidth + x;
            if (alpha > coverage[at]) coverage[at] = alpha;
        }
    }
}

/** Bilinear, as a texture sampler is, with the field held constant past its edges. */
function sample(field: Uint8Array, width: number, height: number, x: number, y: number): number {
    const x0 = Math.floor(x);
    const y0 = Math.floor(y);
    const fx = x - x0;
    const fy = y - y0;

    const at = (px: number, py: number) =>
        field[clamp(py, 0, height - 1) * width + clamp(px, 0, width - 1)];

    return (
        at(x0, y0) * (1 - fx) * (1 - fy) +
        at(x0 + 1, y0) * fx * (1 - fy) +
        at(x0, y0 + 1) * (1 - fx) * fy +
        at(x0 + 1, y0 + 1) * fx * fy
    );
}

function paint(canvas: HTMLCanvasElement, coverage: Float32Array, color: string): void {
    const context = canvas.getContext('2d');
    if (!context) return;

    const [red, green, blue] = parseColor(color);
    const image = context.createImageData(canvas.width, canvas.height);
    for (let at = 0; at < coverage.length; at++) {
        image.data[at * 4] = red;
        image.data[at * 4 + 1] = green;
        image.data[at * 4 + 2] = blue;
        image.data[at * 4 + 3] = Math.round(coverage[at] * 255);
    }
    context.putImageData(image, 0, 0);
}

function parseColor(color: string): [number, number, number] {
    const hex = color.replace('#', '');
    return [
        parseInt(hex.slice(0, 2), 16),
        parseInt(hex.slice(2, 4), 16),
        parseInt(hex.slice(4, 6), 16),
    ];
}

function smoothstep(edge0: number, edge1: number, value: number): number {
    const t = clamp((value - edge0) / (edge1 - edge0 || 1e-6), 0, 1);
    return t * t * (3 - 2 * t);
}

function clamp(value: number, low: number, high: number): number {
    return value < low ? low : value > high ? high : value;
}
