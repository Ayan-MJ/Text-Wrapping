use serde::{Deserialize, Serialize};

use crate::{
    CanonicalShaper, ClusterCaretStop, FontDescriptor, FontFeature, FontVariation,
    GlyphOrientation, LayoutUnit, OpenTypeTag, ParagraphDirection, RichParagraph, RichTextRun,
    RichTextStyle, ShapedCluster, ShapedGlyph, ShapedParagraph, ShapedRun, ShapingError,
    TextDirection, TextOrientation, WritingMode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WireRichParagraph {
    id: String,
    text: String,
    #[serde(default)]
    base_direction: WireParagraphDirection,
    #[serde(default)]
    writing_mode: WireWritingMode,
    #[serde(default)]
    text_orientation: WireTextOrientation,
    runs: Vec<WireRichTextRun>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRichTextRun {
    utf16_start: u32,
    utf16_end: u32,
    style: WireRichTextStyle,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRichTextStyle {
    font_id: String,
    #[serde(rename = "fontSizeQ26_6")]
    font_size_q26_6: i32,
    #[serde(default)]
    direction: WireDirection,
    language: Option<String>,
    features: Vec<WireFeature>,
    variations: Vec<WireVariation>,
    #[serde(rename = "fillRGBA")]
    fill_rgba: u32,
    underline: bool,
    strikethrough: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireParagraphDirection {
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireWritingMode {
    #[default]
    HorizontalTb,
    VerticalRl,
    VerticalLr,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireTextOrientation {
    #[default]
    Mixed,
    Upright,
    Sideways,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WireGlyphOrientation {
    Upright,
    Sideways,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireFeature {
    tag: String,
    value: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireVariation {
    tag: String,
    #[serde(rename = "value16_16")]
    value_16_16: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFontEnvelope {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<WireFontDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WireRichError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireShapingEnvelope {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    paragraph: Option<WireShapedParagraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WireRichError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireRichError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFontDescriptor {
    id: String,
    sha256: String,
    face_index: u32,
    units_per_em: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireShapedParagraph {
    id: String,
    utf16_length: u32,
    requested_base_direction: WireParagraphDirection,
    base_direction: WireDirection,
    writing_mode: WireWritingMode,
    text_orientation: WireTextOrientation,
    runs: Vec<WireShapedRun>,
    clusters: Vec<WireShapedCluster>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireShapedRun {
    utf16_start: u32,
    utf16_end: u32,
    font: WireFontDescriptor,
    #[serde(rename = "fontSizeQ26_6")]
    font_size_q26_6: i32,
    bidi_level: u8,
    direction: WireDirection,
    orientation: WireGlyphOrientation,
    glyphs: Vec<WireShapedGlyph>,
    #[serde(rename = "advanceQ26_6")]
    advance_q26_6: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireShapedGlyph {
    glyph_id: u32,
    cluster_utf16: u32,
    #[serde(rename = "xAdvanceQ26_6")]
    x_advance_q26_6: i32,
    #[serde(rename = "yAdvanceQ26_6")]
    y_advance_q26_6: i32,
    #[serde(rename = "xOffsetQ26_6")]
    x_offset_q26_6: i32,
    #[serde(rename = "yOffsetQ26_6")]
    y_offset_q26_6: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireShapedCluster {
    utf16_start: u32,
    utf16_end: u32,
    #[serde(rename = "advanceQ26_6")]
    advance_q26_6: i32,
    can_break_after: bool,
    is_whitespace: bool,
    bidi_level: u8,
    direction: WireDirection,
    orientation: WireGlyphOrientation,
    caret_stops: Vec<WireCaretStop>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireCaretStop {
    utf16_offset: u32,
    #[serde(rename = "inlineOffsetQ26_6")]
    inline_offset_q26_6: i32,
}

pub(crate) fn font_registration_json(result: Result<FontDescriptor, ShapingError>) -> Vec<u8> {
    let envelope = match result {
        Ok(font) => WireFontEnvelope {
            version: 2,
            font: Some(font.into()),
            error: None,
        },
        Err(error) => WireFontEnvelope {
            version: 2,
            font: None,
            error: Some(shaping_error(error)),
        },
    };
    encode(&envelope)
}

pub(crate) fn shape_paragraph_json(shaper: &CanonicalShaper, input: &[u8]) -> Vec<u8> {
    let result = decode_paragraph(input)
        .and_then(|paragraph| shaper.shape_paragraph(&paragraph).map_err(shaping_error));
    let envelope = match result {
        Ok(paragraph) => WireShapingEnvelope {
            version: 2,
            paragraph: Some(paragraph.into()),
            error: None,
        },
        Err(error) => WireShapingEnvelope {
            version: 2,
            paragraph: None,
            error: Some(error),
        },
    };
    encode(&envelope)
}

fn decode_paragraph(input: &[u8]) -> Result<RichParagraph, WireRichError> {
    let wire: WireRichParagraph = serde_json::from_slice(input).map_err(|error| WireRichError {
        code: "invalid_json",
        message: error.to_string(),
    })?;
    decode_wire_paragraph(wire)
}

pub(crate) fn decode_wire_paragraph(
    wire: WireRichParagraph,
) -> Result<RichParagraph, WireRichError> {
    let runs = wire
        .runs
        .into_iter()
        .map(|run| {
            Ok(RichTextRun {
                utf16_start: run.utf16_start,
                utf16_end: run.utf16_end,
                style: RichTextStyle {
                    font_id: run.style.font_id,
                    font_size: LayoutUnit::from_raw(run.style.font_size_q26_6),
                    direction: run.style.direction.into(),
                    language: run.style.language,
                    features: run
                        .style
                        .features
                        .into_iter()
                        .map(|feature| {
                            Ok(FontFeature {
                                tag: parse_tag(&feature.tag)?,
                                value: feature.value,
                            })
                        })
                        .collect::<Result<Vec<_>, WireRichError>>()?,
                    variations: run
                        .style
                        .variations
                        .into_iter()
                        .map(|variation| {
                            Ok(FontVariation {
                                tag: parse_tag(&variation.tag)?,
                                value_16_16: variation.value_16_16,
                            })
                        })
                        .collect::<Result<Vec<_>, WireRichError>>()?,
                    fill_rgba: run.style.fill_rgba,
                    underline: run.style.underline,
                    strikethrough: run.style.strikethrough,
                },
            })
        })
        .collect::<Result<Vec<_>, WireRichError>>()?;
    Ok(RichParagraph {
        id: wire.id,
        text: wire.text,
        base_direction: wire.base_direction.into(),
        writing_mode: wire.writing_mode.into(),
        text_orientation: wire.text_orientation.into(),
        runs,
    })
}

fn parse_tag(value: &str) -> Result<OpenTypeTag, WireRichError> {
    let bytes: [u8; 4] = value.as_bytes().try_into().map_err(|_| WireRichError {
        code: "invalid_opentype_tag",
        message: format!("OpenType tag {value:?} must be exactly four ASCII bytes"),
    })?;
    OpenTypeTag::from_bytes(bytes).map_err(|error| WireRichError {
        code: "invalid_opentype_tag",
        message: error.to_string(),
    })
}

fn shaping_error(error: ShapingError) -> WireRichError {
    WireRichError {
        code: "shaping_error",
        message: error.to_string(),
    }
}

fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| {
        br#"{"version":2,"error":{"code":"serialization_error","message":"failed to serialize rich-text response"}}"#.to_vec()
    })
}

impl From<TextDirection> for WireDirection {
    fn from(value: TextDirection) -> Self {
        match value {
            TextDirection::LeftToRight => Self::LeftToRight,
            TextDirection::RightToLeft => Self::RightToLeft,
        }
    }
}

impl From<WireDirection> for TextDirection {
    fn from(value: WireDirection) -> Self {
        match value {
            WireDirection::LeftToRight => Self::LeftToRight,
            WireDirection::RightToLeft => Self::RightToLeft,
        }
    }
}

impl From<WireParagraphDirection> for ParagraphDirection {
    fn from(value: WireParagraphDirection) -> Self {
        match value {
            WireParagraphDirection::Auto => Self::Auto,
            WireParagraphDirection::LeftToRight => Self::LeftToRight,
            WireParagraphDirection::RightToLeft => Self::RightToLeft,
        }
    }
}

impl From<ParagraphDirection> for WireParagraphDirection {
    fn from(value: ParagraphDirection) -> Self {
        match value {
            ParagraphDirection::Auto => Self::Auto,
            ParagraphDirection::LeftToRight => Self::LeftToRight,
            ParagraphDirection::RightToLeft => Self::RightToLeft,
        }
    }
}

impl From<WireWritingMode> for WritingMode {
    fn from(value: WireWritingMode) -> Self {
        match value {
            WireWritingMode::HorizontalTb => Self::HorizontalTb,
            WireWritingMode::VerticalRl => Self::VerticalRl,
            WireWritingMode::VerticalLr => Self::VerticalLr,
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

impl From<WireTextOrientation> for TextOrientation {
    fn from(value: WireTextOrientation) -> Self {
        match value {
            WireTextOrientation::Mixed => Self::Mixed,
            WireTextOrientation::Upright => Self::Upright,
            WireTextOrientation::Sideways => Self::Sideways,
        }
    }
}

impl From<TextOrientation> for WireTextOrientation {
    fn from(value: TextOrientation) -> Self {
        match value {
            TextOrientation::Mixed => Self::Mixed,
            TextOrientation::Upright => Self::Upright,
            TextOrientation::Sideways => Self::Sideways,
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

impl From<FontDescriptor> for WireFontDescriptor {
    fn from(value: FontDescriptor) -> Self {
        Self {
            id: value.id,
            sha256: value.sha256,
            face_index: value.face_index,
            units_per_em: value.units_per_em,
        }
    }
}

impl From<ShapedParagraph> for WireShapedParagraph {
    fn from(value: ShapedParagraph) -> Self {
        Self {
            id: value.id,
            utf16_length: value.utf16_len,
            requested_base_direction: value.requested_base_direction.into(),
            base_direction: value.base_direction.into(),
            writing_mode: value.writing_mode.into(),
            text_orientation: value.text_orientation.into(),
            runs: value.runs.into_iter().map(Into::into).collect(),
            clusters: value.clusters.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ShapedRun> for WireShapedRun {
    fn from(value: ShapedRun) -> Self {
        Self {
            utf16_start: value.utf16_start,
            utf16_end: value.utf16_end,
            font: value.font.into(),
            font_size_q26_6: value.font_size.raw(),
            bidi_level: value.bidi_level,
            direction: value.direction.into(),
            orientation: value.orientation.into(),
            glyphs: value.glyphs.into_iter().map(Into::into).collect(),
            advance_q26_6: value.advance.raw(),
        }
    }
}

impl From<ShapedGlyph> for WireShapedGlyph {
    fn from(value: ShapedGlyph) -> Self {
        Self {
            glyph_id: value.glyph_id,
            cluster_utf16: value.cluster_utf16,
            x_advance_q26_6: value.x_advance.raw(),
            y_advance_q26_6: value.y_advance.raw(),
            x_offset_q26_6: value.x_offset.raw(),
            y_offset_q26_6: value.y_offset.raw(),
        }
    }
}

impl From<ShapedCluster> for WireShapedCluster {
    fn from(value: ShapedCluster) -> Self {
        Self {
            utf16_start: value.utf16_start,
            utf16_end: value.utf16_end,
            advance_q26_6: value.advance.raw(),
            can_break_after: value.can_break_after,
            is_whitespace: value.is_whitespace,
            bidi_level: value.bidi_level,
            direction: value.direction.into(),
            orientation: value.orientation.into(),
            caret_stops: value.caret_stops.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ClusterCaretStop> for WireCaretStop {
    fn from(value: ClusterCaretStop) -> Self {
        Self {
            utf16_offset: value.utf16_offset,
            inline_offset_q26_6: value.inline_offset.raw(),
        }
    }
}
