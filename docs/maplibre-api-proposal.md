# What MapLibre GL JS would need for this to stop being a workaround

This plugin renders complex scripts and pointed right-to-left text on **unmodified MapLibre GL JS
6.5.0**. It does so through an interface that was never meant to carry it, and the way it gets
through is worth writing down — both because it shows what the current interface can and cannot
express, and because the shape of the workaround is a fair sketch of the API that would replace it.

Nothing here is needed for the plugin to work. It works now. What follows is what would make it
smaller, faster, and correct in the corners where it currently is not — and what would let it stop
doing two things it should not be doing.

---

## 1. Where the current interface stops

Three facts about `main` as of 6.5.0.

**The text plugin interface is `string → string`.** `RTLTextPlugin` in
`src/source/rtl_text_plugin_worker.ts` is:

```ts
applyArabicShaping: (a: string) => string;
processBidirectionalText: ((b: string, a: number[]) => string[]);
processStyledBidirectionalText: ((c: string, b: number[], a: number[]) => Array<[string, number[]]>);
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
the font asked for is baked into that glyph's signed `left` and `top`.

That much is a fair use of the interface as it stands. Two further things are not, and are marked
below as what they are.

| Cost | Why | |
| --- | --- | --- |
| Glyphs are served through `addProtocol`, and the style gives up its `glyphs` URL | The plugin has no other way to hand MapLibre a picture for a glyph it invented | **transitional — §3.2** |
| A message type squeezed into MapLibre's worker protocol with a cast | The protocol is reachable from both halves, but its message list is closed | **transitional — §3.4** |
| A round trip before every block of shaped glyphs | MapLibre asks for a block of 256 once, so the block must be closed to further allocation before it is answered | goes away with §3.2 |
| ~131 000 shaped glyphs per page, divided between workers | The size of the two private use planes | goes away with §3.2 |
| `is-supported-script` still says no | `codePointRequiresComplexTextShaping` is a hard-coded regex that no plugin can affect | §3.3 |
| `text-letter-spacing` breaks clusters | Spacing is added between every glyph, including between a mark and its base | §3.5 |
| Vertical text is untouched | Vertical layout assumes one codepoint is one glyph, and `charInComplexShapingScript` is Arabic-only | out of scope here |

**The two transitional ones are meant to go.** Answering the `glyphs` URL from a protocol handler is
not what `addProtocol` is for, and reaching into the worker protocol with a cast is not something a
plugin should have to do. Both are in this plugin because there is no alternative today; if §3.2 and
§3.4 land, both come out, and the plugin stops touching anything it was not offered.

---

## 3. The proposal

### 3.1 One plugin interface, covering both kinds of plugin

The heart of it — and it should be a *single* interface, not a second one beside the first.
`mapbox-gl-rtl-text` must keep working untouched, and a plugin should be able to offer both: full
shaping where MapLibre supports it, and the old string methods as a fallback where it does not. That
falls out naturally if every method is optional and MapLibre chooses a path by what it was given.

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
    /** Where in the input this glyph came from, for style sections and hit testing. */
    cluster: number;
};

export interface TextPlugin {
    // ---- What a plugin can do today. `mapbox-gl-rtl-text` provides exactly these three. ----
    applyArabicShaping?(text: string): string;
    processBidirectionalText?(text: string, lineBreaks: number[]): string[];
    processStyledBidirectionalText?(
        text: string, sections: number[], lineBreaks: number[],
    ): Array<[string, number[]]>;

    // ---- What a shaping plugin provides instead. ----

    /** Shapes one run of one section. Returns glyphs in logical order. */
    shapeText?(text: string, fontStack: string): ShapedGlyph[];

    /**
     * Puts shaped glyphs into the order they are read in, once the line breaks are known.
     * Separate from `shapeText` for the same reason it is separate today: the bidirectional
     * algorithm resolves the order of a *line*, not of a paragraph.
     */
    reorderShapedText?(glyphs: ShapedGlyph[], lineBreaks: number[]): ShapedGlyph[][];

    /** The picture for a glyph, in the same form `parseGlyphPbf` produces. */
    getGlyph?(fontId: number, glyphId: number): StyleGlyph | Promise<StyleGlyph>;

    /** ISO 15924 codes this plugin can shape, or `'*'` for anything its fonts cover. See §3.3. */
    supportedScripts?: string[] | '*';
}
```

MapLibre takes the shaping path when `shapeText`, `reorderShapedText` and `getGlyph` are all there,
and the string path when the three older methods are. `RTLWorkerPlugin.isParsed()` becomes "has one
complete set or the other" instead of "has all three of these", and `setMethods` keeps whichever it
was given. A plugin that ships both sets works on every MapLibre version, which is what any plugin
that wants a user base is going to want to do.

Two notes on the interface itself:

- **`shapeText` needs the fontstack.** `applyArabicShaping` is not given one, and a shaper cannot
  shape without knowing the font. This plugin works around it by owning every font in the style,
  which is precisely the arrangement §3.2 would let it stop insisting on.
