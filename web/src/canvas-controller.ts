import { LastDraftCanvasRenderer, type CanvasFlowPainter, resizeCanvasForDisplay } from "./canvas-renderer.ts";
import { LastDraftFlowEngine } from "./engine.ts";
import {
  dipToLayoutUnit,
  layoutUnitToDip,
  type FlowLayout,
  type FlowLayoutRequestV1,
  type FlowPoint,
  type NormalizedPlacement,
  type ResolvedFlowObject,
} from "./types.ts";

export interface CanvasFlowControllerOptions {
  canvas: HTMLCanvasElement;
  engine: LastDraftFlowEngine;
  painter: CanvasFlowPainter;
  onPlacementChange(objectId: string, placement: NormalizedPlacement): void;
}

/** Pointer integration for free two-dimensional dragging and live text reflow. */
export class LastDraftCanvasFlowController {
  readonly #canvas: HTMLCanvasElement;
  readonly #engine: LastDraftFlowEngine;
  readonly #painter: CanvasFlowPainter;
  readonly #onPlacementChange: CanvasFlowControllerOptions["onPlacementChange"];
  readonly #renderer = new LastDraftCanvasRenderer();
  #request?: FlowLayoutRequestV1;
  #layout?: FlowLayout;
  #drag?: { object: ResolvedFlowObject; start: FlowPoint; frameStart: FlowPoint; pointerId: number };

  constructor(options: CanvasFlowControllerOptions) {
    this.#canvas = options.canvas;
    this.#engine = options.engine;
    this.#painter = options.painter;
    this.#onPlacementChange = options.onPlacementChange;
    this.#canvas.addEventListener("pointerdown", this.#pointerDown);
    this.#canvas.addEventListener("pointermove", this.#pointerMove);
    this.#canvas.addEventListener("pointerup", this.#pointerUp);
    this.#canvas.addEventListener("pointercancel", this.#pointerUp);
  }

  setRequest(request: FlowLayoutRequestV1): FlowLayout {
    this.#request = request;
    this.#layout = this.#engine.layout(request);
    this.render();
    return this.#layout;
  }

  render(): void {
    if (!this.#layout) return;
    const contentY = this.#request?.content.yQ26_6 ?? 0;
    const logicalHeight = layoutUnitToDip(Math.max(0, contentY + this.#layout.contentHeightQ26_6));
    resizeCanvasForDisplay(this.#canvas, logicalHeight);
    const context = this.#canvas.getContext("2d");
    if (!context) return;
    context.clearRect(0, 0, this.#canvas.clientWidth, logicalHeight);
    this.#renderer.render(context, this.#layout, this.#painter);
  }

  destroy(): void {
    this.#canvas.removeEventListener("pointerdown", this.#pointerDown);
    this.#canvas.removeEventListener("pointermove", this.#pointerMove);
    this.#canvas.removeEventListener("pointerup", this.#pointerUp);
    this.#canvas.removeEventListener("pointercancel", this.#pointerUp);
  }

  #localPoint(event: PointerEvent): FlowPoint {
    const bounds = this.#canvas.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  }

  #pointerDown = (event: PointerEvent): void => {
    if (!this.#layout) return;
    const point = this.#localPoint(event);
    const object = this.#renderer.hitTestObject(this.#layout, point.x, point.y);
    if (!object) return;
    this.#drag = {
      object,
      start: point,
      frameStart: {
        x: layoutUnitToDip(object.frame.xQ26_6),
        y: layoutUnitToDip(object.frame.yQ26_6),
      },
      pointerId: event.pointerId,
    };
    this.#canvas.setPointerCapture(event.pointerId);
    event.preventDefault();
  };

  #pointerMove = (event: PointerEvent): void => {
    const drag = this.#drag;
    const request = this.#request;
    if (!drag || !request || drag.pointerId !== event.pointerId) return;
    const point = this.#localPoint(event);
    const proposedX = drag.frameStart.x + point.x - drag.start.x;
    const proposedY = drag.frameStart.y + point.y - drag.start.y;
    const paragraph = request.paragraphs.find(
      (candidate) => candidate.id === drag.object.anchor.paragraphId,
    );
    if (!paragraph) return;

    const placement = this.#engine.normalizedPlacementForDrag({
      contentXQ26_6: request.content.xQ26_6,
      contentWidthQ26_6: request.content.widthQ26_6,
      objectWidthQ26_6: drag.object.frame.widthQ26_6,
      anchorReferenceTopQ26_6: drag.object.anchorReferenceTopQ26_6,
      lineHeightQ26_6: paragraph.style.lineHeightQ26_6,
      proposedXQ26_6: dipToLayoutUnit(proposedX),
      proposedYQ26_6: dipToLayoutUnit(proposedY),
    });

    this.#onPlacementChange(drag.object.id, placement);
  };

  #pointerUp = (event: PointerEvent): void => {
    if (!this.#drag || this.#drag.pointerId !== event.pointerId) return;
    this.#canvas.releasePointerCapture(event.pointerId);
    this.#drag = undefined;
  };
}
