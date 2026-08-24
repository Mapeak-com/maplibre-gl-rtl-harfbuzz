/**
 * Bundles the two halves of the plugin.
 *
 * They are built separately because they are loaded separately and by different things: the page
 * imports one, and MapLibre fetches the other into each of its workers.
 *
 * Both keep the `.mjs` extension, and that is not cosmetic. MapLibre decides how to load a text
 * plugin from its extension: a `.mjs` URL is loaded with a dynamic `import()`, and anything else is
 * fetched and run, which would lose the module context the worker half needs.
 *
 * The WebAssembly module sits beside them rather than inlined, so that one download serves the page
 * and every worker.
 */

import {execFileSync} from 'node:child_process';
import {copyFile, mkdir} from 'node:fs/promises';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import * as esbuild from 'esbuild';

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, 'dist');
const wasmPackage = join(here, '..', 'wasm', 'pkg');

await mkdir(dist, {recursive: true});

await esbuild.build({
    entryPoints: [join(here, 'src', 'index.ts'), join(here, 'src', 'text-plugin.ts')],
    outdir: dist,
    outExtension: {'.js': '.mjs'},
    bundle: true,
    format: 'esm',
    platform: 'browser',
    target: 'es2022',
    sourcemap: true,
    minify: true,
    logLevel: 'info',
});

await copyFile(join(wasmPackage, 'shaper_bg.wasm'), join(dist, 'shaper_bg.wasm'));

// Declarations come from the sources rather than being maintained by hand alongside them.
execFileSync('npx', ['tsc', '-p', join(here, 'tsconfig.build.json')], {stdio: 'inherit'});
