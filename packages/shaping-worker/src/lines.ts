/**
 * Unpacking the reordered lines, and putting them back into the two things MapLibre counts in.
 *
 * MapLibre indexes a line's style sections by UTF-16 code unit, while the shaper works in
 * codepoints. Every codepoint a shaped glyph gets comes from plane 15 or 16, so it is two code
 * units, and getting this wrong would misattribute the font and size of every glyph after the first
 * one in a formatted label.
 */

/** One line: its text, and the style section of each of its UTF-16 code units. */
export type DecodedLine = [text: string, sections: number[]];

/**
 * Reads the flat encoding the shaper returns: a line count, then for each line its length in
 * codepoints followed by that many `codepoint, section` pairs.
 */
export function decodeLines(encoded: Uint32Array): DecodedLine[] {
    const lines: DecodedLine[] = [];
    let at = 0;
    const lineCount = encoded[at++] ?? 0;

    for (let line = 0; line < lineCount; line++) {
        const length = encoded[at++];
        const codepoints: number[] = [];
        const sections: number[] = [];

        for (let index = 0; index < length; index++) {
            const codepoint = encoded[at++];
            const section = encoded[at++];
            codepoints.push(codepoint);
            // One entry per UTF-16 code unit, which is two for anything above the basic plane.
            sections.push(section);
            if (codepoint > 0xffff) sections.push(section);
        }

        lines.push([toString(codepoints), sections]);
    }

    return lines;
}

/** `String.fromCodePoint` takes its input as arguments, so a very long label goes over in pieces. */
const CHUNK = 4096;

function toString(codepoints: number[]): string {
    if (codepoints.length <= CHUNK) return String.fromCodePoint(...codepoints);
    let out = '';
    for (let at = 0; at < codepoints.length; at += CHUNK) {
        out += String.fromCodePoint(...codepoints.slice(at, at + CHUNK));
    }
    return out;
}
