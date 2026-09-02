import {
  FLOW_ABI_VERSION,
  type FlowEnvelope,
  type FlowLayout,
  type FlowLayoutRequestV1,
  type CanonicalFontDescriptor,
  type CanonicalTextPosition,
  type CaretMovementDirection,
  type EditorCaretGeometry,
  type EditorCaretMovement,
  type EditorGeometrySnapshotData,
  type EditorSnapshotRequestV1,
  type EditorSnapshotRequestV2,
  type NormalizedPlacement,
  type Q26_6,
  type RichParagraph,
  type ShapedParagraph,
  type WritingMode,
} from "./types.ts";

interface FlowWasmExports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  ld_flow_abi_version(): number;
  ld_flow_bytes_alloc(length: number): number;
  ld_flow_bytes_free(pointer: number, length: number): void;
  ld_flow_layout_json(inputPointer: number, inputLength: number, outputPointer: number): number;
  ld_flow_shaper_create(): number;
  ld_flow_shaper_destroy(shaperPointer: number): void;
  ld_flow_shaper_register_font(
    shaperPointer: number,
    idPointer: number,
    idLength: number,
    fontPointer: number,
    fontLength: number,
    faceIndex: number,
    outputPointer: number,
  ): number;
  ld_flow_shaper_shape_json(
    shaperPointer: number,
    inputPointer: number,
    inputLength: number,
    outputPointer: number,
  ): number;
  ld_flow_editor_snapshot_create_json(
    shaperPointer: number,
    inputPointer: number,
    inputLength: number,
    snapshotOutputPointer: number,
    outputPointer: number,
  ): number;
  ld_flow_editor_snapshot_destroy(snapshotPointer: number): void;
  ld_flow_editor_query_json(
    snapshotPointer: number,
    inputPointer: number,
    inputLength: number,
    outputPointer: number,
  ): number;
  ld_flow_normalized_placement_for_drag(
    contentX: Q26_6,
    contentWidth: Q26_6,
    objectWidth: Q26_6,
    anchorReferenceTop: Q26_6,
    lineHeight: Q26_6,
    proposedX: Q26_6,
    proposedY: Q26_6,
    inlineOutputPointer: number,
    blockOutputPointer: number,
  ): number;
  ld_flow_normalized_placement_for_drag_v4(
    contentX: Q26_6,
    contentY: Q26_6,
    contentWidth: Q26_6,
    contentHeight: Q26_6,
    objectWidth: Q26_6,
    objectHeight: Q26_6,
    anchorReferenceBlockStart: Q26_6,
    lineHeight: Q26_6,
    proposedX: Q26_6,
    proposedY: Q26_6,
    writingMode: number,
    inlineOutputPointer: number,
    blockOutputPointer: number,
  ): number;
}

export class LastDraftFlowError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "LastDraftFlowError";
    this.code = code;
  }
}

export class LastDraftFlowEngine {
  readonly #wasm: FlowWasmExports;
  readonly #encoder = new TextEncoder();
  readonly #decoder = new TextDecoder();
  #shaperPointer: number;

  private constructor(wasm: FlowWasmExports) {
    this.#wasm = wasm;
    const actualVersion = wasm.ld_flow_abi_version();
    if (actualVersion !== FLOW_ABI_VERSION) {
      throw new LastDraftFlowError(
        "abi_version_mismatch",
        `Expected LastDraft Flow ABI ${FLOW_ABI_VERSION}, received ${actualVersion}`,
      );
    }
    this.#shaperPointer = wasm.ld_flow_shaper_create();
    if (this.#shaperPointer === 0) {
      throw new LastDraftFlowError("allocation_failed", "Unable to allocate canonical shaper");
    }
  }

