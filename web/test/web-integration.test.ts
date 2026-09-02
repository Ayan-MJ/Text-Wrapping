import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  LastDraftCanvasRenderer,
  LastDraftFlowEngine,
  type CanvasFlowPainter,
  type EditorSnapshotRequestV1,
  type EditorSnapshotRequestV2,
  type FlowLayoutRequestV1,
} from "../src/index.ts";

const wasmPath = new URL("../dist/lastdraft_flow.wasm", import.meta.url);

function geometryFingerprint(
  geometry: import("../src/index.ts").EditorGeometrySnapshotData,
): string {
  const tokens: Array<string | number> = [
    "lastdraft-geometry-v2",
    geometry.layout.contentHeightQ26_6,
    geometry.layout.contentWidthQ26_6 ?? 0,
  ];
  for (const paragraph of geometry.layout.paragraphs) {
    tokens.push(
      "p", paragraph.paragraphId, paragraph.baseDirection ?? "",
      paragraph.writingMode ?? "", paragraph.topQ26_6, paragraph.bottomQ26_6,
    );
    for (const line of paragraph.lines) {
      tokens.push(
        "l", line.writingMode ?? "", line.topQ26_6, line.baselineQ26_6,
        line.blockStartQ26_6 ?? 0, line.inlineStartQ26_6 ?? 0,
      );
      for (const fragment of line.fragments) {
        tokens.push(
          "f", fragment.utf16Start, fragment.utf16End,
          fragment.rect.xQ26_6, fragment.rect.yQ26_6,
          fragment.rect.widthQ26_6, fragment.rect.heightQ26_6,
        );
      }
    }
  }
  for (const object of geometry.layout.objects) {
    tokens.push(
      "o", object.id, object.anchor.paragraphId, object.anchor.utf16Offset,
      object.anchor.affinity, object.anchorReferenceBlockStartQ26_6 ?? 0,
      object.frame.xQ26_6, object.frame.yQ26_6,
      object.frame.widthQ26_6, object.frame.heightQ26_6,
      object.exclusionFrame.xQ26_6, object.exclusionFrame.yQ26_6,
      object.exclusionFrame.widthQ26_6, object.exclusionFrame.heightQ26_6,
      object.mode,
    );
  }
  for (const cluster of geometry.clusters) {
    tokens.push(
      "c", cluster.paragraphId, cluster.utf16Start, cluster.utf16End,
      cluster.bidiLevel, cluster.direction, cluster.orientation, cluster.writingMode,
      cluster.rect.xQ26_6, cluster.rect.yQ26_6,
      cluster.rect.widthQ26_6, cluster.rect.heightQ26_6,
    );
  }
  for (const glyph of geometry.glyphs) {
    tokens.push(
      "g", glyph.paragraphId, glyph.runIndex, glyph.glyphIndex, glyph.glyphId,
      glyph.clusterUtf16, glyph.pageXQ26_6, glyph.pageYQ26_6,
      glyph.xAdvanceQ26_6, glyph.yAdvanceQ26_6,
      glyph.xOffsetQ26_6, glyph.yOffsetQ26_6,
      glyph.orientation, glyph.writingMode, glyph.lineIndex, glyph.fragmentIndex,
    );
  }
  for (const stop of geometry.caretStops) {
    tokens.push(
      "s", stop.position.paragraphId, stop.position.utf16Offset,
      stop.position.affinity, stop.writingMode,
      stop.rect.xQ26_6, stop.rect.yQ26_6,
      stop.rect.widthQ26_6, stop.rect.heightQ26_6,
      stop.lineIndex, stop.documentLineIndex, stop.fragmentIndex, stop.visualIndex,
    );
  }
  let hash = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(tokens.join("|"))) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return hash.toString(16).padStart(16, "0");
}

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

