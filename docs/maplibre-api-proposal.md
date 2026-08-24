# What MapLibre GL JS would need for this to stop being a workaround

This plugin renders complex scripts and pointed right-to-left text on **unmodified MapLibre GL JS
6.5.0**. It does so through an interface that was never meant to carry it, and the way it gets
through is worth writing down — both because it shows what the current interface can and cannot
express, and because the shape of the workaround is a fair sketch of the API that would replace it.

Nothing here is needed for the plugin to work. It works now. What follows is what would make it
smaller, faster, and correct in the corners where it currently is not.

---

## 1. Where the current interface stops

Three facts about `main` as of 6.5.0.

**The text plugin interface is `string → string`.** `RTLTextPlugin` in
`src/source/rtl_text_plugin_worker.ts` is:

```ts
applyArabicShaping: (a: string) => string;
processBidirectionalText: (b: string, a: number[]) => string[];
processStyledBidirectionalText: (c: string, b: number[], a: number[]) => Array<[string, number[]]>;
```

A string cannot say *this glyph, from this font, a quarter of a pixel below the pen, taking no width
of its own*. So it cannot express a Hebrew niqqud point, an Arabic tashkeel mark, a Devanagari
conjunct, or a Tamil ligature. The name of the first function is a fair summary of its range: it was
built for Arabic joining, and Arabic joining happens to be expressible as a string because Unicode
carries presentation forms for it. Nothing else is.

**Layout is one codepoint, one glyph, one advance.** `shapeLines` in `src/symbol/shaping.ts`:

```ts
const codePoint = char.codePointAt(0);
const positionedGlyph = {glyph: codePoint, x, y: y + SHAPING_DEFAULT_OFFSET, …};
…
x += metrics.advance * section.scale + spacing;
```

There is no per-glyph offset, and glyph identity *is* the codepoint, all the way down through
`GlyphManager`, `GlyphAtlas` and `getGlyphQuads`.

**Two things are already right, and the workaround rests entirely on them.** In
`src/style/parse_glyph_pbf.ts` the `left` and `top` metrics are read with `readSVarint` — they are
**signed**. And `GlyphManager._downloadAndCacheRangePromise` computes `Math.floor(id / 256)` with no
upper bound, so codepoints above the basic plane are fetched like any other. Please keep both.

## 2. What the plugin does with that, and what it costs

Shaping runs in `applyArabicShaping`, because that is the one point where a glyph can be introduced
*before* `SymbolBucket.populate` collects the tile's glyph dependencies. Every glyph that no longer
stands for a character is given a codepoint from the supplementary private use areas, and the offset
the font asked for is baked into that glyph's signed `left` and `top`. The plugin then answers the
style's `glyphs` URL itself, drawing every glyph out of the font files with its own rasterizer.

It works, and it costs the following.

| Cost | Why |
| --- | --- |
| The style gives up its glyph server | Shaping and drawing must agree on the font file; a glyph index means nothing otherwise |
| A `BroadcastChannel` between the workers and the page | The worker half invents the codepoints and the main-thread half draws them, and MapLibre offers a plugin no channel between the two |
| A round trip before every block of shaped glyphs | MapLibre asks for a block once, so the block must be closed to further allocation before it is answered |
| ~131 000 shaped glyphs per page, divided between workers | The size of the two private use planes |
| `is-supported-script` still says no | `codePointRequiresComplexTextShaping` is a hard-coded regex that no plugin can affect |
| `text-letter-spacing` breaks clusters | Spacing is added between every glyph, including between a mark and its base |
| Vertical text is untouched | `charInComplexShapingScript` is Arabic-only, and vertical layout assumes one codepoint one glyph |

Every one of those, except the last two, disappears with the first proposal below.

---

## 3. The proposal, in the order I would do it

### 3.1 A shaping plugin that returns positioned glyphs

The heart of it. A plugin should be able to say what glyphs to draw and where, and to supply the
pictures for glyphs that no codepoint names.

```ts
/** One glyph, positioned by the font's own rules. */
export type ShapedGlyph = {
    /** Which font the plugin shaped with; meaningful only to the plugin. */
    fontId: number;
    /** The glyph's index in that font. Not a codepoint. */
    glyphId: number;
    /** Offset from the pen position, in the same 24 pixel em as `GlyphMetrics`. */
    dx: number;
    dy: number;
    /** How far the pen moves after this glyph. Zero for a mark. */
    advance: number;
    /** Where in the input this glyph came from, for `text-color` runs and hit testing. */
    cluster: number;
};

export interface TextShapingPlugin {
    /** Shapes one run of one section. Returns glyphs in logical order. */
    shape(text: string, fontStack: string): ShapedGlyph[];

    /**
     * Puts shaped glyphs into the order they are read in, once the line breaks are known.
     * Separate from `shape` for the same reason it is separate today: the bidirectional
     * algorithm resolves the order of a *line*, not of a paragraph.
     */
    reorder(glyphs: ShapedGlyph[], lineBreaks: number[]): ShapedGlyph[][];

    /** The picture for a glyph, in the same form `parseGlyphPbf` produces. */
    getGlyph(fontId: number, glyphId: number): StyleGlyph | Promise<StyleGlyph>;
}
```

