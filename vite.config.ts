import {defineConfig} from 'vite';

/**
 * The debug page. Its static files include the built plugin, which it loads from a URL at runtime
 * rather than importing, so that the parts that resolve their own URLs are exercised as they would
 * be on a real page.
 */
export default defineConfig({
    root: 'debug',
    publicDir: 'public',
    server: {open: true},
    // MapLibre 6 loads its worker from a file next to its own module, which Vite's dependency
    // pre-bundling does not carry along; served from source, the worker resolves.
    optimizeDeps: {exclude: ['maplibre-gl']},
    build: {outDir: '../dist-debug', emptyOutDir: true},
});
