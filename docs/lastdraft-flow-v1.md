# LastDraft anchored flow contract v1

## 1. Purpose and boundary

This contract defines how Web, Android, iOS and Windows represent and lay out a
freely positioned image anchored to text. It covers flow geometry. Text shaping,
glyph rasterization, input methods, document editing and accessibility remain
platform integration responsibilities.

The Rust core is the normative implementation. A conforming platform adapter
must provide the same shaped cluster advances and must consume the core's line
fragments rather than independently asking its native text system to wrap the
paragraph.

## 2. Coordinate system

- All persisted dimensions are signed 26.6 fixed-point logical pixels: one DIP
  is 64 layout units.
- A logical pixel is one CSS pixel on Web, one Android `dp`, one iOS point and
  one Windows device-independent pixel.
- Layout uses a start-to-end inline axis and top-to-bottom block axis. A renderer
  maps the inline axis for the paragraph writing direction.
- All persisted text offsets are UTF-16 code-unit offsets. Anchors must occur on
  a canonical grapheme-cluster boundary.
- Paragraph and object IDs are non-empty UTF-8 strings. This directly accepts
  LastDraft's existing ProseMirror `attrs.blockId` and UUID-derived float IDs.

## 3. Semantic anchor

An anchor is `(paragraphId, utf16Offset, affinity)`.

The paragraph ID identifies a durable LastDraft document node; its position in
the document is not an identity. Insertions, deletions and block moves update
the anchor through the document model before layout. `affinity` resolves an
offset exactly at a soft line boundary.

For each paragraph, the engine creates an **anchor track** by laying out its
clusters at the current content width without objects anchored to that
paragraph. The object's reference line comes from this track. Final visible
text is then laid out with every active exclusion. This separation prevents an
object's own exclusion from moving its anchor, which could otherwise oscillate
between lines.

An object may affect its anchor paragraph and later text. It never reaches back
into an earlier semantic paragraph, even if the stored block offset is negative.

## 4. User-driven placement

There is intentionally no left/right placement enum.

`inlineU16` is an unsigned normalized position over the object's current
horizontal travel:

```text
travel = contentWidth - resolvedObjectWidth
x = contentStart + round(travel * inlineU16 / 65535)
```

Therefore `0` aligns the image to the start edge, `65535` aligns it to the end
edge and every intermediate value represents a continuous dragged position.
The value describes placement, not wrapping preference.

`blockOffset1024` is a signed offset from the top of the anchor reference line:

```text
y = anchorReferenceTop
    + round(lineHeight * blockOffset1024 / 1024)
```

Platform drag handlers call `normalized_placement_for_drag` with the pointer's
proposed object origin. Only the returned normalized placement is persisted.

## 5. Dynamic exclusion and line flow

V1 exclusions are image bounds expanded by logical start/top/end/bottom margins.
For each line band the engine:

1. selects vertically intersecting exclusions;
2. clips them to the content inline range;
3. sorts and unions the blocked intervals;
4. subtracts that union from the content interval;
5. removes corridors narrower than the active minimum fragment width;
6. fills every remaining fragment in logical start-to-end order;
7. continues on the next line after all fragments have been considered.

This naturally permits text on both sides of an image when both sides have
space. Moving the image continuously changes the two fragment widths. No side
mode is inferred or stored.

A word that fits the full content width is not split merely to enter a narrow
corridor. The engine tries a wider fragment or waits until a later line clears
the exclusion. Only an unbreakable sequence wider than the full content width
receives an emergency grapheme break.

## 6. Responsive behavior

Responsive layout never mutates persisted placement.

### 6.1 Size resolution

```text
widthCap = round(contentWidth * maxInlineU16 / 65535)
resolvedWidth = min(idealWidth, widthCap, contentWidth)
resolvedHeight = round(idealHeight * resolvedWidth / idealWidth)
```

Aspect ratio is preserved. Re-expanding the viewport restores the ideal size.

### 6.2 Placement attempt

The engine applies the stored inline and block placement to the resolved size,
then computes the exclusion margins and the two remaining inline corridors.

### 6.3 Automatic block fallback

If neither corridor is at least `minimumFragmentWidth`, the layout instance uses
`BlockFallback`:

- center the resolved image horizontally;
- expand its exclusion across the complete content width;
- keep its anchor-relative vertical position;
- resume text below the exclusion band.

The fallback mode is output state, not document state. When the viewport widens,
the original normalized inline placement is evaluated again and ordinary
two-sided flow returns automatically.

This rule is deterministic and does not depend on platform, pointer history,
previous viewport size or a left/right heuristic.

## 7. Multi-object ordering

Objects anchored to the same paragraph are resolved in ascending stable object
ID order. Exclusion intervals are sorted geometrically before union, so the
layout is independent of input array order. Overlapping objects create the union
of their exclusion rectangles.

## 8. Platform mapping

| Contract concept | Web | Android | iOS | Windows |
|---|---|---|---|---|
| Logical unit | CSS px × 64 | dp × 64 | point × 64 | DIP × 64 |
| Text offset | JS UTF-16 index | Kotlin/JVM UTF-16 index | `String.UTF16View`/`NSRange` | .NET UTF-16 index |
| Stable ID | `string` | `String` | `String` | `String` |
| Path direction | CSS logical inline | resolved layout direction | resolved writing direction | resolved reading direction |
| Core build target | `wasm32-unknown-unknown` | Android Rust target | Apple Rust target | Windows Rust target |

Each adapter is responsible for converting native scale-independent coordinates
to raw layout units before calling the core and for mapping returned logical
inline positions to its renderer.

The repository includes a Web TypeScript/WASM wrapper with Canvas integration and
an Apple Swift Package with C, UIKit and SwiftUI integration. Wiring those SDKs
into the LastDraft host applications remains a separate application change.

## 9. Conformance requirements

Given identical raw layout units, shaped clusters and document input, conforming
adapters must produce:

- identical object modes and frames;
- identical paragraph and line counts;
- identical fragment rectangles;
- identical UTF-16 ranges per fragment;
- identical content height.

Golden inputs must be executed against native and WASM builds in CI. Platform
rendering may differ at the pixel level, but it may not alter flow geometry.