  static async instantiate(
    module: BufferSource | WebAssembly.Module,
    imports: WebAssembly.Imports = {},
  ): Promise<LastDraftFlowEngine> {
    let instance: WebAssembly.Instance;
    if (module instanceof WebAssembly.Module) {
      instance = await WebAssembly.instantiate(module, imports);
    } else {
      instance = (await WebAssembly.instantiate(module, imports)).instance;
    }
    return new LastDraftFlowEngine(instance.exports as FlowWasmExports);
  }

  static async load(url: string | URL, imports: WebAssembly.Imports = {}): Promise<LastDraftFlowEngine> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new LastDraftFlowError("wasm_fetch_failed", `Unable to load ${url}: ${response.status}`);
    }
    return LastDraftFlowEngine.instantiate(await response.arrayBuffer(), imports);
  }

  layout(request: FlowLayoutRequestV1): FlowLayout {
    const input = this.#encoder.encode(JSON.stringify(request));
    const inputPointer = this.#allocate(input.byteLength);
    const outputStructPointer = this.#allocate(8);
    let outputPointer = 0;
    let outputLength = 0;

    try {
      new Uint8Array(this.#wasm.memory.buffer, inputPointer, input.byteLength).set(input);
      const status = this.#wasm.ld_flow_layout_json(
        inputPointer,
        input.byteLength,
        outputStructPointer,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", `Layout ABI returned status ${status}`);
      }

      // The layout call may grow WASM memory, so create the view afterwards.
      const outputView = new DataView(this.#wasm.memory.buffer, outputStructPointer, 8);
      outputPointer = outputView.getUint32(0, true);
      outputLength = outputView.getUint32(4, true);
      if (outputPointer === 0 || outputLength === 0) {
        throw new LastDraftFlowError("empty_layout_response", "Layout ABI returned no response");
      }

      const bytes = new Uint8Array(this.#wasm.memory.buffer, outputPointer, outputLength).slice();
      const envelope = JSON.parse(this.#decoder.decode(bytes)) as FlowEnvelope;
      if (envelope.error) {
        throw new LastDraftFlowError(envelope.error.code, envelope.error.message);
      }
      return envelope.layout;
    } finally {
      if (outputPointer !== 0 && outputLength !== 0) {
        this.#wasm.ld_flow_bytes_free(outputPointer, outputLength);
      }
      this.#wasm.ld_flow_bytes_free(outputStructPointer, 8);
      this.#wasm.ld_flow_bytes_free(inputPointer, input.byteLength);
    }
  }

  registerFont(
    id: string,
    fontData: Uint8Array | ArrayBuffer,
    faceIndex = 0,
  ): CanonicalFontDescriptor {
    this.#requireShaper();
    const idBytes = this.#encoder.encode(id);
    const fontBytes = fontData instanceof Uint8Array ? fontData : new Uint8Array(fontData);
    const idPointer = this.#allocate(idBytes.byteLength);
    const fontPointer = this.#allocate(fontBytes.byteLength);
    const outputStructPointer = this.#allocate(8);
    try {
      new Uint8Array(this.#wasm.memory.buffer, idPointer, idBytes.byteLength).set(idBytes);
      new Uint8Array(this.#wasm.memory.buffer, fontPointer, fontBytes.byteLength).set(fontBytes);
      const status = this.#wasm.ld_flow_shaper_register_font(
        this.#shaperPointer,
        idPointer,
        idBytes.byteLength,
        fontPointer,
        fontBytes.byteLength,
        faceIndex,
        outputStructPointer,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", "Font registration ABI failed");
      }
      const envelope = this.#readJsonOutput<{
        version: 2;
        font?: CanonicalFontDescriptor;
        error?: { code: string; message: string };
      }>(outputStructPointer);
      if (envelope.error) {
        throw new LastDraftFlowError(envelope.error.code, envelope.error.message);
      }
      if (!envelope.font) {
        throw new LastDraftFlowError("empty_font_response", "Font registration returned no font");
      }
      return envelope.font;
    } finally {
      this.#wasm.ld_flow_bytes_free(outputStructPointer, 8);
      this.#wasm.ld_flow_bytes_free(fontPointer, fontBytes.byteLength);
      this.#wasm.ld_flow_bytes_free(idPointer, idBytes.byteLength);
    }
  }

  shapeParagraph(paragraph: RichParagraph): ShapedParagraph {
    this.#requireShaper();
    const input = this.#encoder.encode(JSON.stringify(paragraph));
    const inputPointer = this.#allocate(input.byteLength);
    const outputStructPointer = this.#allocate(8);
    try {
      new Uint8Array(this.#wasm.memory.buffer, inputPointer, input.byteLength).set(input);
      const status = this.#wasm.ld_flow_shaper_shape_json(
        this.#shaperPointer,
        inputPointer,
        input.byteLength,
        outputStructPointer,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", "Paragraph shaping ABI failed");
      }
      const envelope = this.#readJsonOutput<{
        version: 2;
        paragraph?: ShapedParagraph;
        error?: { code: string; message: string };
      }>(outputStructPointer);
      if (envelope.error) {
        throw new LastDraftFlowError(envelope.error.code, envelope.error.message);
      }
      if (!envelope.paragraph) {
        throw new LastDraftFlowError("empty_shaping_response", "Shaping returned no paragraph");
      }
      return envelope.paragraph;
    } finally {
      this.#wasm.ld_flow_bytes_free(outputStructPointer, 8);
      this.#wasm.ld_flow_bytes_free(inputPointer, input.byteLength);
    }
  }

  createEditorSnapshot(
    request: EditorSnapshotRequestV1 | EditorSnapshotRequestV2,
  ): LastDraftEditorSnapshot {
    this.#requireShaper();
    const input = this.#encoder.encode(JSON.stringify(request));
    const inputPointer = this.#allocate(input.byteLength);
    const snapshotOutputPointer = this.#allocate(4);
    const outputStructPointer = this.#allocate(8);
    let snapshotPointer = 0;
    try {
      new Uint8Array(this.#wasm.memory.buffer, inputPointer, input.byteLength).set(input);
      const status = this.#wasm.ld_flow_editor_snapshot_create_json(
        this.#shaperPointer,
        inputPointer,
        input.byteLength,
        snapshotOutputPointer,
        outputStructPointer,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", `Editor snapshot ABI returned status ${status}`);
      }
      snapshotPointer = new DataView(this.#wasm.memory.buffer, snapshotOutputPointer, 4)
        .getUint32(0, true);
      const envelope = this.#readJsonOutput<{
        version: 2;
        snapshot?: EditorGeometrySnapshotData;
        error?: { code: string; message: string };
      }>(outputStructPointer);
      if (envelope.error) {
        throw new LastDraftFlowError(envelope.error.code, envelope.error.message);
      }
      if (snapshotPointer === 0 || !envelope.snapshot) {
        throw new LastDraftFlowError("empty_editor_snapshot", "Editor snapshot creation returned no snapshot");
      }
      const handle = snapshotPointer;
      snapshotPointer = 0;
      return new LastDraftEditorSnapshot(
        envelope.snapshot,
        (query) => this.#editorQuery(handle, query),
        () => this.#wasm.ld_flow_editor_snapshot_destroy(handle),
      );
    } finally {
      if (snapshotPointer !== 0) {
        this.#wasm.ld_flow_editor_snapshot_destroy(snapshotPointer);
      }
      this.#wasm.ld_flow_bytes_free(outputStructPointer, 8);
      this.#wasm.ld_flow_bytes_free(snapshotOutputPointer, 4);
      this.#wasm.ld_flow_bytes_free(inputPointer, input.byteLength);
    }
  }

  dispose(): void {
    if (this.#shaperPointer !== 0) {
      this.#wasm.ld_flow_shaper_destroy(this.#shaperPointer);
      this.#shaperPointer = 0;
    }
  }

  normalizedPlacementForDrag(input: {
    contentXQ26_6: Q26_6;
    contentWidthQ26_6: Q26_6;
    objectWidthQ26_6: Q26_6;
    anchorReferenceTopQ26_6: Q26_6;
    lineHeightQ26_6: Q26_6;
    proposedXQ26_6: Q26_6;
    proposedYQ26_6: Q26_6;
  }): NormalizedPlacement {
    const outputPointer = this.#allocate(8);
    try {
      const status = this.#wasm.ld_flow_normalized_placement_for_drag(
        input.contentXQ26_6,
        input.contentWidthQ26_6,
        input.objectWidthQ26_6,
        input.anchorReferenceTopQ26_6,
        input.lineHeightQ26_6,
        input.proposedXQ26_6,
        input.proposedYQ26_6,
        outputPointer,
        outputPointer + 4,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", `Drag ABI returned status ${status}`);
      }
      const view = new DataView(this.#wasm.memory.buffer, outputPointer, 8);
      return {
        inlineU16: view.getUint16(0, true),
        blockOffset1024: view.getInt32(4, true),
      };
    } finally {
      this.#wasm.ld_flow_bytes_free(outputPointer, 8);
    }
  }

  normalizedPlacementForDragV4(input: {
    contentXQ26_6: Q26_6;
    contentYQ26_6: Q26_6;
    contentWidthQ26_6: Q26_6;
    contentHeightQ26_6: Q26_6;
    objectWidthQ26_6: Q26_6;
    objectHeightQ26_6: Q26_6;
    anchorReferenceBlockStartQ26_6: Q26_6;
    lineHeightQ26_6: Q26_6;
    proposedXQ26_6: Q26_6;
    proposedYQ26_6: Q26_6;
    writingMode: WritingMode;
  }): NormalizedPlacement {
    const outputPointer = this.#allocate(8);
    const writingMode = {
      horizontalTb: 0,
      verticalRl: 1,
      verticalLr: 2,
    }[input.writingMode];
    try {
      const status = this.#wasm.ld_flow_normalized_placement_for_drag_v4(
        input.contentXQ26_6,
        input.contentYQ26_6,
        input.contentWidthQ26_6,
        input.contentHeightQ26_6,
        input.objectWidthQ26_6,
        input.objectHeightQ26_6,
        input.anchorReferenceBlockStartQ26_6,
        input.lineHeightQ26_6,
        input.proposedXQ26_6,
        input.proposedYQ26_6,
        writingMode,
        outputPointer,
        outputPointer + 4,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", `Drag ABI v4 returned status ${status}`);
      }
      const view = new DataView(this.#wasm.memory.buffer, outputPointer, 8);
      return {
        inlineU16: view.getUint16(0, true),
        blockOffset1024: view.getInt32(4, true),
      };
    } finally {
      this.#wasm.ld_flow_bytes_free(outputPointer, 8);
    }
  }

  #allocate(length: number): number {
    const pointer = this.#wasm.ld_flow_bytes_alloc(length);
    if (pointer === 0) {
      throw new LastDraftFlowError("allocation_failed", `Unable to allocate ${length} WASM bytes`);
    }
    return pointer;
  }

  #requireShaper(): void {
    if (this.#shaperPointer === 0) {
      throw new LastDraftFlowError("engine_disposed", "LastDraft Flow engine is disposed");
    }
  }

  #readJsonOutput<T>(outputStructPointer: number): T {
    const view = new DataView(this.#wasm.memory.buffer, outputStructPointer, 8);
    const outputPointer = view.getUint32(0, true);
    const outputLength = view.getUint32(4, true);
    if (outputPointer === 0 || outputLength === 0) {
      throw new LastDraftFlowError("empty_response", "ABI returned no JSON response");
    }
    try {
      const bytes = new Uint8Array(
        this.#wasm.memory.buffer,
        outputPointer,
        outputLength,
      ).slice();
      return JSON.parse(this.#decoder.decode(bytes)) as T;
    } finally {
      this.#wasm.ld_flow_bytes_free(outputPointer, outputLength);
    }
  }

  #editorQuery(snapshotPointer: number, query: object): EditorQueryResult {
    const input = this.#encoder.encode(JSON.stringify(query));
    const inputPointer = this.#allocate(input.byteLength);
    const outputStructPointer = this.#allocate(8);
    try {
      new Uint8Array(this.#wasm.memory.buffer, inputPointer, input.byteLength).set(input);
      const status = this.#wasm.ld_flow_editor_query_json(
        snapshotPointer,
        inputPointer,
        input.byteLength,
        outputStructPointer,
      );
      if (status !== 0) {
        throw new LastDraftFlowError("abi_call_failed", `Editor query ABI returned status ${status}`);
      }
      const envelope = this.#readJsonOutput<{
        version: 2;
        result?: EditorQueryResult;
        error?: { code: string; message: string };
      }>(outputStructPointer);
      if (envelope.error) {
        throw new LastDraftFlowError(envelope.error.code, envelope.error.message);
      }
      if (!envelope.result) {
        throw new LastDraftFlowError("empty_editor_query", "Editor query returned no result");
      }
      return envelope.result;
    } finally {
      this.#wasm.ld_flow_bytes_free(outputStructPointer, 8);
      this.#wasm.ld_flow_bytes_free(inputPointer, input.byteLength);
    }
  }
}

