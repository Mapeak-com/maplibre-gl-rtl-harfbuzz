/**
 * Fetches the fonts the debug page draws with, and the fixtures the tests compare against.
 *
 * Neither set is in version control: they are other people's to distribute, they are large, and the
 * plugin has no opinion about which fonts anyone uses. The Noto files are simply a set that covers
 * the scripts worth looking at, all from one family so that a map made of them looks like one map.
 *
 * The debug fonts are deliberately the variable versions: reading one is a fair test that the
 * plugin handles what a real style would hand it, and it was a variable font that caught the
 * default-instance bug in the first place.
 */

import {mkdir, stat, writeFile} from 'node:fs/promises';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

/** A file to fetch: where it goes, and where it comes from. */
type Download = {
    name: string;
    url: string;
};

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Latin, Greek and Cyrillic first, so that everything else falls through to its own script's font. */
const NOTO_FAMILIES: Array<[folder: string, name: string]> = [
    ['notosans', 'NotoSans'],
    ['notosanshebrew', 'NotoSansHebrew'],
    ['notosansarabic', 'NotoSansArabic'],
    ['notosansdevanagari', 'NotoSansDevanagari'],
    ['notosansbengali', 'NotoSansBengali'],
    ['notosanstamil', 'NotoSansTamil'],
    ['notosansthai', 'NotoSansThai'],
    ['notosanskhmer', 'NotoSansKhmer'],
];

const fonts: Download[] = NOTO_FAMILIES.map(([folder, name]) => ({
    name: `${name}.ttf`,
    url:
        `https://raw.githubusercontent.com/google/fonts/main/ofl/${folder}/` +
        encodeURIComponent(`${name}[wdth,wght].ttf`),
}));

/**
 * What the rasterizer is checked against: a font a public glyph server draws with, and the block of
 * glyphs it serves for it. The conventions of the glyph protocol buffer are written down nowhere,
 * so the only way to be sure of them is to compare against glyphs someone else produced.
 */
const fixtures: Download[] = [
    {
        name: 'NotoSans-Regular.ttf',
        url: 'https://github.com/openmaptiles/fonts/raw/master/noto-sans/NotoSans-Regular.ttf',
    },
    {
        name: 'noto-sans-regular-0-255.pbf',
        url: 'https://demotiles.maplibre.org/font/Noto%20Sans%20Regular/0-255.pbf',
    },
];

await Promise.all([
    fetchAll(fonts, join(root, 'debug', 'public', 'fonts')),
    fetchAll(fixtures, join(root, 'tests', 'fixtures')),
]);

async function fetchAll(downloads: Download[], directory: string): Promise<void> {
    await mkdir(directory, {recursive: true});
    await Promise.all(downloads.map((download) => fetchOne(download, directory)));
}

async function fetchOne({name, url}: Download, directory: string): Promise<void> {
    const path = join(directory, name);
    if (await exists(path)) {
        console.log(`${name} is already here`);
        return;
    }

    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`could not fetch ${url} (${response.status})`);
    }

    const bytes = new Uint8Array(await response.arrayBuffer());
    await writeFile(path, bytes);
    console.log(`${name}  ${(bytes.length / 1024).toFixed(0)} kB`);
}

async function exists(path: string): Promise<boolean> {
    return stat(path).then(
        () => true,
        () => false,
    );
}
