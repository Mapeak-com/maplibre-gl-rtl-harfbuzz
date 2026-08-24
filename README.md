# maplibre-gl-rtl-harfbuzz

Complex-script and right-to-left text for **MapLibre GL JS 6**, with HarfBuzz compiled to
WebAssembly — no C++, no ICU, and no changes to MapLibre.

MapLibre lays text out one codepoint at a time, at that codepoint's own advance. Most of the world's
writing systems are not like that:

| | without a plugin | with this plugin |
| --- | --- | --- |
| Hebrew with niqqud | vowel points march along the line as if they were letters | hung under the letters they belong to |
| Arabic with tashkeel | unjoined letters, marks strewn across the line | joined, with the marks on their letters |
| Devanagari, Bengali | `दलि्ली` — vowel signs in storage order, conjuncts unformed | `दिल्ली` |
| Tamil, Khmer, Thai | vowels and subscripts placed after their consonants | placed around and under them |
| mixed direction | logical order, left to right | the order it is read in |

It renders on **unmodified maplibre-gl-js 6.5.0**. How it manages that, and what MapLibre could add
so that it would not have to, is in [docs/maplibre-api-proposal.md](docs/maplibre-api-proposal.md).

## Using it

```ts
// MapLibre GL JS 6 has no default export.
import * as maplibregl from 'maplibre-gl';
import {registerHarfBuzzTextPlugin, glyphsUrl} from 'maplibre-gl-rtl-harfbuzz';

await registerHarfBuzzTextPlugin(maplibregl, {
    // Tried in order: the first file with a glyph for a character is the one it is drawn with.
    fonts: [
        '/fonts/NotoSans.ttf',
        '/fonts/NotoSansHebrew.ttf',
        '/fonts/NotoSansDevanagari.ttf',
        {source: '/fonts/NotoSansArabic.ttf', weight: 400},
    ],
});

const map = new maplibregl.Map({
    container: 'map',
    style: {
        ...style,
        // The plugin draws every glyph, so the style points at it rather than at a glyph server.
        glyphs: glyphsUrl(),
    },
});
```

Call it before the map starts loading tiles. It must be registered eagerly rather than deferred:
MapLibre only fetches a deferred text plugin once it meets right-to-left text, and Devanagari, Tamil
and Khmer are not right to left.

### Fonts

Give it the font files themselves — TrueType or OpenType, including variable fonts. WOFF and WOFF2
are compressed and cannot be read; use the uncompressed file.

The plugin then serves the style's whole fontstack out of those files, which is why the style's
`glyphs` has to point at it. That is the point of the arrangement rather than a limitation of it:
shaping produces glyph *indices*, and an index means nothing unless whoever draws it has the same
file open.

A variable font is read at `weight: 400, width: 100` by default, which is what `font-weight: normal`
and `font-stretch: normal` mean in CSS. This matters more than it sounds: several of Google's Noto
variable fonts keep their thin master in the file, so a file read without asking for an instance
draws hairlines.

### What it does not do

- **Vertical text.** Passed through unchanged; MapLibre's vertical layout still assumes one
  codepoint is one glyph.
- **`text-letter-spacing` on shaped text.** MapLibre adds the spacing between every glyph, including
  between a mark and its base. Leave it at zero for scripts that need shaping.
- **`is-supported-script`.** A style using that expression still falls back to `name:latin` for
  Indic scripts, because the list behind it is hard-coded in MapLibre. See §3.2 of the proposal.
- **Fonts the plugin was not given.** There is no fallback to a glyph server; a character no font
  covers simply does not draw.

## The debug page

```sh
npm install
npm run fetch-fonts   # Noto files from Google Fonts, and the glyph server test fixtures
npm run dev
```

A live map drawing every sample through the plugin, beside a panel that draws one string three
ways — as MapLibre would draw it with no plugin, as this plugin draws it, and as the browser's own
text engine draws it — with the shaped glyphs listed underneath. The middle rendering is not an
approximation: it uses the very distance fields that go into the glyph atlas, placed at the metrics
that go into the glyph protocol buffer, resolved with the threshold from
`symbol_sdf.fragment.glsl`. If a mark is a pixel out of place there, it is a pixel out of place on
the map.

## How it works

Two halves, in two places.

