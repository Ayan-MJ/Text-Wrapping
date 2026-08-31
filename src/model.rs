use crate::{Insets, LayoutUnit, Normalized, Rect};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AnchorAffinity {
    Upstream,
    #[default]
    Downstream,
}

/// Semantic location in LastDraft's document model. UTF-16 is deliberate: it
/// is the common indexing representation of Swift, Kotlin/JVM, JavaScript and
/// .NET text APIs. `paragraph_id` must remain stable across edits.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextAnchor {
    pub paragraph_id: String,
    pub utf16_offset: u32,
    pub affinity: AnchorAffinity,
}

/// User-authored placement relative to the semantic anchor, never to document
/// coordinates. `inline_position` maps over the object's available horizontal
/// travel: 0=start, 65,535=end. `block_offset` is signed 1/1024 line heights
/// from the top of the anchor's reference line.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NormalizedPlacement {
    pub inline_position: Normalized,
    pub block_offset: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectSize {
    pub ideal_width: LayoutUnit,
    pub ideal_height: LayoutUnit,
    /// Maximum width as a normalized fraction of the content width.
    pub max_inline_fraction: Normalized,
}

impl Default for ObjectSize {
    fn default() -> Self {
        Self {
            ideal_width: LayoutUnit::ZERO,
            ideal_height: LayoutUnit::ZERO,
            max_inline_fraction: Normalized::END,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExclusionRules {
    pub margin: Insets,
    /// Corridors narrower than this are not used for text. When neither side
    /// reaches this width, the object enters automatic BlockFallback mode.
    pub minimum_fragment_width: LayoutUnit,
}

impl Default for ExclusionRules {
    fn default() -> Self {
        Self {
            margin: Insets::default(),
            minimum_fragment_width: LayoutUnit::ZERO,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowObject {
    pub id: String,
    pub anchor: TextAnchor,
    pub placement: NormalizedPlacement,
    pub size: ObjectSize,
    pub exclusion: ExclusionRules,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphStyle {
    pub line_height: LayoutUnit,
    pub ascent: LayoutUnit,
    pub space_after: LayoutUnit,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            line_height: LayoutUnit::from_raw(20 * 64),
            ascent: LayoutUnit::from_raw(15 * 64),
            space_after: LayoutUnit::ZERO,
        }
    }
}

/// A shaped grapheme cluster. A canonical shaping layer must supply these in
/// logical text order. Break opportunities are determined before flow layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cluster {
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub advance: LayoutUnit,
    pub can_break_after: bool,
    pub is_whitespace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Paragraph {
    pub id: String,
    pub utf16_len: u32,
    pub style: ParagraphStyle,
    pub clusters: Vec<Cluster>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutRequest {
    pub content: Rect,
    pub paragraphs: Vec<Paragraph>,
    pub objects: Vec<FlowObject>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectLayoutMode {
    /// Position came directly from the user's normalized placement. Text may
    /// use every viable fragment on both sides; there is no left/right mode.
    UserPositioned,
    /// Narrow-view fallback. The stored user placement is untouched, the
    /// object is centered for this layout, and its vertical band blocks the
    /// full content width so text resumes below it.
    BlockFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedObject {
    pub id: String,
    pub anchor: TextAnchor,
    /// Stable local reference used to turn pointer movement back into a
    /// normalized placement. This value is layout output and is never stored.
    pub anchor_reference_top: LayoutUnit,
    pub frame: Rect,
    pub exclusion_frame: Rect,
    pub mode: ObjectLayoutMode,
    pub minimum_fragment_width: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowFragment {
    pub rect: Rect,
    pub utf16_start: u32,
    pub utf16_end: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowLine {
    pub top: LayoutUnit,
    pub baseline: LayoutUnit,
    pub fragments: Vec<FlowFragment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphLayout {
    pub paragraph_id: String,
    pub top: LayoutUnit,
    pub bottom: LayoutUnit,
    pub lines: Vec<FlowLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowLayout {
    pub paragraphs: Vec<ParagraphLayout>,
    pub objects: Vec<ResolvedObject>,
    pub content_height: LayoutUnit,
}
