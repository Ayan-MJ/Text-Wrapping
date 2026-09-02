use serde::{Deserialize, Serialize};

use crate::rich_wire::{decode_wire_paragraph, WireRichParagraph};
use crate::{
    layout, AnchorAffinity, CanonicalShaper, CaretMovementDirection, EditorGeometrySnapshot,
    EditorTextPosition, ExclusionRules, FlowLayout, FlowObject, GlyphOrientation,
    LogicalInsets,
    LayoutRequest, LayoutUnit, Normalized, NormalizedPlacement, ObjectLayoutMode, ObjectSize,
    ParagraphStyle, Rect, TextAlignment, TextAnchor, TextDirection, WritingMode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEditorSnapshotRequest {
    version: u32,
    content: WireRect,
    paragraphs: Vec<WireEditorParagraph>,
    objects: Vec<WireObject>,
    #[serde(rename = "caretWidthQ26_6", default = "default_caret_width")]
    caret_width_q26_6: i32,
}

fn default_caret_width() -> i32 {
    64
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEditorParagraph {
    paragraph: WireRichParagraph,
    style: WireParagraphStyle,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRect {
    #[serde(rename = "xQ26_6")]
    x_q26_6: i32,
    #[serde(rename = "yQ26_6")]
    y_q26_6: i32,
    #[serde(rename = "widthQ26_6")]
    width_q26_6: i32,
    #[serde(rename = "heightQ26_6")]
    height_q26_6: i32,
}

impl From<WireRect> for Rect {
    fn from(value: WireRect) -> Self {
        Self::new(
            LayoutUnit::from_raw(value.x_q26_6),
            LayoutUnit::from_raw(value.y_q26_6),
            LayoutUnit::from_raw(value.width_q26_6),
            LayoutUnit::from_raw(value.height_q26_6),
        )
    }
}

impl From<Rect> for WireRect {
    fn from(value: Rect) -> Self {
        Self {
            x_q26_6: value.x.raw(),
            y_q26_6: value.y.raw(),
            width_q26_6: value.width.raw(),
            height_q26_6: value.height.raw(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireParagraphStyle {
    #[serde(rename = "lineHeightQ26_6")]
    line_height_q26_6: i32,
    #[serde(rename = "ascentQ26_6")]
    ascent_q26_6: i32,
    #[serde(rename = "spaceAfterQ26_6")]
    space_after_q26_6: i32,
    // Both default, so a request that never mentions them is byte-identical to
    // one written before they existed. The adapter omits them at their default
    // for the same reason.
    #[serde(default)]
    alignment: WireTextAlignment,
    #[serde(default, rename = "indentQ26_6")]
    indent_q26_6: i32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WireTextAlignment {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

impl From<WireTextAlignment> for TextAlignment {
    fn from(value: WireTextAlignment) -> Self {
        match value {
            WireTextAlignment::Start => Self::Start,
            WireTextAlignment::Center => Self::Center,
            WireTextAlignment::End => Self::End,
            WireTextAlignment::Justify => Self::Justify,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireObject {
    id: String,
    anchor: WirePosition,
    placement: WirePlacement,
    size: WireSize,
    exclusion: WireExclusion,
    responsive_policy: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WirePosition {
    paragraph_id: String,
    utf16_offset: u32,
    affinity: WireAffinity,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireAffinity {
    Upstream,
    Downstream,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WirePlacement {
    inline_u16: u16,
    block_offset1024: i32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireSize {
    #[serde(rename = "idealWidthQ26_6")]
    ideal_width_q26_6: i32,
    #[serde(rename = "idealHeightQ26_6")]
    ideal_height_q26_6: i32,
    max_inline_u16: u16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireExclusion {
    shape: String,
    #[serde(rename = "marginStartQ26_6")]
    margin_start_q26_6: i32,
    #[serde(rename = "marginBeforeQ26_6")]
    margin_before_q26_6: i32,
    #[serde(rename = "marginEndQ26_6")]
    margin_end_q26_6: i32,
    #[serde(rename = "marginAfterQ26_6")]
    margin_after_q26_6: i32,
    #[serde(rename = "minimumFragmentWidthQ26_6")]
    minimum_fragment_width_q26_6: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireEditorError {
    pub(crate) code: String,
    pub(crate) message: String,
}

pub(crate) fn editor_snapshot_from_json(
    shaper: &CanonicalShaper,
    input: &[u8],
) -> Result<EditorGeometrySnapshot, WireEditorError> {
    let wire: WireEditorSnapshotRequest =
        serde_json::from_slice(input).map_err(|error| wire_error("invalid_json", error))?;
    if !matches!(wire.version, 1 | 2) {
        return Err(error(
            "unsupported_version",
            format!("unsupported editor snapshot version {}", wire.version),
        ));
    }
    let content: Rect = wire.content.into();
    let mut shaped = Vec::with_capacity(wire.paragraphs.len());
    let mut paragraphs = Vec::with_capacity(wire.paragraphs.len());
    for wire_paragraph in wire.paragraphs {
        let rich = decode_wire_paragraph(wire_paragraph.paragraph)
            .map_err(|source| error(source.code, source.message))?;
        let shaped_paragraph = shaper
            .shape_paragraph(&rich)
            .map_err(|source| wire_error("shaping_error", source))?;
        paragraphs.push(shaped_paragraph.to_flow_paragraph(ParagraphStyle {
            line_height: LayoutUnit::from_raw(wire_paragraph.style.line_height_q26_6),
            ascent: LayoutUnit::from_raw(wire_paragraph.style.ascent_q26_6),
            space_after: LayoutUnit::from_raw(wire_paragraph.style.space_after_q26_6),
            alignment: wire_paragraph.style.alignment.into(),
            indent: LayoutUnit::from_raw(wire_paragraph.style.indent_q26_6),
        }));
        shaped.push(shaped_paragraph);
    }
    let objects = wire
        .objects
        .into_iter()
        .map(decode_object)
        .collect::<Result<Vec<_>, _>>()?;
    let request = LayoutRequest {
        content,
        paragraphs,
        objects,
    };
    let flow = layout(&request).map_err(|source| wire_error("layout_error", source))?;
    EditorGeometrySnapshot::new(
        flow,
        shaped,
        content,
        LayoutUnit::from_raw(wire.caret_width_q26_6),
    )
    .map_err(|source| wire_error("editor_geometry_error", source))
}

fn decode_object(object: WireObject) -> Result<FlowObject, WireEditorError> {
    if object.responsive_policy != "scale-then-centered-block-v1" {
        return Err(error(
            "unsupported_responsive_policy",
            format!(
                "object {} uses unsupported responsive policy {}",
                object.id, object.responsive_policy
            ),
        ));
    }
    if object.exclusion.shape != "bounds" {
        return Err(error(
            "unsupported_exclusion_shape",
            format!(
                "object {} uses unsupported exclusion shape {}",
                object.id, object.exclusion.shape
            ),
        ));
    }
    Ok(FlowObject {
        id: object.id,
        anchor: TextAnchor {
            paragraph_id: object.anchor.paragraph_id,
            utf16_offset: object.anchor.utf16_offset,
            affinity: object.anchor.affinity.into(),
        },
        placement: NormalizedPlacement {
            inline_position: Normalized::from_raw(object.placement.inline_u16),
            block_offset: object.placement.block_offset1024,
        },
        size: ObjectSize {
            ideal_width: LayoutUnit::from_raw(object.size.ideal_width_q26_6),
            ideal_height: LayoutUnit::from_raw(object.size.ideal_height_q26_6),
            max_inline_fraction: Normalized::from_raw(object.size.max_inline_u16),
        },
        exclusion: ExclusionRules {
            margin: LogicalInsets {
                inline_start: LayoutUnit::from_raw(object.exclusion.margin_start_q26_6),
                block_start: LayoutUnit::from_raw(object.exclusion.margin_before_q26_6),
                inline_end: LayoutUnit::from_raw(object.exclusion.margin_end_q26_6),
                block_end: LayoutUnit::from_raw(object.exclusion.margin_after_q26_6),
            },
            minimum_fragment_width: LayoutUnit::from_raw(
                object.exclusion.minimum_fragment_width_q26_6,
            ),
        },
    })
}

impl From<WireAffinity> for AnchorAffinity {
    fn from(value: WireAffinity) -> Self {
        match value {
            WireAffinity::Upstream => Self::Upstream,
            WireAffinity::Downstream => Self::Downstream,
        }
    }
}

impl From<AnchorAffinity> for WireAffinity {
    fn from(value: AnchorAffinity) -> Self {
        match value {
            AnchorAffinity::Upstream => Self::Upstream,
            AnchorAffinity::Downstream => Self::Downstream,
        }
    }
}

fn error(code: impl Into<String>, message: impl Into<String>) -> WireEditorError {
    WireEditorError {
        code: code.into(),
        message: message.into(),
    }
}

fn wire_error(code: impl Into<String>, source: impl std::fmt::Display) -> WireEditorError {
    error(code, source.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireSnapshotEnvelope<'a> {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<WireSnapshot<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WireEditorError>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireSnapshot<'a> {
    layout: WireFlowLayout,
    lines: Vec<WirePositionedLine>,
    clusters: Vec<WirePositionedCluster>,
    glyphs: Vec<WirePositionedGlyph>,
    caret_stops: Vec<WireCaretStop>,
    #[serde(rename = "caretWidthQ26_6")]
    caret_width_q26_6: i32,
    #[serde(skip)]
    _snapshot: std::marker::PhantomData<&'a EditorGeometrySnapshot>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFlowLayout {
    #[serde(rename = "contentHeightQ26_6")]
    content_height_q26_6: i32,
    #[serde(rename = "contentWidthQ26_6")]
    content_width_q26_6: i32,
    paragraphs: Vec<WireParagraphLayout>,
    objects: Vec<WireResolvedObject>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireParagraphLayout {
    paragraph_id: String,
    #[serde(rename = "topQ26_6")]
    top_q26_6: i32,
    #[serde(rename = "bottomQ26_6")]
    bottom_q26_6: i32,
    bounds: WireRect,
    base_direction: WireDirection,
    writing_mode: WireWritingMode,
    lines: Vec<WireFlowLine>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFlowLine {
    #[serde(rename = "topQ26_6")]
    top_q26_6: i32,
    #[serde(rename = "baselineQ26_6")]
    baseline_q26_6: i32,
    writing_mode: WireWritingMode,
    #[serde(rename = "blockStartQ26_6")]
    block_start_q26_6: i32,
    #[serde(rename = "inlineStartQ26_6")]
    inline_start_q26_6: i32,
    fragments: Vec<WireFragment>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFragment {
    rect: WireRect,
    utf16_start: u32,
    utf16_end: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireResolvedObject {
    id: String,
    anchor: WirePosition,
    #[serde(rename = "anchorReferenceTopQ26_6")]
    anchor_reference_top_q26_6: i32,
    #[serde(rename = "anchorReferenceBlockStartQ26_6")]
    anchor_reference_block_start_q26_6: i32,
    frame: WireRect,
    exclusion_frame: WireRect,
    mode: WireObjectMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum WireObjectMode {
    UserPositioned,
    BlockFallback,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WirePositionedLine {
    paragraph_id: String,
    line_index: usize,
    document_line_index: usize,
    #[serde(rename = "topQ26_6")]
    top_q26_6: i32,
    #[serde(rename = "baselineQ26_6")]
    baseline_q26_6: i32,
    writing_mode: WireWritingMode,
    #[serde(rename = "blockStartQ26_6")]
    block_start_q26_6: i32,
    #[serde(rename = "inlineStartQ26_6")]
    inline_start_q26_6: i32,
    fragment_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireCaretStop {
    position: WirePosition,
    rect: WireRect,
    line_index: usize,
    document_line_index: usize,
    fragment_index: usize,
    visual_index: usize,
    writing_mode: WireWritingMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WirePositionedCluster {
    paragraph_id: String,
    utf16_start: u32,
    utf16_end: u32,
    direction: WireDirection,
    bidi_level: u8,
    orientation: WireGlyphOrientation,
    writing_mode: WireWritingMode,
    rect: WireRect,
    line_index: usize,
    document_line_index: usize,
    fragment_index: usize,
    visual_index: usize,
    caret_stops: Vec<WireCaretStop>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireWritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireGlyphOrientation {
    Upright,
    Sideways,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WirePositionedGlyph {
    paragraph_id: String,
    run_index: usize,
    glyph_index: usize,
    glyph_id: u32,
    cluster_utf16: u32,
    #[serde(rename = "pageXQ26_6")]
    page_x_q26_6: i32,
    #[serde(rename = "pageYQ26_6")]
    page_y_q26_6: i32,
    #[serde(rename = "xAdvanceQ26_6")]
    x_advance_q26_6: i32,
    #[serde(rename = "yAdvanceQ26_6")]
    y_advance_q26_6: i32,
    #[serde(rename = "xOffsetQ26_6")]
    x_offset_q26_6: i32,
    #[serde(rename = "yOffsetQ26_6")]
    y_offset_q26_6: i32,
    orientation: WireGlyphOrientation,
    writing_mode: WireWritingMode,
    line_index: usize,
    fragment_index: usize,
}

pub(crate) fn editor_snapshot_json(
    result: Result<&EditorGeometrySnapshot, WireEditorError>,
) -> Vec<u8> {
    let envelope = match result {
        Ok(snapshot) => WireSnapshotEnvelope {
            version: 2,
            snapshot: Some(snapshot.into()),
            error: None,
        },
        Err(error) => WireSnapshotEnvelope {
            version: 2,
            snapshot: None,
            error: Some(error),
        },
    };
    encode(&envelope)
}

impl<'a> From<&'a EditorGeometrySnapshot> for WireSnapshot<'a> {
    fn from(snapshot: &'a EditorGeometrySnapshot) -> Self {
        Self {
            layout: (&snapshot.layout).into(),
            lines: snapshot
                .lines
                .iter()
                .map(|line| WirePositionedLine {
                    paragraph_id: line.paragraph_id.clone(),
                    line_index: line.line_index,
                    document_line_index: line.document_line_index,
                    top_q26_6: line.top.raw(),
                    baseline_q26_6: line.baseline.raw(),
                    writing_mode: line.writing_mode.into(),
                    block_start_q26_6: line.block_start.raw(),
                    inline_start_q26_6: line.inline_start.raw(),
                    fragment_count: line.fragment_count,
                })
                .collect(),
            clusters: snapshot
                .clusters
                .iter()
                .map(|cluster| WirePositionedCluster {
                    paragraph_id: cluster.paragraph_id.clone(),
                    utf16_start: cluster.utf16_start,
                    utf16_end: cluster.utf16_end,
                    direction: cluster.direction.into(),
                    bidi_level: cluster.bidi_level,
                    orientation: cluster.orientation.into(),
                    writing_mode: cluster.writing_mode.into(),
                    rect: cluster.rect.into(),
                    line_index: cluster.line_index,
                    document_line_index: cluster.document_line_index,
                    fragment_index: cluster.fragment_index,
                    visual_index: cluster.visual_index,
                    caret_stops: cluster.caret_stops.iter().map(Into::into).collect(),
                })
                .collect(),
            glyphs: snapshot
                .glyphs
                .iter()
                .map(|glyph| WirePositionedGlyph {
                    paragraph_id: glyph.paragraph_id.clone(),
                    run_index: glyph.run_index,
                    glyph_index: glyph.glyph_index,
                    glyph_id: glyph.glyph_id,
                    cluster_utf16: glyph.cluster_utf16,
                    page_x_q26_6: glyph.page_x.raw(),
                    page_y_q26_6: glyph.page_y.raw(),
                    x_advance_q26_6: glyph.x_advance.raw(),
                    y_advance_q26_6: glyph.y_advance.raw(),
                    x_offset_q26_6: glyph.x_offset.raw(),
                    y_offset_q26_6: glyph.y_offset.raw(),
                    orientation: glyph.orientation.into(),
                    writing_mode: glyph.writing_mode.into(),
                    line_index: glyph.line_index,
                    fragment_index: glyph.fragment_index,
                })
                .collect(),
            caret_stops: snapshot.caret_stops.iter().map(Into::into).collect(),
            caret_width_q26_6: snapshot.caret_width().raw(),
            _snapshot: std::marker::PhantomData,
        }
    }
}

impl From<&FlowLayout> for WireFlowLayout {
    fn from(layout: &FlowLayout) -> Self {
        Self {
            content_height_q26_6: layout.content_height.raw(),
            content_width_q26_6: layout.content_width.raw(),
            paragraphs: layout
                .paragraphs
                .iter()
                .map(|paragraph| WireParagraphLayout {
                    paragraph_id: paragraph.paragraph_id.clone(),
                    top_q26_6: paragraph.top.raw(),
                    bottom_q26_6: paragraph.bottom.raw(),
                    bounds: paragraph.bounds.into(),
                    base_direction: paragraph.base_direction.into(),
                    writing_mode: paragraph.writing_mode.into(),
                    lines: paragraph
                        .lines
                        .iter()
                        .map(|line| WireFlowLine {
                            top_q26_6: line.top.raw(),
                            baseline_q26_6: line.baseline.raw(),
                            writing_mode: line.writing_mode.into(),
                            block_start_q26_6: line.block_start.raw(),
                            inline_start_q26_6: line.inline_start.raw(),
                            fragments: line
                                .fragments
                                .iter()
                                .map(|fragment| WireFragment {
                                    rect: fragment.rect.into(),
                                    utf16_start: fragment.utf16_start,
                                    utf16_end: fragment.utf16_end,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            objects: layout
                .objects
                .iter()
                .map(|object| WireResolvedObject {
                    id: object.id.clone(),
                    anchor: WirePosition {
                        paragraph_id: object.anchor.paragraph_id.clone(),
                        utf16_offset: object.anchor.utf16_offset,
                        affinity: object.anchor.affinity.into(),
                    },
                    anchor_reference_top_q26_6: object.anchor_reference_top.raw(),
                    anchor_reference_block_start_q26_6: object.anchor_reference_block_start.raw(),
                    frame: object.frame.into(),
                    exclusion_frame: object.exclusion_frame.into(),
                    mode: match object.mode {
                        ObjectLayoutMode::UserPositioned => WireObjectMode::UserPositioned,
                        ObjectLayoutMode::BlockFallback => WireObjectMode::BlockFallback,
                    },
                })
                .collect(),
        }
    }
}

impl From<&crate::PositionedCaretStop> for WireCaretStop {
    fn from(stop: &crate::PositionedCaretStop) -> Self {
        Self {
            position: (&stop.position).into(),
            rect: stop.rect.into(),
            line_index: stop.line_index,
            document_line_index: stop.document_line_index,
            fragment_index: stop.fragment_index,
            visual_index: stop.visual_index,
            writing_mode: stop.writing_mode.into(),
        }
    }
}

impl From<&EditorTextPosition> for WirePosition {
    fn from(position: &EditorTextPosition) -> Self {
        Self {
            paragraph_id: position.paragraph_id.clone(),
            utf16_offset: position.utf16_offset,
            affinity: position.affinity.into(),
        }
    }
}

impl From<WirePosition> for EditorTextPosition {
    fn from(position: WirePosition) -> Self {
        Self {
            paragraph_id: position.paragraph_id,
            utf16_offset: position.utf16_offset,
            affinity: position.affinity.into(),
        }
    }
}

impl From<TextDirection> for WireDirection {
    fn from(value: TextDirection) -> Self {
        match value {
            TextDirection::LeftToRight => Self::LeftToRight,
            TextDirection::RightToLeft => Self::RightToLeft,
        }
    }
}

impl From<WritingMode> for WireWritingMode {
    fn from(value: WritingMode) -> Self {
        match value {
            WritingMode::HorizontalTb => Self::HorizontalTb,
            WritingMode::VerticalRl => Self::VerticalRl,
            WritingMode::VerticalLr => Self::VerticalLr,
        }
    }
}

impl From<GlyphOrientation> for WireGlyphOrientation {
    fn from(value: GlyphOrientation) -> Self {
        match value {
            GlyphOrientation::Upright => Self::Upright,
            GlyphOrientation::Sideways => Self::Sideways,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum WireEditorQuery {
    Caret {
        version: u32,
        position: WirePosition,
    },
    HitTest {
        version: u32,
        #[serde(rename = "xQ26_6")]
        x_q26_6: i32,
        #[serde(rename = "yQ26_6")]
        y_q26_6: i32,
    },
    Selection {
        version: u32,
        paragraph_id: String,
        utf16_start: u32,
        utf16_end: u32,
    },
    Move {
        version: u32,
        position: WirePosition,
        direction: WireMovementDirection,
        #[serde(rename = "preferredInlineQ26_6", alias = "preferredXQ26_6", default)]
        preferred_inline_q26_6: Option<i32>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WireMovementDirection {
    Left,
    Right,
    Up,
    Down,
}

impl From<WireMovementDirection> for CaretMovementDirection {
    fn from(value: WireMovementDirection) -> Self {
        match value {
            WireMovementDirection::Left => Self::Left,
            WireMovementDirection::Right => Self::Right,
            WireMovementDirection::Up => Self::Up,
            WireMovementDirection::Down => Self::Down,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireQueryEnvelope {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<WireEditorQueryResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WireEditorError>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum WireEditorQueryResult {
    Caret { caret: WireCaretGeometry },
    HitTest { position: Option<WirePosition> },
    Selection { selection: WireSelectionGeometry },
    Move { movement: WireMovement },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireCaretGeometry {
    position: WirePosition,
    rect: WireRect,
    line_index: usize,
    fragment_index: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireSelectionGeometry {
    paragraph_id: String,
    utf16_start: u32,
    utf16_end: u32,
    rects: Vec<WireRect>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireMovement {
    position: WirePosition,
    rect: WireRect,
    #[serde(rename = "preferredXQ26_6")]
    preferred_x_q26_6: i32,
    #[serde(rename = "preferredInlineQ26_6")]
    preferred_inline_q26_6: i32,
}

pub(crate) fn editor_query_json(snapshot: &EditorGeometrySnapshot, input: &[u8]) -> Vec<u8> {
    let result = decode_query(input).and_then(|query| run_query(snapshot, query));
    let envelope = match result {
        Ok(result) => WireQueryEnvelope {
            version: 2,
            result: Some(result),
            error: None,
        },
        Err(error) => WireQueryEnvelope {
            version: 2,
            result: None,
            error: Some(error),
        },
    };
    encode(&envelope)
}

fn decode_query(input: &[u8]) -> Result<WireEditorQuery, WireEditorError> {
    let query: WireEditorQuery =
        serde_json::from_slice(input).map_err(|source| wire_error("invalid_json", source))?;
    let version = match &query {
        WireEditorQuery::Caret { version, .. }
        | WireEditorQuery::HitTest { version, .. }
        | WireEditorQuery::Selection { version, .. }
        | WireEditorQuery::Move { version, .. } => *version,
    };
    if !matches!(version, 1 | 2) {
        return Err(error(
            "unsupported_version",
            format!("unsupported editor query version {version}"),
        ));
    }
    Ok(query)
}

fn run_query(
    snapshot: &EditorGeometrySnapshot,
    query: WireEditorQuery,
) -> Result<WireEditorQueryResult, WireEditorError> {
    match query {
        WireEditorQuery::Caret { position, .. } => {
            let caret = snapshot
                .caret(&position.into())
                .map_err(|source| wire_error("editor_geometry_error", source))?;
            Ok(WireEditorQueryResult::Caret {
                caret: WireCaretGeometry {
                    position: (&caret.position).into(),
                    rect: caret.rect.into(),
                    line_index: caret.line_index,
                    fragment_index: caret.fragment_index,
                },
            })
        }
        WireEditorQuery::HitTest {
            x_q26_6, y_q26_6, ..
        } => Ok(WireEditorQueryResult::HitTest {
            position: snapshot
                .hit_test(LayoutUnit::from_raw(x_q26_6), LayoutUnit::from_raw(y_q26_6))
                .as_ref()
                .map(Into::into),
        }),
        WireEditorQuery::Selection {
            paragraph_id,
            utf16_start,
            utf16_end,
            ..
        } => {
            let selection = snapshot
                .selection(&paragraph_id, utf16_start, utf16_end)
                .map_err(|source| wire_error("editor_geometry_error", source))?;
            Ok(WireEditorQueryResult::Selection {
                selection: WireSelectionGeometry {
                    paragraph_id: selection.paragraph_id,
                    utf16_start: selection.utf16_start,
                    utf16_end: selection.utf16_end,
                    rects: selection.rects.into_iter().map(Into::into).collect(),
                },
            })
        }
        WireEditorQuery::Move {
            position,
            direction,
            preferred_inline_q26_6,
            ..
        } => {
            let movement = snapshot
                .move_caret(
                    &position.into(),
                    direction.into(),
                    preferred_inline_q26_6.map(LayoutUnit::from_raw),
                )
                .map_err(|source| wire_error("editor_geometry_error", source))?;
            Ok(WireEditorQueryResult::Move {
                movement: WireMovement {
                    position: (&movement.position).into(),
                    rect: movement.rect.into(),
                    preferred_x_q26_6: movement.preferred_x.raw(),
                    preferred_inline_q26_6: movement.preferred_x.raw(),
                },
            })
        }
    }
}

fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| {
        br#"{"version":2,"error":{"code":"serialization_error","message":"failed to serialize editor geometry response"}}"#.to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> EditorGeometrySnapshot {
        let mut shaper = CanonicalShaper::new();
        shaper
            .register_font(
                "fixture-noto-sans",
                &include_bytes!("../fixtures/fonts/NotoSans-Variable.ttf")[..],
                0,
            )
            .unwrap();
        shaper
            .register_font(
                "fixture-noto-sans-italic",
                &include_bytes!("../fixtures/fonts/NotoSans-Italic-Variable.ttf")[..],
                0,
            )
            .unwrap();
        shaper
            .register_font(
                "fixture-noto-devanagari",
                &include_bytes!("../fixtures/fonts/NotoSansDevanagari-Variable.ttf")[..],
                0,
            )
            .unwrap();
        shaper
            .register_font(
                "fixture-noto-arabic",
                &include_bytes!("../fixtures/fonts/NotoSansArabic-Variable.ttf")[..],
                0,
            )
            .unwrap();
        editor_snapshot_from_json(
            &shaper,
            include_bytes!("../fixtures/editor/unicode-exclusion-v1.json"),
        )
        .unwrap()
    }

    #[test]
    fn shared_fixture_positions_unicode_runs_and_both_exclusion_sides() {
        let snapshot = fixture_snapshot();
        assert!(snapshot.glyphs.iter().any(|glyph| glyph.run_index == 0));
        assert!(snapshot.glyphs.iter().any(|glyph| glyph.run_index == 1));
        assert!(snapshot
            .clusters
            .iter()
            .any(|cluster| cluster.utf16_end - cluster.utf16_start > 1));
        assert!(snapshot
            .clusters
            .iter()
            .any(|cluster| cluster.direction == TextDirection::RightToLeft));
        assert!(!snapshot
            .caret_stops
            .iter()
            .any(|stop| stop.position.utf16_offset == 26));
        assert_eq!(snapshot.layout.paragraphs[0].lines[0].fragments.len(), 2);
        assert!(snapshot.glyphs.iter().all(|glyph| glyph.page_y.raw() >= 0));
    }

    #[test]
    fn shared_fixture_queries_selection_hit_test_and_visual_movement() {
        let snapshot = fixture_snapshot();
        let fragments = &snapshot.layout.paragraphs[0].lines[0].fragments;
        let boundary = fragments[0].utf16_end;
        assert_eq!(fragments[1].utf16_start, boundary);
        let upstream = snapshot
            .caret(&EditorTextPosition {
                paragraph_id: "unicode-exclusion".into(),
                utf16_offset: boundary,
                affinity: AnchorAffinity::Upstream,
            })
            .unwrap();
        let downstream = snapshot
            .caret(&EditorTextPosition {
                paragraph_id: "unicode-exclusion".into(),
                utf16_offset: boundary,
                affinity: AnchorAffinity::Downstream,
            })
            .unwrap();
        assert!(upstream.rect.x < downstream.rect.x);
        assert_eq!(
            snapshot
                .hit_test(upstream.rect.x + LayoutUnit::from_raw(1), upstream.rect.y)
                .unwrap()
                .affinity,
            AnchorAffinity::Upstream
        );
        assert_eq!(
            snapshot
                .hit_test(
                    downstream.rect.x - LayoutUnit::from_raw(1),
                    downstream.rect.y
                )
                .unwrap()
                .affinity,
            AnchorAffinity::Downstream
        );
        assert!(
            snapshot
                .selection("unicode-exclusion", 1, 90)
                .unwrap()
                .rects
                .len()
                >= 3
        );
        let moved = snapshot
            .move_caret(&upstream.position, CaretMovementDirection::Right, None)
            .unwrap();
        assert_eq!(moved.position, downstream.position);
    }
}
