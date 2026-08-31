import type {
  FlowFragment,
  FlowLayout,
  FlowLine,
  FlowParagraphLayout,
  ResolvedFlowObject,
} from "./types.ts";
import { layoutUnitToDip } from "./types.ts";

export interface CanvasFlowPainter {
  drawTextFragment(input: {
    context: CanvasRenderingContext2D;
    paragraph: FlowParagraphLayout;
    line: FlowLine;
    fragment: FlowFragment;
    x: number;
    top: number;
    baseline: number;
    width: number;
    height: number;
  }): void;
  drawObject(input: {
    context: CanvasRenderingContext2D;
    object: ResolvedFlowObject;
    x: number;
    y: number;
    width: number;
    height: number;
  }): void;
}

/** Draws core-owned geometry without allowing Canvas text metrics to rewrap it. */
export class LastDraftCanvasRenderer {
  render(
    context: CanvasRenderingContext2D,
    layout: FlowLayout,
    painter: CanvasFlowPainter,
  ): void {
    context.save();
    try {
      for (const paragraph of layout.paragraphs) {
        for (const line of paragraph.lines) {
          for (const fragment of line.fragments) {
            painter.drawTextFragment({
              context,
              paragraph,
              line,
              fragment,
              x: layoutUnitToDip(fragment.rect.xQ26_6),
              top: layoutUnitToDip(fragment.rect.yQ26_6),
              baseline: layoutUnitToDip(line.baselineQ26_6),
              width: layoutUnitToDip(fragment.rect.widthQ26_6),
              height: layoutUnitToDip(fragment.rect.heightQ26_6),
            });
          }
        }
      }

      // Images are composited after prose. Their exclusion has already kept all
      // text out of the occupied geometry.
      for (const object of layout.objects) {
        painter.drawObject({
          context,
          object,
          x: layoutUnitToDip(object.frame.xQ26_6),
          y: layoutUnitToDip(object.frame.yQ26_6),
          width: layoutUnitToDip(object.frame.widthQ26_6),
          height: layoutUnitToDip(object.frame.heightQ26_6),
        });
      }
    } finally {
      context.restore();
    }
  }

  hitTestObject(layout: FlowLayout, x: number, y: number): ResolvedFlowObject | undefined {
    return [...layout.objects].reverse().find((object) => {
      const left = layoutUnitToDip(object.frame.xQ26_6);
      const top = layoutUnitToDip(object.frame.yQ26_6);
      const right = left + layoutUnitToDip(object.frame.widthQ26_6);
      const bottom = top + layoutUnitToDip(object.frame.heightQ26_6);
      return x >= left && x <= right && y >= top && y <= bottom;
    });
  }
}

export function resizeCanvasForDisplay(canvas: HTMLCanvasElement, logicalHeight: number): void {
  const ratio = window.devicePixelRatio || 1;
  const logicalWidth = canvas.clientWidth;
  canvas.style.height = `${logicalHeight}px`;
  const pixelWidth = Math.max(1, Math.round(logicalWidth * ratio));
  const pixelHeight = Math.max(1, Math.round(logicalHeight * ratio));
  if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
    canvas.width = pixelWidth;
    canvas.height = pixelHeight;
  }
  const context = canvas.getContext("2d");
  context?.setTransform(ratio, 0, 0, ratio, 0, 0);
}

