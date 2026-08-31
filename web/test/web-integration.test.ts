import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  LastDraftCanvasRenderer,
  LastDraftFlowEngine,
  type CanvasFlowPainter,
  type FlowLayoutRequestV1,
} from "../src/index.ts";

const wasmPath = new URL("../dist/lastdraft_flow.wasm", import.meta.url);

function request(): FlowLayoutRequestV1 {
  return {
    version: 1,
    content: { xQ26_6: 0, yQ26_6: 0, widthQ26_6: 100, heightQ26_6: 10_000 },
    paragraphs: [{
      id: "prosemirror-block-uuid",
      utf16Length: 4,
      style: { lineHeightQ26_6: 10, ascentQ26_6: 8, spaceAfterQ26_6: 0 },
      clusters: Array.from({ length: 4 }, (_, index) => ({
        utf16Start: index,
        utf16End: index + 1,
        advanceQ26_6: 10,
        canBreakAfter: true,
      })),
    }],
    objects: [{
      id: "fl_uuid",
      anchor: { paragraphId: "prosemirror-block-uuid", utf16Offset: 0, affinity: "downstream" },
      placement: { inlineU16: 32_768, blockOffset1024: 0 },
      size: { idealWidthQ26_6: 40, idealHeightQ26_6: 10, maxInlineU16: 65_535 },
      exclusion: {
        shape: "bounds",
        marginStartQ26_6: 0,
        marginTopQ26_6: 0,
        marginEndQ26_6: 0,
        marginBottomQ26_6: 0,
        minimumFragmentWidthQ26_6: 10,
      },
      responsivePolicy: "scale-then-centered-block-v1",
    }],
  };
}

test("WASM wrapper lays text out on both sides and preserves LastDraft string IDs", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const layout = engine.layout(request());

  assert.equal(layout.paragraphs[0].paragraphId, "prosemirror-block-uuid");
  assert.equal(layout.objects[0].id, "fl_uuid");
  assert.equal(layout.paragraphs[0].lines[0].fragments.length, 2);
});

test("WASM owns drag normalization", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  assert.deepEqual(engine.normalizedPlacementForDrag({
    contentXQ26_6: 0,
    contentWidthQ26_6: 100,
    objectWidthQ26_6: 40,
    anchorReferenceTopQ26_6: 30,
    lineHeightQ26_6: 10,
    proposedXQ26_6: 48,
    proposedYQ26_6: 50,
  }), { inlineU16: 52_428, blockOffset1024: 2_048 });
});

test("Canvas renderer paints fragment geometry before the image", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const layout = engine.layout(request());
  const calls: string[] = [];
  const context = { save() {}, restore() {} } as unknown as CanvasRenderingContext2D;
  const painter: CanvasFlowPainter = {
    drawTextFragment: ({ fragment }) => calls.push(`text:${fragment.utf16Start}-${fragment.utf16End}`),
    drawObject: ({ object }) => calls.push(`object:${object.id}`),
  };

  new LastDraftCanvasRenderer().render(context, layout, painter);
  assert.deepEqual(calls, ["text:0-3", "text:3-4", "object:fl_uuid"]);
});
