//! LastDraft's deterministic, cross-platform anchored-object flow engine.
//!
//! The v1 flow API still accepts pre-shaped clusters for compatibility. The
//! rich-text API owns canonical font registration and HarfBuzz-compatible
//! shaping, then feeds the same exclusion geometry on native and WebAssembly.

mod editor;
mod editor_snapshot;
mod editor_wire;
// The versioned C ABI. Public so a parity test can call the same entry
// points a native host does, rather than an internal shortcut that could
// diverge from what ships.
pub mod ffi;
mod fixed;
mod geometry;
mod layout;
mod model;
mod rich_text;
mod rich_wire;
mod shaping;
mod wire;

pub use editor::{
    caret_geometry, selection_geometry, text_position_at_point, CaretGeometry, SelectionGeometry,
    TextPosition,
};
pub use editor_snapshot::{
    CaretMovement, CaretMovementDirection, EditorCaretGeometry, EditorGeometryError,
    EditorGeometrySnapshot, EditorSelectionGeometry, EditorTextPosition, PositionedCaretStop,
    PositionedCluster, PositionedGlyph, PositionedLine,
};
pub use fixed::{LayoutUnit, Normalized, BLOCK_OFFSET_SCALE, LAYOUT_UNITS_PER_DIP};
pub use geometry::{Insets, Interval, LogicalInsets, Rect};
pub use layout::{
    layout, normalized_placement_for_drag, normalized_placement_for_drag_in_mode, LayoutError,
};
pub use model::{
    AnchorAffinity, Cluster, ExclusionRules, FlowFragment, FlowLayout, FlowLine, FlowObject,
    LayoutRequest, NormalizedPlacement, ObjectLayoutMode, ObjectSize, Paragraph, ParagraphLayout,
    ParagraphStyle, ResolvedObject, TextAlignment, TextAnchor,
};
pub use rich_text::{
    FontFeature, FontVariation, GlyphOrientation, InvalidOpenTypeTag, OpenTypeTag,
    ParagraphDirection, RichParagraph, RichTextRun, RichTextStyle, TextDirection, TextOrientation,
    WritingMode,
};
pub use shaping::{
    CanonicalShaper, ClusterCaretStop, FontDescriptor, ShapedCluster, ShapedGlyph, ShapedParagraph,
    ShapedRun, ShapingError,
};
pub use wire::layout_json;
