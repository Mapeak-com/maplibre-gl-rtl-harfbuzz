/**
 * Getting font files into memory, and choosing which instance of one to read.
 */

/** How a font is asked for: a URL, the bytes themselves, or either with an instance named. */
export type FontSource = FontFile | (FontInstance & {source: FontFile});

type FontFile = string | URL | ArrayBuffer | ArrayBufferView;

/**
 * The instance of a variable font to read.
 *
 * A variable font file holds one set of outlines plus the deltas that carry them from one weight or
 * width to another, and the outlines in the file are its *default* instance -- which for several of
 * Google's Noto families is the thin master rather than the regular one. So an instance is always
 * asked for, on the same scales CSS uses. A font that is not variable ignores both.
 */
export type FontInstance = {
    /** `wght`, from 100 to 900. 400 is what `font-weight: normal` means. */
    weight?: number;
    /** `wdth`, as a percentage of normal. 100 is what `font-stretch: normal` means. */
    width?: number;
};

/** A font file with its instance settled, ready to be handed to a shaper. */
export type LoadedFont = Required<FontInstance> & {bytes: ArrayBuffer};

const DEFAULT_INSTANCE: Required<FontInstance> = {weight: 400, width: 100};

export async function loadFont(source: FontSource): Promise<LoadedFont> {
    const {file, instance} = split(source);
    return {...DEFAULT_INSTANCE, ...instance, bytes: await read(file)};
}

function split(source: FontSource): {file: FontFile; instance: FontInstance} {
    if (typeof source === 'object' && source !== null && 'source' in source) {
        const {source: file, ...instance} = source;
        return {file, instance};
    }
    return {file: source, instance: {}};
}

async function read(file: FontFile): Promise<ArrayBuffer> {
    if (typeof file === 'string' || file instanceof URL) {
        const response = await fetch(String(file));
        if (!response.ok) {
            throw new Error(
                `maplibre-gl-rtl-harfbuzz: could not fetch the font at ${file} (${response.status})`,
            );
        }
        return response.arrayBuffer();
    }

    if (ArrayBuffer.isView(file)) {
        // Copied rather than referenced: the bytes are handed to the workers as well, and a view
        // over somebody else's buffer would take whatever else is in it along with it.
        return file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength) as ArrayBuffer;
    }

    return file;
}
