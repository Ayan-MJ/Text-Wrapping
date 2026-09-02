# LastDraft Flow

`lastdraft-flow` is LastDraft's deterministic geometry engine for text flowing
around semantically anchored images. An image has no persisted document-space
position and no `wrapLeft` or `wrapRight` mode. It stores:

- a stable paragraph identifier and UTF-16 text offset;
- an inline position normalized over the available inline-axis travel;
- a signed block offset measured in 1/1024ths of the anchor line height;
- ideal dimensions and a responsive maximum width;
- exclusion margins and the minimum viable text-fragment width.

The engine resolves the object from its anchor for each layout, subtracts its
dynamic exclusion from every intersecting line band, and fills all viable
fragments in logical order. On narrow viewports it scales the object and, when
neither side remains useful, temporarily switches to a centered full-band
exclusion. The stored user placement is not changed, so the original placement
returns when space is available again.

## Status

This repository contains the buildable v1 flow core, portable persistence
contract, shared JSON/C ABI, Web TypeScript/WASM wrapper with Canvas integration,
and an Apple Swift Package with UIKit and SwiftUI integration. The original v1
request accepts host-shaped clusters; the new rich-text API shapes registered
font bytes inside the shared engine.

Development on the rich-text editor foundation is tracked in
[docs/rich-text-editor-foundation.md](docs/rich-text-editor-foundation.md). The
native Rust API registers content-addressed fonts, performs HarfBuzz-compatible
shaping, creates immutable positioned editor snapshots, and answers caret,
selection, hit-testing and visual-movement queries. ABI v4 exposes the same
snapshot through C/WASM, TypeScript and Swift while preserving the original v1
layout request. ABI v4 adds paragraph-level Unicode bidi resolution, automatic
directional shaping items, native horizontal/vertical writing modes and
axis-aware editor/object geometry. Platform input and accessibility adapters
remain outside this prototype.

The LastDraft Web and iOS projects under `/Users/ayanmukherjee/dev` were inspected
read-only and were not modified. Their string `blockId` conventions and
attachment-ID rendering rule are reflected by these standalone SDKs.

Implemented:

- 26.6 fixed-point geometry;
- semantic UTF-16 anchors;
- continuous two-dimensional local placement;
- rectangular dynamic exclusions;
- simultaneous flow through left and right fragments;
- stable exclusion-free anchor tracks;
- proportional responsive scaling;
- deterministic, reversible block fallback;
- multi-paragraph reflow and cross-paragraph exclusions;
- drag-coordinate normalization;
- input validation and layout safety limits;
- arbitrary LastDraft string block/object IDs;
- a tested raw WebAssembly memory bridge;
- a TypeScript Canvas renderer and pointer-drag controller;
- a Swift/C bridge, UIKit draggable renderer and SwiftUI wrapper;
- content-addressed font registration and shared rich-text shaping;
- shared caret, selection and point hit-testing geometry;
- page-coordinate glyph and cluster origins;
- visual left/right/up/down caret movement across wrapped fragments;
- paragraph `auto`/LTR/RTL base direction resolved with the Unicode
  Bidirectional Algorithm, independently of rich-text style runs;
- right-origin RTL lines, short final lines and two-sided image exclusions;
- native `verticalRl` and `verticalLr` shaping with vertical font metrics,
  `vert`/`vrt2`, Unicode vertical orientation and per-glyph sideways output;
- axis-aware vertical exclusions, normalized placement, fallback, drag,
  caret/selection/hit-test and arrow navigation;
- exclusion-boundary affinity on both sides of an image;
- shared OFL-font Unicode fixtures for italic transitions, ligatures, emoji,
  Hindi, Arabic, Hebrew, mixed English/numbers/punctuation and CJK vertical text;
- identical Web/WASM and Swift/C geometry fingerprints from committed fonts;
- an XCFramework for macOS arm64, iOS arm64 and universal iOS Simulator
  (arm64/x86_64).

The normative behavior and platform mapping are in
[`docs/lastdraft-flow-v1.md`](docs/lastdraft-flow-v1.md). The persisted object is
defined by
[`schema/anchored-flow-object-v1.schema.json`](schema/anchored-flow-object-v1.schema.json).
Host wiring is described in
[`docs/web-ios-integration.md`](docs/web-ios-integration.md).
ABI-v4 behavior and migration are documented in
[`docs/bidi-vertical-abi-v4.md`](docs/bidi-vertical-abi-v4.md).

## Build and test

```sh
cargo test
cargo clippy --all-targets -- -D warnings

cd web
npm install
npm run typecheck
npm test

cd ../apple
swift test
```

## Minimal use

```rust
use lastdraft_flow::{layout, LayoutRequest};

fn perform_layout(request: &LayoutRequest) {
    let result = layout(request).expect("valid LastDraft flow input");

    for paragraph in result.paragraphs {
        for line in paragraph.lines {
            for fragment in line.fragments {
                // Draw the fragment's shaped cluster range at fragment.rect.
            }
        }
    }
}
```

During a horizontal legacy drag, persist the output of
`normalized_placement_for_drag`. New clients use
`normalized_placement_for_drag_in_mode`. Never persist
`ResolvedObject.frame` or `anchor_reference_block_start`; both belong to one
specific layout.