**Shaping**, in MapLibre's workers, where the text plugin interface lives. It runs the Unicode
Bidirectional Algorithm, splits the text into runs of one direction, one script and one font, and
shapes each run with HarfBuzz. Each glyph that no longer stands for a character gets a codepoint
from the supplementary private use areas, and the shaped text is handed back to MapLibre as a string
of those. MapLibre carries them through its pipeline exactly as before — collecting them as the
glyphs a tile needs, measuring them for line breaking, laying them out.

Text that shaping does not change comes back as the characters it was, so ordinary Latin labels stay
ordinary codepoints: line breaking still sees its spaces and hyphens, and the glyph atlas does not
fill up with a private copy of the alphabet.

**Drawing**, on the main thread, where glyph requests are made. The plugin registers itself as a
protocol and answers the style's `glyphs` URL, rasterizing each glyph out of the font file into the
same signed distance field a glyph server would have sent. For a glyph the font wants offset from
the pen, the offset is baked into the glyph's own `left` and `top` — which the glyph protocol buffer
carries as *signed* values. That is what puts a niqqud point under its letter rather than after it.

The two halves talk over a `BroadcastChannel`: the workers say what codepoints they have invented,
and the main thread asks them to stop allocating into a block of codepoints before it draws it.

## Layout

Rust, in layers, each crate depending only on the ones below it:

```
crates/
  text-domain/      types, constants, and the two traits the layers above depend on
  glyph-pbf/        writes the protocol buffer a `glyphs` URL serves
  sdf-rasterizer/   draws a glyph outline into a signed distance field
  font-set/         the fallback chain of font files, and which instance of each to read
  text-shaping/     bidirectional resolution, script itemization, HarfBuzz     (impls TextShaping)
  glyph-registry/   codepoint allocation and glyph assembly                    (generic over both)
  wasm-bindings/    the JavaScript face: wires the layers together
```

`glyph-registry` holds the whole trick and none of the craft — it names the two capabilities
(`TextShaping`, `GlyphRasterizing`) and knows nothing about HarfBuzz, fonts or distance fields.

TypeScript, as npm workspaces:

```
packages/
  protocol/                   what the two halves say to each other; depends on nothing
  glyph-provider/             the drawing half, on the main thread
  shaping-worker/             the shaping half, in MapLibre's workers
  wasm/                       the WebAssembly build (generated)
  maplibre-gl-rtl-harfbuzz/   the published package: wires it up and bundles both halves
```

## Size

| | raw | gzip |
| --- | --- | --- |
| this plugin's WebAssembly | 702 kB | 295 kB |
| its two JavaScript bundles | 20 kB | 7 kB |
| `mapbox-gl-rtl-text` 0.4.0, for comparison | 130 kB | 36 kB |
| HarfBuzz itself in C++ (`harfbuzzjs` 1.6.0) | 412 kB | 169 kB |

`mapbox-gl-rtl-text` is the small one because it does far less: bidirectional ordering and Arabic
joining, and no shaping, no fonts, no rasterizer. The fair comparison is the last row — HarfBuzz
compiled from C++, which is 169 kB gzipped and shapes and nothing else. This module is that plus the
bidirectional algorithm, Unicode script data, a font parser, a distance field rasterizer and a
protocol buffer writer, for 126 kB more. Rewriting it in C++ would not change the picture: the bulk
is HarfBuzz's own shaping tables and Unicode data, which are the same either way.

Nothing is inlined: the WebAssembly module is fetched once and shared by the page and every worker.

## Building

```sh
npm install
npm run build          # WebAssembly, then both bundles, then the debug page's copy
npm run fetch-fonts    # fonts for the debug page and fixtures for the tests
npm test               # cargo test --workspace, then tsc
```

Needs a Rust toolchain with the `wasm32-unknown-unknown` target, `wasm-pack`, and Node 22.18 or
newer — the build scripts are TypeScript and are run by `node` directly, with no build step of their
own.

The two files the build *produces* keep the `.mjs` extension on purpose. MapLibre decides how to
load a text plugin from its extension: a `.mjs` URL is loaded with a dynamic `import()`, and
anything else is fetched and run, which would lose the module context the worker half needs.

The rasterizer is checked against a real glyph server: `npm run fetch-fonts` fetches the font
`demotiles.maplibre.org` draws with and the block of glyphs it serves for it, and the test compares
every glyph of that block, metric by metric and pixel by pixel. The conventions of the glyph
protocol buffer — where `top` is measured from, whether bounds are rounded or truncated, whether an
advance is rounded or floored — are written down nowhere, and that test is how they were found.

## Licence

MIT. The Noto fonts the debug page fetches are Google's, under the SIL Open Font License, and are
not distributed here.
