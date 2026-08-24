/**
 * Builds the Rust workspace to WebAssembly.
 *
 * Size is the reason this is a script rather than a one-line `wasm-pack` call. The module carries
 * HarfBuzz's shaping tables and the Unicode data behind the bidirectional algorithm and script
 * itemization, none of which compress away, so the build reports what it produced and how well it
 * compresses -- the number that actually reaches anyone over the wire.
 */

import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {brotliCompressSync, gzipSync} from 'node:zlib';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'packages', 'wasm', 'pkg');

execFileSync(
    'wasm-pack',
    ['build', '--release', '--target', 'web', '--out-dir', out, '--out-name', 'shaper'],
    {cwd: join(root, 'crates', 'wasm-bindings'), stdio: 'inherit'},
);

const wasm = readFileSync(join(out, 'shaper_bg.wasm'));

console.log(
    `\nshaper_bg.wasm  ${kilobytes(wasm.length)} raw` +
        `  ${kilobytes(gzipSync(wasm).length)} gzip` +
        `  ${kilobytes(brotliCompressSync(wasm).length)} brotli`,
);

function kilobytes(bytes: number): string {
    return `${(bytes / 1024).toFixed(0)} kB`;
}
