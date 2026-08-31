# Web and iOS integration

The integrations in this repository are standalone. `/Users/ayanmukherjee/dev/lastdraft`
and `/Users/ayanmukherjee/dev/lastdraft-ios` were inspected read-only and were not
modified.

## LastDraft model mapping

LastDraft already gives paragraph and heading nodes durable string `blockId`
attributes. Map them directly:

```text
ProseMirror node.attrs.blockId → FlowParagraph.id
anchored image anchorId        → FlowObject.anchor.paragraphId
image attachment/float ID      → FlowObject.id
```

The existing LastDraft free-form `floats` contract is canvas-relative and is
explicitly overlay-only: it does not wrap text. The new flow object is therefore
a new semantic representation, not a reinterpretation of old `x/y/w/h` values.
A host migration must create an anchor offset and normalized local placement;
silently treating old canvas coordinates as the new contract would move content.

The engine indexes text in UTF-16, matching browser strings, `NSRange`, TextKit,
Kotlin/JVM and .NET. Host shaping must emit grapheme clusters in logical order
with raw 26.6 advances and legal break opportunities. Rendering must use the
returned fragment ranges and must not ask Canvas or TextKit to independently
rewrap those ranges.

## Web

Build and verify:

```sh
cd web
npm install
npm run typecheck
npm test
```

`npm run build:wasm` writes `web/dist/lastdraft_flow.wasm`. Load it once:

```ts
import {
  LastDraftFlowEngine,
  LastDraftCanvasFlowController,
} from "@lastdraft/flow-web";

const engine = await LastDraftFlowEngine.load(
  new URL("./lastdraft_flow.wasm", import.meta.url),
);

const controller = new LastDraftCanvasFlowController({
  canvas,
  engine,
  painter: {
    drawTextFragment({ context, fragment, x, baseline }) {
      // Draw exactly fragment.utf16Start..<fragment.utf16End using the
      // corresponding shaped paragraph run at x/baseline.
    },
    drawObject({ context, object, x, y, width, height }) {
      context.drawImage(imagesByFloatID.get(object.id), x, y, width, height);
    },
  },
  onPlacementChange(objectID, placement) {
    updatePersistedFlowObject(objectID, placement);
    controller.setRequest(buildRequestFromCurrentDocument());
  },
});

controller.setRequest(request);
```

The controller performs hit testing and pointer capture. Every move calls the
Rust drag-normalization function, persists only normalized local placement, and
lets the host rerun layout for immediate exclusion reflow.

## iOS

`apple/Artifacts/CLastDraftFlow.xcframework` contains:

- macOS arm64, for Swift package tests;
- iOS arm64, for devices;
- iOS Simulator arm64 and x86_64.

Rebuild it from source with:

```sh
bash apple/build-xcframework.sh
```

Add the local `apple` package to an Xcode project and import
`LastDraftFlowApple`. The Swift bridge owns request encoding, ABI validation,
Rust calls, response decoding and engine errors.

UIKit uses `LastDraftFlowView`. Its delegate draws exact text fragments, resolves
images from LastDraft's attachment map, and receives normalized placement after
pan gestures:

```swift
let engine = try LastDraftFlowEngine()
let flowView = LastDraftFlowView(engine: engine)
flowView.delegate = renderer
flowView.contentRect = request.content
flowView.paragraphLineHeightsQ26_6 = Dictionary(
    uniqueKeysWithValues: request.paragraphs.map { ($0.id, $0.style.lineHeightQ26_6) }
)
flowView.layout = try engine.layout(request)
```

SwiftUI uses `LastDraftFlowCanvas`, backed by the same UIKit view. Construct a
`LastDraftFlowRenderer` with text, image and placement callbacks, then pass the
current `FlowLayout`. When placement changes, update the model, call the engine
again and provide the new layout.

The renderer intentionally obtains images by object ID rather than URL, matching
LastDraft's rule that document objects store attachment IDs and signed URLs are
resolved only at render time.