test("WASM registers exact font bytes and shapes a rich-text run", async (context) => {
  let font: Uint8Array;
  try {
    font = await readFile("/System/Library/Fonts/Supplemental/Arial.ttf");
  } catch {
    context.skip("portable CI font fixture is not installed");
    return;
  }
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const descriptor = engine.registerFont("test-arial", font);
  assert.equal(descriptor.sha256.length, 64);

  const paragraph = engine.shapeParagraph({
    id: "rich-block",
    text: "office",
    baseDirection: "auto",
    writingMode: "horizontalTb",
    textOrientation: "mixed",
    runs: [{
      utf16Start: 0,
      utf16End: 6,
      style: {
        fontId: "test-arial",
        fontSizeQ26_6: 16 * 64,
        direction: "leftToRight",
        language: "en",
        features: [{ tag: "liga", value: 1 }],
        variations: [],
        fillRGBA: 0x000000ff,
        underline: false,
        strikethrough: false,
      },
    }],
  });
  assert.equal(paragraph.id, "rich-block");
  assert.ok(paragraph.runs[0].glyphs.length > 0);
  assert.equal(paragraph.clusters.at(-1)?.utf16End, 6);
  engine.dispose();
});

test("ABI v4 resolves mixed bidi, right-aligns RTL fragments, and ignores run direction hints", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const fixtureRoot = new URL("../../fixtures/", import.meta.url);
  await Promise.all([
    ["fixture-noto-sans", "fonts/NotoSans-Variable.ttf"],
    ["fixture-noto-arabic", "fonts/NotoSansArabic-Variable.ttf"],
    ["fixture-noto-hebrew", "fonts/NotoSansHebrew-Variable.ttf"],
  ].map(async ([id, path]) => engine.registerFont(id, await readFile(new URL(path, fixtureRoot)))));
  const request = JSON.parse(
    await readFile(new URL("editor/bidi-horizontal-v2.json", fixtureRoot), "utf8"),
  ) as EditorSnapshotRequestV2;
  const snapshot = engine.createEditorSnapshot(request);
  const paragraph = snapshot.geometry.layout.paragraphs[0];
  assert.equal(paragraph.baseDirection, "rightToLeft");
  assert.equal(paragraph.writingMode, "horizontalTb");
  const split = paragraph.lines.find((line) => line.fragments.length === 2);
  assert.ok(split);
  assert.ok(split.fragments[0].rect.xQ26_6 > split.fragments[1].rect.xQ26_6);
  assert.ok(snapshot.geometry.clusters.some((cluster) => cluster.bidiLevel % 2 === 0));
  assert.ok(snapshot.geometry.clusters.some((cluster) => cluster.bidiLevel % 2 === 1));
  const englishStart = "مرحبا، שלום ".length;
  const english = snapshot.geometry.clusters.filter((cluster) => (
    cluster.utf16Start >= englishStart && cluster.utf16End < 24
  ));
  assert.ok(english.some((cluster) => cluster.direction === "leftToRight"));
  assert.equal(geometryFingerprint(snapshot.geometry), "afaf495e47e79478");
  snapshot.dispose();
  engine.dispose();
});

