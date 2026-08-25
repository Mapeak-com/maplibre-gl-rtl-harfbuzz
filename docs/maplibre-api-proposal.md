# A text shaping API for MapLibre GL JS

A proposal for one plugin interface that can carry complex text shaping, with the existing
right-to-left plugin adapted onto it rather than kept beside it.

---

## 1. The gap

**The text plugin interface is `string → string`.** `RTLTextPlugin` in
`src/source/rtl_text_plugin_worker.ts`:

```ts
applyArabicShaping: (a: string) => string;
processBidirectionalText: ((b: string, a: number[]) => string[]);
processStyledBidirectionalText: ((c: string, b: number[], a: number[]) => Array<[string, number[]]>);
```

A string cannot say *this glyph, from this font, a quarter of a pixel below the pen, taking no width
of its own*. So it cannot express a Hebrew niqqud point, an Arabic tashkeel mark, a Devanagari
conjunct, or a Tamil ligature. Arabic joining is the exception rather than the rule: it is
expressible as a string only because Unicode carries presentation forms for it.

**Layout is one codepoint, one glyph, one advance.** `shapeLines` in `src/symbol/shaping.ts`:

```ts
const codePoint = char.codePointAt(0);
const positionedGlyph = {glyph: codePoint, x, y: y + SHAPING_DEFAULT_OFFSET, …};
…
x += metrics.advance * section.scale + spacing;
```

There is no per-glyph offset, and glyph identity *is* the codepoint, through `GlyphManager`,
`GlyphAtlas` and `getGlyphQuads`.

## 2. The interface

```ts
/** One glyph, positioned by the font's own rules. */
export type ShapedGlyph = {
    /**
     * Which font the plugin shaped with; meaningful only to the plugin, and resolved by its
     * `getGlyph`. Omitted for a glyph MapLibre should draw the way it always has, in which case
     * `glyphId` is a codepoint and the `glyphs` URL and local fallbacks answer for it.
     */
    fontId?: number;
    /** The glyph's index in that font, or a codepoint when there is no `fontId`. */
    glyphId: number;
    /** Offset from the pen position, in the same 24 pixel em as `GlyphMetrics`. Defaults to zero. */
    dx?: number;
    dy?: number;
    /** How far the pen moves after this glyph; zero for a mark. Defaults to the glyph's own advance. */
    advance?: number;
    /** Which style section this glyph belongs to, for its font, scale and colour. */
    sectionIndex: number;
};

export interface TextPlugin {
    /**
     * Shapes one section of a label, in logical order.
     *
     * The fontstack is passed because a shaper cannot shape without knowing the font it is shaping
     * for. `sectionIndex` is passed so that it can be stamped onto the glyphs, which is what lets
     * `reorderShapedText` move them around without losing what they belong to.
     */
    shapeText(text: string, fontStack: string, sectionIndex: number): ShapedGlyph[];

    /**
     * Puts a label's glyphs into the order they are read in, once the line breaks are known.
     *
     * Separate from `shapeText` for the same reason the two calls are separate today: the
     * bidirectional algorithm resolves the order of a *line*, not of a paragraph, so the same words
     * come out in a different order depending on where the line ends. Line breaks are in UTF-16 code
     * units of the shaped text, as they are today.
     */
    reorderShapedText(glyphs: readonly ShapedGlyph[], lineBreaks: number[]): ShapedGlyph[][];

    /**
     * The picture for a glyph the plugin shaped, in the form `parseGlyphPbf` produces. Only called
     * for glyphs that carry a `fontId`. Allowed to be async, since `GlyphManager.getGlyphs` already
     * is throughout.
     */
    getGlyph?(fontId: number, glyphId: number): StyleGlyph | Promise<StyleGlyph>;

    /** ISO 15924 codes this plugin can shape, or `'*'` for anything its fonts cover. See §5. */
    supportedScripts?: string[] | '*';
}
```

Three of those fields are optional for a reason worth stating on its own: **a shaped glyph must be
able to describe an ordinary one.** A glyph with no `fontId`, no offset and no advance is exactly
what MapLibre draws today — a codepoint, at its own advance, on the baseline. That is what makes §3
possible, and it also means a plugin only has to say what it is actually changing.

## 3. Adapting the existing plugin rather than keeping two interfaces