What it touches inside MapLibre, roughly:

- **`src/symbol/shaping.ts`** — `shapeLines` stops computing `x` and instead reads `dx`, `dy` and
  `advance` off the shaped glyph. `PositionedGlyph` already carries `x` and `y`, so this is mostly
  *removing* arithmetic. This is the only change that matters for correctness; everything else is
  plumbing.
- **`src/data/bucket/symbol_bucket.ts`** — `calculateGlyphDependencies` collects
  `{fontId, glyphId}` rather than codepoints.
- **`src/render/glyph_manager.ts`** and **`src/render/glyph_atlas.ts`** — key glyphs by
  `(fontId, glyphId)` rather than by codepoint, and ask the plugin for a glyph the `glyphs` URL and
  the local fallbacks do not have. The existing codepoint path stays exactly as it is for styles
  with no plugin.
- **`src/source/rtl_text_plugin_worker.ts`** — the new interface alongside the old one, so that
  `mapbox-gl-rtl-text` keeps working.

Two smaller notes on the same interface:

- **`shape` needs the fontstack.** `applyArabicShaping` is not given one today, and a shaper cannot
  shape without knowing the font. This plugin works around it by owning every font in the style.
- **`getGlyph` should be allowed to be async.** `GlyphManager.getGlyphs` is already async
  throughout, so this costs nothing and lets a plugin fetch or rasterize lazily.

### 3.2 Let a plugin say which scripts it supports

`isStringInSupportedScript` decides whether a style's `["is-supported-script", …]` expression sees a
name as renderable, which is how styles choose between `name` and `name:latin`. It asks
`codePointRequiresComplexTextShaping`, a generated regex over `U+0900–0DFF`, `U+0F00–109F` and
`U+1780–17FF`, and nothing a plugin does can change the answer. With this plugin loaded, Devanagari
renders correctly and `is-supported-script` still says it does not — so a style that asks politely
gets the Latin name instead of the correct one.

The same list decides when a deferred plugin is lazily loaded, through `stringContainsRTLText`: a
deferred plugin is never loaded for Devanagari, because Devanagari is not right to left. (This
plugin therefore has to be registered eagerly.)

Something as small as this would settle both:

```ts
export interface TextShapingPlugin {
    /** ISO 15924 codes this plugin can shape, or `'*'` for anything the fonts cover. */
    supportedScripts?: string[] | '*';
}
```

### 3.3 A channel between a plugin's two halves

Shaping happens in the workers; glyph requests are answered on the main thread. A plugin that needs
to say anything to itself across that line has no way to, so this one uses a `BroadcastChannel` —
which works, and which no plugin should have to invent. Either of these would do:

- a documented `MessageType` a plugin may send and receive through the existing `Actor`; or
- passing the plugin an object with `postMessage`/`onmessage` when it registers.

### 3.4 A way to invalidate a glyph range

`GlyphManager` records that it has asked for a range and never asks again. That is right for a
static glyph server and wrong for a generated one: a glyph that becomes needed after its block was
fetched can never be drawn. This plugin deals with it by making the main thread ask every worker to
stop allocating into a block before answering for it — a round trip on every block of shaped glyphs.

`GlyphManager.invalidateRange(fontStack, range)`, or letting a protocol handler mark a response as
not-final, would remove that entirely.

### 3.5 Do not add letter spacing inside a cluster

`shapeLines` adds `spacing` after every glyph. Between a mark and the letter it belongs to, that is
simply wrong — the mark drifts away from its base as `text-letter-spacing` grows.
`charAllowsLetterSpacing` already excludes cursive scripts; a glyph with a zero advance is a mark by
definition, and skipping spacing after one would fix it without any new API.

---

## 4. What I would not change

**The two-call shape of the text plugin.** Shaping before line breaking and reordering after is not
an accident of the old plugin; it is what the Unicode Bidirectional Algorithm requires. Keep it.

**Signed `left` and `top` in the glyph protocol buffer, and unbounded codepoints in
`GlyphManager`.** They are what make the current workaround possible, and they are worth a test each
so that a future tidy-up does not take them away.

**The `glyphs` URL going through `addProtocol`.** Being able to answer glyph requests from the page
is what lets a plugin draw from real font files at all. It is a good API.
