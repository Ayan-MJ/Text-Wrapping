export const LAYOUT_UNITS_PER_DIP = 64;
export const FLOW_ABI_VERSION = 1;

export type Q26_6 = number;

export interface FlowRect {
  xQ26_6: Q26_6;
  yQ26_6: Q26_6;
  widthQ26_6: Q26_6;
  heightQ26_6: Q26_6;
}

export interface FlowCluster {
  utf16Start: number;
  utf16End: number;
  advanceQ26_6: Q26_6;
  canBreakAfter: boolean;
  isWhitespace?: boolean;
}

export interface FlowParagraph {
  /** LastDraft's existing ProseMirror attrs.blockId. */
  id: string;
  utf16Length: number;
  style: {
    lineHeightQ26_6: Q26_6;
    ascentQ26_6: Q26_6;
    spaceAfterQ26_6: Q26_6;
  };
  clusters: FlowCluster[];
}

export interface NormalizedPlacement {
  inlineU16: number;
  blockOffset1024: number;
}

export interface AnchoredFlowObjectV1 {
  id: string;
  anchor: {
    paragraphId: string;
    utf16Offset: number;
    affinity: "upstream" | "downstream";
  };
  placement: NormalizedPlacement;
  size: {
    idealWidthQ26_6: Q26_6;
    idealHeightQ26_6: Q26_6;
    maxInlineU16: number;
  };
  exclusion: {
    shape: "bounds";
    marginStartQ26_6: Q26_6;
    marginTopQ26_6: Q26_6;
    marginEndQ26_6: Q26_6;
    marginBottomQ26_6: Q26_6;
    minimumFragmentWidthQ26_6: Q26_6;
  };
  responsivePolicy: "scale-then-centered-block-v1";
}

export interface FlowLayoutRequestV1 {
  version: 1;
  content: FlowRect;
  paragraphs: FlowParagraph[];
  objects: AnchoredFlowObjectV1[];
}

export interface FlowFragment {
  rect: FlowRect;
  utf16Start: number;
  utf16End: number;
}

export interface FlowLine {
  topQ26_6: Q26_6;
  baselineQ26_6: Q26_6;
  fragments: FlowFragment[];
}

export interface FlowParagraphLayout {
  paragraphId: string;
  topQ26_6: Q26_6;
  bottomQ26_6: Q26_6;
  lines: FlowLine[];
}

export interface ResolvedFlowObject {
  id: string;
  anchor: AnchoredFlowObjectV1["anchor"];
  anchorReferenceTopQ26_6: Q26_6;
  frame: FlowRect;
  exclusionFrame: FlowRect;
  mode: "userPositioned" | "blockFallback";
}

export interface FlowLayout {
  contentHeightQ26_6: Q26_6;
  paragraphs: FlowParagraphLayout[];
  objects: ResolvedFlowObject[];
}

export interface FlowError {
  code: string;
  message: string;
}

export type FlowEnvelope =
  | { version: 1; layout: FlowLayout; error?: never }
  | { version: 1; error: FlowError; layout?: never };

export interface FlowPoint {
  x: number;
  y: number;
}

export function dipToLayoutUnit(value: number): Q26_6 {
  return Math.round(value * LAYOUT_UNITS_PER_DIP);
}

export function layoutUnitToDip(value: Q26_6): number {
  return value / LAYOUT_UNITS_PER_DIP;
}