`setRTLTextPlugin` should go on accepting a plugin that implements the three string methods, but
MapLibre's pipeline should only ever see one interface. The old shape survives as an adapter, not as
a second branch through `shaping.ts`.

It adapts cleanly because for such a plugin **a glyph is its codepoint**, so nothing is lost either
way across the boundary:

```ts
function fromRTLTextPlugin(plugin: RTLTextPlugin): TextPlugin {
    return {
        shapeText(text, _fontStack, sectionIndex) {
            return [...plugin.applyArabicShaping(text)].map((char) => ({
                glyphId: char.codePointAt(0)!,
                sectionIndex,
            }));
        },

        reorderShapedText(glyphs, lineBreaks) {
            const text = String.fromCodePoint(...glyphs.map((glyph) => glyph.glyphId));
            const sections = glyphs.flatMap((glyph) =>
                glyph.glyphId > 0xffff ? [glyph.sectionIndex, glyph.sectionIndex] : [glyph.sectionIndex],
            );

            return plugin
                .processStyledBidirectionalText(text, sections, lineBreaks)
                .map(([line, lineSections]) => {
                    let codeUnit = 0;
                    return [...line].map((char) => {
                        const glyph = {glyphId: char.codePointAt(0)!, sectionIndex: lineSections[codeUnit]};
                        codeUnit += char.length;
                        return glyph;
                    });
                });
        },
    };
}
```

`getGlyph` is absent, so every glyph falls through to the `glyphs` URL and the local fallbacks, which
is where they come from today. `processBidirectionalText` is the single-section case of
`processStyledBidirectionalText` and needs no separate treatment.

That adapter is written out in full because it is the load-bearing claim here, and it has been run:
against a plugin implementing the three string methods, `shapeText` followed by
`reorderShapedText` produces exactly what calling the string methods directly produces — same
codepoints, same per-code-unit section indices, same lines — for right-to-left text, mixed
direction, three nested bidirectional levels, line breaks, and text whose codepoints are outside the
basic plane. (In real code the two `String.fromCodePoint(...)` spreads want chunking; a long label
will otherwise overflow the argument limit.)

The only thing the adapter cannot recover is which *input* character a glyph came from, since the
string methods do not report it. Nothing in MapLibre needs that today, which is why `ShapedGlyph`
carries `sectionIndex` rather than a cluster index. If a cluster index is ever wanted, it should be
optional, and the adapter should leave it out.

`RTLWorkerPlugin.setMethods` becomes: wrap what it was given if it looks like the old interface,
keep it as it is if it looks like the new one. `isParsed()` asks whether it has a `TextPlugin`.

## 4. Where it lands

- **`src/symbol/shaping.ts`** — `shapeLines` stops computing `x` and reads `dx`, `dy` and `advance`
  off the glyph, defaulting as §2 describes. `PositionedGlyph` already carries `x` and `y`, so this
  is mostly *removing* arithmetic. It is the only change that matters for correctness. If a hook at
  the point where a line is split into units for layout is easier than threading glyphs through
  `TaggedString`, that does just as well — what matters is that the unit stops being a codepoint.
- **`src/data/bucket/symbol_bucket.ts`** — `calculateGlyphDependencies` collects
  `{fontId, glyphId}` rather than codepoints.
- **`src/render/glyph_manager.ts`**, **`src/render/glyph_atlas.ts`** — key glyphs by
  `(fontId, glyphId)`, and ask the plugin's `getGlyph` for the ones that carry a `fontId`. Glyphs
  with no `fontId` take the path they take now, so a style with no plugin is untouched.
- **`src/source/rtl_text_plugin_worker.ts`** — the adapter of §3.

## 5. Two smaller things

**Let a plugin say which scripts it supports.** `isStringInSupportedScript` decides whether a style's
`["is-supported-script", …]` expression sees a name as renderable, which is how styles choose between
`name` and `name:latin`. It asks `codePointRequiresComplexTextShaping`, a generated regex over
`U+0900–0DFF`, `U+0F00–109F` and `U+1780–17FF`, and nothing a plugin does can change the answer — so
a plugin that shapes Devanagari correctly still gets the Latin name. The same list decides when a
deferred plugin is lazily loaded, through `stringContainsRTLText`, which means a deferred plugin is
never loaded for Devanagari at all, because Devanagari is not right to left. `supportedScripts`
settles both.