test("ABI v4 exposes native vertical glyphs, exclusion flow, editor geometry, and drag axes", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const fixtureRoot = new URL("../../fixtures/", import.meta.url);
  engine.registerFont(
    "fixture-noto-jp",
    await readFile(new URL("fonts/NotoSansJP-Variable.ttf", fixtureRoot)),
  );
  const request = JSON.parse(
    await readFile(new URL("editor/vertical-v2.json", fixtureRoot), "utf8"),
  ) as EditorSnapshotRequestV2;
  const first = engine.createEditorSnapshot(request);
  const second = engine.createEditorSnapshot(request);
  assert.deepEqual(first.geometry, second.geometry);
  assert.equal(geometryFingerprint(first.geometry), "4a6e9d22d58d71c6");
  assert.deepEqual(
    first.geometry.layout.paragraphs.map((paragraph) => paragraph.writingMode),
    ["verticalRl", "verticalLr"],
  );
  assert.ok(first.geometry.layout.paragraphs.every((paragraph) => (
    paragraph.lines.some((line) => line.fragments.length === 2)
  )));
  assert.ok(first.geometry.glyphs.some((glyph) => (
    glyph.orientation === "upright" && glyph.yAdvanceQ26_6 !== 0
  )));
  assert.ok(first.geometry.glyphs.some((glyph) => glyph.orientation === "sideways"));
  assert.ok(first.geometry.caretStops.every((stop) => stop.rect.heightQ26_6 === 64));
  const stop = first.geometry.caretStops[0];
  assert.equal(
    first.hitTest(stop.rect.xQ26_6, stop.rect.yQ26_6)?.utf16Offset,
    stop.position.utf16Offset,
  );
  assert.ok(first.selection("vertical-rl", 0, 6).rects.length > 0);
  assert.ok(first.moveCaret(stop.position, "down").rect.yQ26_6 >= stop.rect.yQ26_6);

  assert.deepEqual(engine.normalizedPlacementForDragV4({
    contentXQ26_6: 0,
    contentYQ26_6: 0,
    contentWidthQ26_6: 100,
    contentHeightQ26_6: 200,
    objectWidthQ26_6: 20,
    objectHeightQ26_6: 40,
    anchorReferenceBlockStartQ26_6: 100,
    lineHeightQ26_6: 10,
    proposedXQ26_6: 60,
    proposedYQ26_6: 80,
    writingMode: "verticalRl",
  }), { inlineU16: 32_768, blockOffset1024: 2_048 });
  first.dispose();
  second.dispose();
  engine.dispose();
});

test("WASM exports deterministic Unicode editor geometry across an image exclusion", async () => {
  const engine = await LastDraftFlowEngine.instantiate(await readFile(wasmPath));
  const fixtureRoot = new URL("../../fixtures/", import.meta.url);
  await Promise.all([
    ["fixture-noto-sans", "fonts/NotoSans-Variable.ttf"],
    ["fixture-noto-sans-italic", "fonts/NotoSans-Italic-Variable.ttf"],
    ["fixture-noto-devanagari", "fonts/NotoSansDevanagari-Variable.ttf"],
    ["fixture-noto-arabic", "fonts/NotoSansArabic-Variable.ttf"],
  ].map(async ([id, path]) => engine.registerFont(id, await readFile(new URL(path, fixtureRoot)))));

  const request = JSON.parse(
    await readFile(new URL("editor/unicode-exclusion-v1.json", fixtureRoot), "utf8"),
  ) as EditorSnapshotRequestV1;
  const snapshot = engine.createEditorSnapshot(request);
  assert.equal(snapshot.geometry.layout.paragraphs[0].lines[0].fragments.length, 2);
  assert.ok(snapshot.geometry.glyphs.length > 0);
  assert.ok(snapshot.geometry.glyphs.every((glyph) => Number.isInteger(glyph.pageXQ26_6)));
  assert.ok(snapshot.geometry.clusters.some((cluster) => cluster.direction === "rightToLeft"));
  assert.ok(!snapshot.geometry.caretStops.some((stop) => stop.position.utf16Offset === 26));

  const fragments = snapshot.geometry.layout.paragraphs[0].lines[0].fragments;
  const boundary = fragments[0].utf16End;
  assert.equal(fragments[1].utf16Start, boundary);
  const upstream = snapshot.caret({
    paragraphId: "unicode-exclusion", utf16Offset: boundary, affinity: "upstream",
  });
  const downstream = snapshot.caret({
    paragraphId: "unicode-exclusion", utf16Offset: boundary, affinity: "downstream",
  });
  assert.ok(upstream.rect.xQ26_6 < downstream.rect.xQ26_6);
  assert.equal(snapshot.hitTest(upstream.rect.xQ26_6 + 1, upstream.rect.yQ26_6)?.affinity, "upstream");
  assert.equal(snapshot.hitTest(downstream.rect.xQ26_6 - 1, downstream.rect.yQ26_6)?.affinity, "downstream");
  assert.ok(snapshot.selection("unicode-exclusion", 1, 90).rects.length >= 3);
  assert.deepEqual(
    snapshot.moveCaret(upstream.position, "right").position,
    downstream.position,
  );
  snapshot.dispose();
  engine.dispose();
});
