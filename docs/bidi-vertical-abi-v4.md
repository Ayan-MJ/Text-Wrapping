# Bidirectional and vertical-writing contract (ABI v4)

ABI v4 separates paragraph direction from shaping-item direction and makes the
flow model axis-aware. The persisted paragraph fields are:

| Field | Values | Default for old requests |
| --- | --- | --- |
| `baseDirection` | `auto`, `leftToRight`, `rightToLeft` | `auto` |
| `writingMode` | `horizontalTb`, `verticalRl`, `verticalLr` | `horizontalTb` |
| `textOrientation` | `mixed`, `upright`, `sideways` | `mixed` |

`baseDirection:auto` is resolved for the complete paragraph with the Unicode
Bidirectional Algorithm. It is never inferred from the first rich-text run.
The deprecated style-run `direction` field is accepted for ABI-v3 source and
JSON compatibility but does not influence ABI-v4 shaping.

The shaper divides text at font/style, resolved bidi-level, script and vertical
orientation boundaries. Each shaped run and cluster reports `bidiLevel`,
`direction` and `orientation`. This keeps Latin and European numbers
internally LTR in an RTL paragraph while Arabic and Hebrew remain RTL. Visual
cluster order is computed from embedding levels for each wrapped fragment.
Trailing whitespace is reset to the resolved paragraph level before reordering.

For horizontal RTL paragraphs, available exclusion fragments are consumed from
right to left and each short fragment is aligned to its right edge. This applies
to final lines and to both corridors beside an image.

## Vertical behavior

Upright vertical shaping uses the font's native top-to-bottom advances and
OpenType `vert` and `vrt2` features. Under `textOrientation:mixed`, Unicode
Vertical_Orientation selects upright or sideways shaping items. Sideways items
are rotated individually by the host renderer; the engine never lays out a
horizontal paragraph and rotates the completed result.

The axis mapping is:

| Mode | Inline axis | Positive block progression |
| --- | --- | --- |
| `horizontalTb` | x | down |
| `verticalRl` | y | left |
| `verticalLr` | y | right |

Normalized object placement, maximum inline size, exclusion corridors,
centered-block fallback and pointer drag all use that mapping. A vertical
fallback blocks the full y inline extent for its x band, so text resumes in the
next column. Object dimensions stay physical width/height and preserve aspect
ratio when the inline dimension is capped.

Caret output is a vertical bar in horizontal mode and a horizontal bar in
vertical mode. Selection, hit-testing and arrow navigation operate in physical
page coordinates. In vertical mode Up/Down move within a column; Left/Right move
between columns, respecting RL or LR column progression. UTF-16 offsets,
affinity and stable paragraph IDs do not change with writing mode.

## ABI and JSON versions

- Binary C/WASM ABI: 4.
- Rich shaping response envelope: 2.
- Editor snapshot request/response: 2.
- Editor query request/response: 2.
- Legacy pre-shaped layout request/response: 1.

All ABI-v3 function symbols remain exported. The old
`ld_flow_normalized_placement_for_drag` remains horizontal-only.
`ld_flow_normalized_placement_for_drag_v4` accepts content x/y/width/height,
object width/height, the block-start anchor reference and a writing-mode value
(0 horizontal-TB, 1 vertical-RL, 2 vertical-LR).

Editor request v1 and query v1 remain accepted. Missing rich paragraph fields
receive the defaults in the table above. Hosts should nevertheless migrate to
request v2 so persisted intent is explicit.

## Host migration

Web:

1. Require `FLOW_ABI_VERSION === 4`.
2. Add `baseDirection`, `writingMode` and `textOrientation` to every rich
   paragraph and use `EditorSnapshotRequestV2`.
3. Stop assigning semantic meaning to `RichTextStyle.direction`.
4. Use `preferredInlineQ26_6` for movement and
   `normalizedPlacementForDragV4` for new object interactions.
5. Paint `PositionedGlyph` origins directly and rotate only glyphs whose
   `orientation` is `sideways`.

Swift:

1. Require `LastDraftFlowEngine.abiVersion == 4`.
2. Populate `CanonicalRichParagraph.baseDirection`, `writingMode` and
   `textOrientation`, then create `EditorSnapshotRequestV2`.
3. Use `preferredInlineQ26_6` and
   `normalizedPlacementForDragV4(...writingMode:)`.
4. Supply `EditorGeometrySnapshotData` to `LastDraftFlowView` or
   `LastDraftFlowCanvas`; the glyph callback receives the required per-glyph
   rotation.

Documents and anchors do not need an ID or UTF-16 migration. Old documents
decode as automatic horizontal text and can be rewritten with explicit fields
on their next normal save.