**Do not add letter spacing inside a cluster.** `shapeLines` adds `spacing` after every glyph.
Between a mark and the letter it belongs to that is simply wrong: the mark drifts away from its base
as `text-letter-spacing` grows. `charAllowsLetterSpacing` already excludes cursive scripts, and a
glyph with a zero advance is a mark by definition, so skipping spacing after one would fix it with no
new API at all.

## 6. One thing to keep

**The two calls.** Shaping before line breaking and reordering after is not an accident of the
current plugin; it is what the Unicode Bidirectional Algorithm requires. Keep the split in the new
methods too.

## 7. A cheaper alternative, and where it stops

Before adopting any of the above it is worth knowing that a large part of the problem can be solved
without a shaping engine at all, because the browser already has one. The numbers below are measured
rather than estimated: Chromium, Firefox and WebKit, the fonts pinned with `FontFace`, at 48px.

MapLibre cannot ask the browser for *positioned glyphs* — there is no API for glyph ids, per-glyph
positions, or the visual order the bidirectional algorithm resolved. But it does not have to work in
glyphs. It could work in **grapheme clusters**, which `Intl.Segmenter` does give it, drawing each
cluster with `fillText` and measuring it with `measureText`. All three engines segment identically
(`दिल्ली → दि | ल्ली`, `ភ្នំពេញ → ភ្នំ | ពេ | ញ`, `שְׁדֵרוֹת → שְׁ | דֵ | ר | וֹ | ת`).

Comparing a string drawn cluster by cluster against the same string drawn whole, as a fraction of
inked pixels that disagree:

| | per cluster | |
| --- | --- | --- |
| Hebrew with niqqud | 0.0% | correct |
| Devanagari | 0.0% | correct |
| Khmer | 0.0% | correct |
| Thai | 0.0% | correct |
| Bengali | 0.7% | correct |
| Tamil | 25% | **wrong** — the ligature spans two clusters |
| Arabic | 72–82% | **wrong** — joining spans every cluster boundary |

Zero-width joiners around each cluster, to stand in for its neighbours, do not rescue Arabic: 70–83%,
no better than without them.

Two objections that turn out not to hold, and are worth retiring:

- **It is not engine-dependent.** With the font pinned, the three engines measure the same string to
  within 0.02px — 0.01%. Earlier differences of up to 31% were entirely CSS font *matching* resolving
  differently per engine, not shaping.
- **It works where MapLibre needs it.** `OffscreenCanvas` plus `FontFace` in a worker measures
  correctly in all three engines, so this can run in the same place shaping runs today.

So a cluster-based path inside MapLibre — iterate `Intl.Segmenter` clusters instead of codepoints,
and let `GlyphManager` rasterize a cluster rather than a codepoint — is a real option, and a much
smaller change than §2. It reuses the canvas rasterization already there for
`localIdeographFontFamily`, needs no new dependency, and would render Hebrew with niqqud, Devanagari,
Bengali, Khmer and Thai correctly.

Where it stops:

- **Cursive scripts.** Arabic, Syriac, N'Ko, Mongolian: joining crosses cluster boundaries, and no
  arrangement of clusters reproduces it. Arabic would stay at today's quality, via the presentation
  forms the existing plugin substitutes — so not a regression, but not a fix either.
- **Tamil**, and any script whose ligatures cross clusters.
- **Ordering.** There is no bidirectional algorithm in the platform, so ordering still has to come
  from a plugin or from an implementation in MapLibre.
- **Font pinning is not optional.** The determinism above holds only because the font was pinned;
  without it, engines diverge by up to a third of the line width.
- **Atlas granularity.** One entry per distinct cluster rather than per codepoint, which for
  Devanagari or Khmer is a much larger set.

A shaping engine earns its place on the first two of those, on giving positions per glyph rather than
per cluster — which is what text along a curve wants — and on running the same way with no canvas at
all, which matters for headless and server-side rendering. It is not needed for the rest, and if the
cluster path is the one that ships first, it fixes more scripts per line of code than anything else
here.

---

*Aside, unrelated to shaping: `MessageType` is a `const enum` and `RequestResponseMessageMap` a
closed mapped type, so a plugin with two halves cannot name a message of its own on the worker
protocol without widening both. A `MessageType.plugin`, or an escape hatch for namespaced types,
would save every such plugin the same cast.*
