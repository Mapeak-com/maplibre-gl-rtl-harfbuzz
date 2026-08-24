/**
 * Copies the built plugin into the debug page's static files.
 *
 * The debug page loads the plugin the way a real page would -- from a URL, at runtime -- rather
 * than importing the sources. That is the only way to exercise the parts that matter: MapLibre
 * fetches the worker half from a URL of its own, and both halves resolve the WebAssembly module
 * against wherever they were loaded from.
 */

import {cp, mkdir} from 'node:fs/promises';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const from = join(root, 'packages', 'maplibre-gl-rtl-harfbuzz', 'dist');
const to = join(root, 'debug', 'public', 'plugin');

await mkdir(to, {recursive: true});
await cp(from, to, {recursive: true});
console.log(`copied the plugin into ${to}`);
