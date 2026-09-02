# Rich-text editor foundation

This branch begins the transition from a plain flow renderer to a shared
rich-text editor engine. It does not yet replace TipTap or UITextView.

## Shared ownership

The Rust core now owns:

- immutable font registration with SHA-256 identities;
- HarfBuzz-compatible OpenType shaping through Rustybuzz;
- fixed-point glyph advances and offsets;
- rich UTF-16 style runs, OpenType features and variable-font axes;
- paragraph-level UBA base direction and automatic directional/script items;
- native horizontal, vertical-RL and vertical-LR shaping with vertical metrics;
- Unicode line-break opportunities and grapheme-safe caret stops;
- caret rectangles, selection rectangles and point-to-text hit testing;
- page-coordinate glyph origins, cluster rectangles and visual caret stops;
- visual left/right/up/down movement with a sticky inline-axis coordinate;
- distinct upstream/downstream carets at the two sides of an exclusion gap;
- conversion of canonical shaped clusters into the existing image-exclusion
  flow algorithm.

Font IDs are not family names. Every Web and iOS client must register the same
font bytes, face index, features and variations. A document font manifest must
carry the returned SHA-256 value so substitution is detected rather than
silently changing layout.

## Platform ownership

Web remains responsible for browser keyboard and IME events, clipboard MIME
integration, a semantic DOM accessibility mirror, glyph rasterization and
selection presentation.

iOS remains responsible for UITextInput, UITextInteraction, marked text,
UIPasteboard, Core Graphics glyph rasterization and UIKit accessibility
elements.

The TypeScript and Swift packages contain matching rich-text, positioned-glyph,
cluster, caret, selection and movement models. Both wrappers register exact
font bytes and create immutable editor snapshots through the same stateful Rust
registry. ABI v4 exposes snapshot creation and tagged geometry queries through
C and WASM. Snapshot creation owns shaping and flow layout as one operation, so
hosts cannot accidentally query geometry from mismatched shaped/layout inputs.

The shared fixture at `fixtures/editor/unicode-exclusion-v1.json` uses the
committed OFL fonts in `fixtures/fonts`. Native Rust, Web/WASM and Swift/C tests
all execute this request. It covers regular/italic transitions, `fi` ligatures,
emoji surrogate pairs, Hindi, mixed Arabic/English directional runs, clicks on
both sides of the exclusion and a selection spanning multiple fragments.

The ABI-v4 fixtures at `fixtures/editor/bidi-horizontal-v2.json` and
`fixtures/editor/vertical-v2.json` add Arabic, Hebrew, English, numbers,
punctuation, CJK, vertical Latin orientation and image exclusions on both axes.
Web/WASM and Swift/C tests hash the same complete fragment, object, cluster,
glyph and caret projection, proving matching fixed-point output.

## Current boundary

The original v1 JSON layout request remains compatible and can still provide
pre-shaped horizontal LTR cluster advances. Rich editor snapshot request v1 is
also accepted and defaults missing paragraph fields to
`auto`/`horizontalTb`/`mixed`. ABI-v4 clients should send editor request
v2 explicitly. Font fallback remains host-selected through rich-text font runs.
Copy/paste adapters, IME hosts and accessibility mirrors belong in LastDraft
and must land before replacing TipTap. The prototype's UIKit, SwiftUI and Canvas
adapters consume engine glyph geometry but do not implement those input-system
responsibilities.
