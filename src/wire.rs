use serde::{Deserialize, Serialize};

use crate::{
    layout, AnchorAffinity, Cluster, ExclusionRules, FlowLayout, FlowObject, GlyphOrientation,
    LayoutRequest, LayoutUnit, LogicalInsets, Normalized, NormalizedPlacement,
    ObjectLayoutMode,
    ObjectSize, Paragraph, ParagraphDirection, ParagraphStyle, Rect, TextAnchor, TextDirection,
    TextOrientation, WritingMode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireLayoutRequest {
    version: u32,
    content: WireRect,
    paragraphs: Vec<WireParagraph>,
    objects: Vec<WireObject>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireParagraph {
    id: String,
    utf16_length: u32,
    style: WireParagraphStyle,
    clusters: Vec<WireCluster>,
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
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireCluster {
    utf16_start: u32,
    utf16_end: u32,
    #[serde(rename = "advanceQ26_6")]
    advance_q26_6: i32,
    can_break_after: bool,
    #[serde(default)]
    is_whitespace: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireObject {
    id: String,
    anchor: WireAnchor,
    placement: WirePlacement,
    size: WireSize,
    exclusion: WireExclusion,
    responsive_policy: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireAnchor {
    paragraph_id: String,
    utf16_offset: u32,
    affinity: WireAffinity,
}

#[derive(Clone, Copy, Debug, Deserialize)]
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
    #[serde(rename = "marginTopQ26_6")]
    margin_top_q26_6: i32,
    #[serde(rename = "marginEndQ26_6")]
    margin_end_q26_6: i32,
    #[serde(rename = "marginBottomQ26_6")]
    margin_bottom_q26_6: i32,
    #[serde(rename = "minimumFragmentWidthQ26_6")]
    minimum_fragment_width_q26_6: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireEnvelope {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<WireLayout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WireError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireError {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireLayout {
    #[serde(rename = "contentHeightQ26_6")]
    content_height_q26_6: i32,
    paragraphs: Vec<WireParagraphLayout>,
    objects: Vec<WireResolvedObject>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireParagraphLayout {
    paragraph_id: String,
    #[serde(rename = "topQ26_6")]
    top_q26_6: i32,
    #[serde(rename = "bottomQ26_6")]
    bottom_q26_6: i32,
    lines: Vec<WireLine>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireLine {
    #[serde(rename = "topQ26_6")]
    top_q26_6: i32,
    #[serde(rename = "baselineQ26_6")]
    baseline_q26_6: i32,
    fragments: Vec<WireFragment>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFragment {
    rect: WireRect,
    utf16_start: u32,
    utf16_end: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireResolvedObject {
    id: String,
    anchor: WireOutputAnchor,
    #[serde(rename = "anchorReferenceTopQ26_6")]
    anchor_reference_top_q26_6: i32,
    frame: WireRect,
    exclusion_frame: WireRect,
    mode: WireObjectMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireOutputAnchor {
    paragraph_id: String,
    utf16_offset: u32,
    affinity: WireAffinityOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireAffinityOutput {
    Upstream,
    Downstream,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireObjectMode {
    UserPositioned,
    BlockFallback,
}

/// Layout JSON boundary shared by the C and WebAssembly ABIs.
///
/// The returned envelope is always valid JSON. Validation and layout failures
/// are represented by its `error` member so every host receives identical error
/// semantics.
pub fn layout_json(input: &[u8]) -> Vec<u8> {
    let envelope = match decode_request(input).and_then(|request| {
        layout(&request)
            .map_err(|error| WireError {
                code: "layout_error",
                message: error.to_string(),
            })
            .map(|result| encode_layout(&request, result))
    }) {
        Ok(layout) => WireEnvelope {
            version: 1,
            layout: Some(layout),
            error: None,
        },
        Err(error) => WireEnvelope {
            version: 1,
            layout: None,
            error: Some(error),
        },
    };

    serde_json::to_vec(&envelope).unwrap_or_else(|_| {
        br#"{"version":1,"error":{"code":"serialization_error","message":"failed to serialize layout response"}}"#.to_vec()
    })
}

fn decode_request(input: &[u8]) -> Result<LayoutRequest, WireError> {
    let wire: WireLayoutRequest = serde_json::from_slice(input).map_err(|error| WireError {
        code: "invalid_json",
        message: error.to_string(),
    })?;
    if wire.version != 1 {
        return Err(WireError {
            code: "unsupported_version",
            message: format!("unsupported layout request version {}", wire.version),
        });
    }

    let paragraphs = wire
        .paragraphs
        .into_iter()
        .map(|paragraph| {
            Ok(Paragraph {
                id: paragraph.id,
                utf16_len: paragraph.utf16_length,
                style: ParagraphStyle {
                    line_height: LayoutUnit::from_raw(paragraph.style.line_height_q26_6),
                    ascent: LayoutUnit::from_raw(paragraph.style.ascent_q26_6),
                    space_after: LayoutUnit::from_raw(paragraph.style.space_after_q26_6),
                    // The v1 flow request predates paragraph alignment.
                    ..Default::default()
                },
                requested_base_direction: ParagraphDirection::LeftToRight,
                base_direction: TextDirection::LeftToRight,
                writing_mode: WritingMode::HorizontalTb,
                text_orientation: TextOrientation::Mixed,
                clusters: paragraph
                    .clusters
                    .into_iter()
                    .map(|cluster| Cluster {
                        utf16_start: cluster.utf16_start,
                        utf16_end: cluster.utf16_end,
                        advance: LayoutUnit::from_raw(cluster.advance_q26_6),
                        can_break_after: cluster.can_break_after,
                        is_whitespace: cluster.is_whitespace,
                        bidi_level: 0,
                        direction: TextDirection::LeftToRight,
                        orientation: GlyphOrientation::Upright,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, WireError>>()?;

    let objects = wire
        .objects
        .into_iter()
        .map(|object| {
            if object.responsive_policy != "scale-then-centered-block-v1" {
                return Err(WireError {
                    code: "unsupported_responsive_policy",
                    message: format!(
                        "object {} uses unsupported responsive policy {}",
                        object.id, object.responsive_policy
                    ),
                });
            }
            if object.exclusion.shape != "bounds" {
                return Err(WireError {
                    code: "unsupported_exclusion_shape",
                    message: format!(
                        "object {} uses unsupported exclusion shape {}",
                        object.id, object.exclusion.shape
                    ),
                });
            }
            Ok(FlowObject {
                id: object.id,
                anchor: TextAnchor {
                    paragraph_id: object.anchor.paragraph_id,
                    utf16_offset: object.anchor.utf16_offset,
                    affinity: match object.anchor.affinity {
                        WireAffinity::Upstream => AnchorAffinity::Upstream,
                        WireAffinity::Downstream => AnchorAffinity::Downstream,
                    },
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
                    // The v1 flow request is horizontal-ltr only, where the
                    // logical and physical sides coincide.
                    margin: LogicalInsets {
                        inline_start: LayoutUnit::from_raw(object.exclusion.margin_start_q26_6),
                        block_start: LayoutUnit::from_raw(object.exclusion.margin_top_q26_6),
                        inline_end: LayoutUnit::from_raw(object.exclusion.margin_end_q26_6),
                        block_end: LayoutUnit::from_raw(object.exclusion.margin_bottom_q26_6),
                    },
                    minimum_fragment_width: LayoutUnit::from_raw(
                        object.exclusion.minimum_fragment_width_q26_6,
                    ),
                },
            })
        })
        .collect::<Result<Vec<_>, WireError>>()?;

    Ok(LayoutRequest {
        content: wire.content.into(),
        paragraphs,
        objects,
    })
}

fn encode_layout(_request: &LayoutRequest, layout: FlowLayout) -> WireLayout {
    WireLayout {
        content_height_q26_6: layout.content_height.raw(),
        paragraphs: layout
            .paragraphs
            .into_iter()
            .map(|paragraph| WireParagraphLayout {
                paragraph_id: paragraph.paragraph_id,
                top_q26_6: paragraph.top.raw(),
                bottom_q26_6: paragraph.bottom.raw(),
                lines: paragraph
                    .lines
                    .into_iter()
                    .map(|line| WireLine {
                        top_q26_6: line.top.raw(),
                        baseline_q26_6: line.baseline.raw(),
                        fragments: line
                            .fragments
                            .into_iter()
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
            .into_iter()
            .map(|object| WireResolvedObject {
                id: object.id,
                anchor: WireOutputAnchor {
                    paragraph_id: object.anchor.paragraph_id,
                    utf16_offset: object.anchor.utf16_offset,
                    affinity: match object.anchor.affinity {
                        AnchorAffinity::Upstream => WireAffinityOutput::Upstream,
                        AnchorAffinity::Downstream => WireAffinityOutput::Downstream,
                    },
                },
                anchor_reference_top_q26_6: object.anchor_reference_top.raw(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_boundary_preserves_string_ids_and_layouts_both_sides() {
        let input = br#"{
          "version": 1,
          "content": {"xQ26_6":0,"yQ26_6":0,"widthQ26_6":100,"heightQ26_6":1000},
          "paragraphs": [{
            "id":"block_01HZX-lastdraft",
            "utf16Length":4,
            "style":{"lineHeightQ26_6":10,"ascentQ26_6":8,"spaceAfterQ26_6":0},
            "clusters":[
              {"utf16Start":0,"utf16End":1,"advanceQ26_6":10,"canBreakAfter":true},
              {"utf16Start":1,"utf16End":2,"advanceQ26_6":10,"canBreakAfter":true},
              {"utf16Start":2,"utf16End":3,"advanceQ26_6":10,"canBreakAfter":true},
              {"utf16Start":3,"utf16End":4,"advanceQ26_6":10,"canBreakAfter":true}
            ]
          }],
          "objects": [{
            "id":"fl_550e8400-e29b-41d4-a716-446655440000",
            "anchor":{"paragraphId":"block_01HZX-lastdraft","utf16Offset":0,"affinity":"downstream"},
            "placement":{"inlineU16":32768,"blockOffset1024":0},
            "size":{"idealWidthQ26_6":40,"idealHeightQ26_6":10,"maxInlineU16":65535},
            "exclusion":{"shape":"bounds","marginStartQ26_6":0,"marginTopQ26_6":0,"marginEndQ26_6":0,"marginBottomQ26_6":0,"minimumFragmentWidthQ26_6":10},
            "responsivePolicy":"scale-then-centered-block-v1"
          }]
        }"#;
        let output: serde_json::Value = serde_json::from_slice(&layout_json(input)).unwrap();
        assert!(output.get("error").is_none(), "{output}");
        assert_eq!(
            output["layout"]["paragraphs"][0]["paragraphId"],
            "block_01HZX-lastdraft"
        );
        assert_eq!(
            output["layout"]["paragraphs"][0]["lines"][0]["fragments"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