- **`getGlyph` should be allowed to be async.** `GlyphManager.getGlyphs` is already async
  throughout, so this costs nothing and lets a plugin fetch or rasterize lazily.

Where it lands inside MapLibre, roughly:

- **`src/symbol/shaping.ts`** — `shapeLines` stops computing `x` and reads `dx`, `dy` and `advance`
  off the shaped glyph. `PositionedGlyph` already carries `x` and `y`, so this is mostly *removing*
  arithmetic. It is the only change that matters for correctness; the rest is plumbing. If it is
  easier to put a hook at the point where a line is split into units for layout rather than to
  thread glyphs through `TaggedString`, that works just as well — what matters is that the unit
  stops being a codepoint.
- **`src/data/bucket/symbol_bucket.ts`** — `calculateGlyphDependencies` collects
  `{fontId, glyphId}` rather than codepoints.
- **`src/render/glyph_manager.ts`** and **`src/render/glyph_atlas.ts`** — key glyphs by
  `(fontId, glyphId)` rather than by codepoint. The existing codepoint path stays exactly as it is
  for styles with no plugin.

### 3.2 Glyphs from the plugin, not from a protocol handler

`getGlyph` above is small in the interface and large in what it removes, so it is worth stating on
its own.

Today a plugin that invents a glyph has no way to say what it looks like. This one answers the
style's `glyphs` URL through `addProtocol` instead — which works, but it is not what `addProtocol` is
for, and it costs the style its glyph server, since the plugin then has to serve *every* glyph in
every fontstack rather than only the ones it invented.

With `getGlyph`, `GlyphManager` asks the plugin for the glyphs the plugin invented and the `glyphs`
URL for everything else, and three of the costs in §2 disappear at once:

- the style keeps its glyph server, and the plugin's fonts only have to cover the scripts it shapes;
- there are no private-use codepoints, so no pool to run out of;
- there are no 256-codepoint blocks to seal, because glyphs are asked for one at a time rather than
  a block at a time — which removes the round trip *and* the reason for §3.4's message.

That last point is worth spelling out: `GlyphManager` records that it has asked for a block and
never asks again, which is right for a static glyph server and wrong for a generated one. A glyph
that becomes needed after its block was fetched can never be drawn. This plugin deals with it by
asking every worker to stop allocating into a block before answering for it. Per-glyph requests make
the whole question moot.

### 3.3 Let a plugin say which scripts it supports

`isStringInSupportedScript` decides whether a style's `["is-supported-script", …]` expression sees a
name as renderable, which is how styles choose between `name` and `name:latin`. It asks
`codePointRequiresComplexTextShaping`, a generated regex over `U+0900–0DFF`, `U+0F00–109F` and
`U+1780–17FF`, and nothing a plugin does can change the answer. With this plugin loaded, Devanagari
renders correctly and `is-supported-script` still says it does not — so a style that asks politely
gets the Latin name instead of the correct one.

The same list decides when a deferred plugin is lazily loaded, through `stringContainsRTLText`: a
deferred plugin is never loaded for Devanagari, because Devanagari is not right to left. (This
plugin therefore has to be registered eagerly.)

`supportedScripts` on the plugin interface settles both.

### 3.4 A general-purpose message on the worker protocol

Smaller than it looked. There is already a channel, and both halves of a plugin can reach it:
`getGlobalDispatcher()` on the page, `self.worker.actor` inside a worker, both public. What is not
reachable is the *type*: `MessageType` is a `const enum` and `RequestResponseMessageMap` is a closed
mapped type, so a message MapLibre does not already know about cannot be named without widening
them.

So this plugin widens them, in one file, and uses the channel as it stands. It works — an
unregistered message type is answered with `null` rather than an error, so borrowing the channel
cannot disturb MapLibre's own traffic — but every plugin that needs to talk to itself will write the
same cast.

Something like `MessageType.plugin = 'PL'` with `[MessageType.plugin]: [unknown, unknown]` in the
map, or an escape hatch for namespaced types, would remove the cast. Worth having on its own
merits — though note that if §3.2 lands, this plugin no longer needs to say anything to itself at
all, because there is nothing left to coordinate.

### 3.5 Do not add letter spacing inside a cluster

`shapeLines` adds `spacing` after every glyph. Between a mark and the letter it belongs to, that is
simply wrong — the mark drifts away from its base as `text-letter-spacing` grows.
`charAllowsLetterSpacing` already excludes cursive scripts; a glyph with a zero advance is a mark by
definition, and skipping spacing after one would fix it with no new API at all.

---

## 4. What I would not change

**The two-call shape of the text plugin.** Shaping before line breaking and reordering after is not
an accident of the old plugin; it is what the Unicode Bidirectional Algorithm requires. Keep it, and
keep it in the new methods too.

**Signed `left` and `top` in the glyph protocol buffer, and unbounded codepoints in
`GlyphManager`.** They are what make the current workaround possible, and they are worth a test each
so that a future tidy-up does not quietly take them away.

**`addProtocol` itself.** It is a good API. It is simply not the right one for this, and this plugin
should stop using it for this the moment §3.2 lands.
