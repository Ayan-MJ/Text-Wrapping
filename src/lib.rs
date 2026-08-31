//! LastDraft's deterministic, cross-platform anchored-object flow engine.
//!
//! The core intentionally does not shape text. It consumes grapheme/cluster
//! advances from LastDraft's canonical shaping stage and owns the geometry that
//! places those clusters around anchored media.

mod ffi;
mod fixed;
mod geometry;
mod layout;
mod model;
mod wire;

pub use fixed::{LayoutUnit, Normalized, BLOCK_OFFSET_SCALE, LAYOUT_UNITS_PER_DIP};
pub use geometry::{Insets, Interval, Rect};
pub use layout::{layout, normalized_placement_for_drag, LayoutError};
pub use model::{
    AnchorAffinity, Cluster, ExclusionRules, FlowFragment, FlowLayout, FlowLine, FlowObject,
    LayoutRequest, NormalizedPlacement, ObjectLayoutMode, ObjectSize, Paragraph, ParagraphLayout,
    ParagraphStyle, ResolvedObject, TextAnchor,
};
pub use wire::layout_json;