type EditorQueryResult =
  | { kind: "caret"; caret: EditorCaretGeometry }
  | { kind: "hitTest"; position?: CanonicalTextPosition }
  | { kind: "selection"; selection: { paragraphId: string; utf16Start: number; utf16End: number; rects: import("./types.ts").FlowRect[] } }
  | { kind: "move"; movement: EditorCaretMovement };

export class LastDraftEditorSnapshot {
  readonly geometry: EditorGeometrySnapshotData;
  readonly #query: (query: object) => EditorQueryResult;
  readonly #destroy: () => void;
  #disposed = false;

  constructor(
    geometry: EditorGeometrySnapshotData,
    query: (query: object) => EditorQueryResult,
    destroy: () => void,
  ) {
    this.geometry = geometry;
    this.#query = query;
    this.#destroy = destroy;
  }

  caret(position: CanonicalTextPosition): EditorCaretGeometry {
    const result = this.#run({ version: 2, kind: "caret", position });
    if (result.kind !== "caret") throw new LastDraftFlowError("query_kind_mismatch", "Expected caret geometry");
    return result.caret;
  }

  hitTest(xQ26_6: Q26_6, yQ26_6: Q26_6): CanonicalTextPosition | undefined {
    const result = this.#run({ version: 2, kind: "hitTest", xQ26_6, yQ26_6 });
    if (result.kind !== "hitTest") throw new LastDraftFlowError("query_kind_mismatch", "Expected hit-test geometry");
    return result.position;
  }

  selection(paragraphId: string, utf16Start: number, utf16End: number): import("./types.ts").SelectionGeometry {
    const result = this.#run({ version: 2, kind: "selection", paragraphId, utf16Start, utf16End });
    if (result.kind !== "selection") throw new LastDraftFlowError("query_kind_mismatch", "Expected selection geometry");
    return result.selection;
  }

  moveCaret(
    position: CanonicalTextPosition,
    direction: CaretMovementDirection,
    preferredInlineQ26_6?: Q26_6,
  ): EditorCaretMovement {
    const result = this.#run({
      version: 2,
      kind: "move",
      position,
      direction,
      preferredInlineQ26_6,
    });
    if (result.kind !== "move") throw new LastDraftFlowError("query_kind_mismatch", "Expected caret movement geometry");
    return result.movement;
  }

  dispose(): void {
    if (!this.#disposed) {
      this.#destroy();
      this.#disposed = true;
    }
  }

  #run(query: object): EditorQueryResult {
    if (this.#disposed) throw new LastDraftFlowError("snapshot_disposed", "Editor geometry snapshot is disposed");
    return this.#query(query);
  }
}
