import {
  FLOW_ABI_VERSION,
  type FlowEnvelope,
  type FlowLayout,
  type FlowLayoutRequestV1,
  type NormalizedPlacement,
  type Q26_6,
} from "./types.ts";

interface FlowWasmExports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  ld_flow_abi_version(): number;
  ld_flow_bytes_alloc(length: number): number;
  ld_flow_bytes_free(pointer: number, length: number): void;
  ld_flow_layout_json(inputPointer: number, inputLength: number, outputPointer: number): number;
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

  private constructor(wasm: FlowWasmExports) {
    this.#wasm = wasm;
    const actualVersion = wasm.ld_flow_abi_version();
    if (actualVersion !== FLOW_ABI_VERSION) {
      throw new LastDraftFlowError(
        "abi_version_mismatch",
        `Expected LastDraft Flow ABI ${FLOW_ABI_VERSION}, received ${actualVersion}`,
      );
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

  #allocate(length: number): number {
    const pointer = this.#wasm.ld_flow_bytes_alloc(length);
    if (pointer === 0) {
      throw new LastDraftFlowError("allocation_failed", `Unable to allocate ${length} WASM bytes`);
    }
    return pointer;
  }
}
