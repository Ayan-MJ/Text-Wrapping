export const LAYOUT_UNITS_PER_DIP = 64;
export const FLOW_ABI_VERSION = 4;

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
  writingMode?: WritingMode;
  blockStartQ26_6?: Q26_6;
  inlineStartQ26_6?: Q26_6;
  fragments: FlowFragment[];
}

export interface FlowParagraphLayout {
  paragraphId: string;
  topQ26_6: Q26_6;
  bottomQ26_6: Q26_6;
  bounds?: FlowRect;
  baseDirection?: CanonicalTextDirection;
  writingMode?: WritingMode;
  lines: FlowLine[];
}

export interface ResolvedFlowObject {
  id: string;
  anchor: AnchoredFlowObjectV1["anchor"];
  anchorReferenceTopQ26_6: Q26_6;
  anchorReferenceBlockStartQ26_6?: Q26_6;
  frame: FlowRect;
  exclusionFrame: FlowRect;
  mode: "userPositioned" | "blockFallback";
}

export interface FlowLayout {
  contentHeightQ26_6: Q26_6;
  contentWidthQ26_6?: Q26_6;
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

/** Four printable ASCII bytes such as liga, kern, or wght. */
export type OpenTypeTag = string;

export type CanonicalTextDirection = "leftToRight" | "rightToLeft";
export type ParagraphBaseDirection = "auto" | CanonicalTextDirection;
export type WritingMode = "horizontalTb" | "verticalRl" | "verticalLr";
export type TextOrientation = "mixed" | "upright" | "sideways";
export type GlyphOrientation = "upright" | "sideways";

export interface CanonicalFontFeature {
  tag: OpenTypeTag;
  value: number;
}

export interface CanonicalFontVariation {
  tag: OpenTypeTag;
  value16_16: number;
}

export interface RichTextStyle {
  fontId: string;
  fontSizeQ26_6: Q26_6;
  /** @deprecated ABI v4 derives shaping-run direction from paragraph UBA levels. */
  direction?: CanonicalTextDirection;
  language?: string;
  features: CanonicalFontFeature[];
  variations: CanonicalFontVariation[];
  fillRGBA: number;
  underline: boolean;
  strikethrough: boolean;
}

export interface RichTextRun {
  utf16Start: number;
  utf16End: number;
  style: RichTextStyle;
}

export interface RichParagraph {
  id: string;
  text: string;
  baseDirection: ParagraphBaseDirection;
  writingMode: WritingMode;
  textOrientation: TextOrientation;
  runs: RichTextRun[];
}

export interface CanonicalFontDescriptor {
  id: string;
  sha256: string;
  faceIndex: number;
  unitsPerEm: number;
}

export interface ShapedGlyph {
  glyphId: number;
  clusterUtf16: number;
  xAdvanceQ26_6: Q26_6;
  yAdvanceQ26_6: Q26_6;
  xOffsetQ26_6: Q26_6;
  yOffsetQ26_6: Q26_6;
}

export interface ClusterCaretStop {
  utf16Offset: number;
  inlineOffsetQ26_6: Q26_6;
}

export interface ShapedCluster {
  utf16Start: number;
  utf16End: number;
  advanceQ26_6: Q26_6;
  canBreakAfter: boolean;
  isWhitespace: boolean;
  bidiLevel: number;
  direction: CanonicalTextDirection;
  orientation: GlyphOrientation;
  caretStops: ClusterCaretStop[];
}

export interface ShapedRun {
  utf16Start: number;
  utf16End: number;
  font: CanonicalFontDescriptor;
  fontSizeQ26_6: Q26_6;
  bidiLevel: number;
  direction: CanonicalTextDirection;
  orientation: GlyphOrientation;
  glyphs: ShapedGlyph[];
  advanceQ26_6: Q26_6;
}

export interface ShapedParagraph {
  id: string;
  utf16Length: number;
  requestedBaseDirection: ParagraphBaseDirection;
  baseDirection: CanonicalTextDirection;
  writingMode: WritingMode;
  textOrientation: TextOrientation;
  runs: ShapedRun[];
  clusters: ShapedCluster[];
}

export interface CanonicalTextPosition {
  paragraphId: string;
  utf16Offset: number;
  affinity: "upstream" | "downstream";
}

export interface CaretGeometry {
  position: CanonicalTextPosition;
  rect: FlowRect;
}

export interface SelectionGeometry {
  paragraphId: string;
  utf16Start: number;
  utf16End: number;
  rects: FlowRect[];
}

export interface EditorRichParagraph {
  paragraph: RichParagraph;
  style: FlowParagraph["style"];
}

export interface EditorSnapshotRequestV1 {
  version: 1;
  content: FlowRect;
  paragraphs: EditorRichParagraph[];
  objects: AnchoredFlowObjectV1[];
  caretWidthQ26_6?: Q26_6;
}

/** ABI-v4 editor contract. Rust also accepts v1 with horizontal defaults. */
export interface EditorSnapshotRequestV2 {
  version: 2;
  content: FlowRect;
  paragraphs: EditorRichParagraph[];
  objects: AnchoredFlowObjectV1[];
  caretWidthQ26_6?: Q26_6;
}

export interface PositionedLine {
  paragraphId: string;
  lineIndex: number;
  documentLineIndex: number;
  topQ26_6: Q26_6;
  baselineQ26_6: Q26_6;
  writingMode: WritingMode;
  blockStartQ26_6: Q26_6;
  inlineStartQ26_6: Q26_6;
  fragmentCount: number;
}

export interface PositionedCaretStop {
  position: CanonicalTextPosition;
  rect: FlowRect;
  lineIndex: number;
  documentLineIndex: number;
  fragmentIndex: number;
  visualIndex: number;
  writingMode: WritingMode;
}

export interface PositionedCluster {
  paragraphId: string;
  utf16Start: number;
  utf16End: number;
  direction: CanonicalTextDirection;
  bidiLevel: number;
  orientation: GlyphOrientation;
  writingMode: WritingMode;
  rect: FlowRect;
  lineIndex: number;
  documentLineIndex: number;
  fragmentIndex: number;
  visualIndex: number;
  caretStops: PositionedCaretStop[];
}

export interface PositionedGlyph {
  paragraphId: string;
  runIndex: number;
  glyphIndex: number;
  glyphId: number;
  clusterUtf16: number;
  pageXQ26_6: Q26_6;
  pageYQ26_6: Q26_6;
  xAdvanceQ26_6: Q26_6;
  yAdvanceQ26_6: Q26_6;
  xOffsetQ26_6: Q26_6;
  yOffsetQ26_6: Q26_6;
  orientation: GlyphOrientation;
  writingMode: WritingMode;
  lineIndex: number;
  fragmentIndex: number;
}

export interface EditorGeometrySnapshotData {
  layout: FlowLayout;
  lines: PositionedLine[];
  clusters: PositionedCluster[];
  glyphs: PositionedGlyph[];
  caretStops: PositionedCaretStop[];
  caretWidthQ26_6: Q26_6;
}

export interface EditorCaretGeometry {
  position: CanonicalTextPosition;
  rect: FlowRect;
  lineIndex: number;
  fragmentIndex: number;
}

export interface EditorCaretMovement {
  position: CanonicalTextPosition;
  rect: FlowRect;
  preferredXQ26_6: Q26_6;
  preferredInlineQ26_6: Q26_6;
}

export type CaretMovementDirection = "left" | "right" | "up" | "down";

export function dipToLayoutUnit(value: number): Q26_6 {
  return Math.round(value * LAYOUT_UNITS_PER_DIP);
}

export function layoutUnitToDip(value: Q26_6): number {
  return value / LAYOUT_UNITS_PER_DIP;
}
