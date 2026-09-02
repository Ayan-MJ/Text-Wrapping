use core::fmt;

use crate::LayoutUnit;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpenTypeTag([u8; 4]);

impl OpenTypeTag {
    pub fn from_bytes(bytes: [u8; 4]) -> Result<Self, InvalidOpenTypeTag> {
        if bytes.iter().all(u8::is_ascii_graphic) {
            Ok(Self(bytes))
        } else {
            Err(InvalidOpenTypeTag)
        }
    }

    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidOpenTypeTag;

impl fmt::Display for InvalidOpenTypeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpenType tags must contain four printable ASCII bytes")
    }
}

impl std::error::Error for InvalidOpenTypeTag {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontFeature {
    pub tag: OpenTypeTag,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontVariation {
    pub tag: OpenTypeTag,
    /// OpenType variation coordinate encoded as signed 16.16 fixed point.
    pub value_16_16: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParagraphDirection {
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WritingMode {
    #[default]
    HorizontalTb,
    VerticalRl,
    VerticalLr,
}

impl WritingMode {
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::VerticalRl | Self::VerticalLr)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextOrientation {
    #[default]
    Mixed,
    Upright,
    Sideways,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GlyphOrientation {
    #[default]
    Upright,
    Sideways,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RichTextStyle {
    /// Content-addressed font identifier registered with CanonicalShaper.
    pub font_id: String,
    pub font_size: LayoutUnit,
    /// Deprecated ABI-v3 source-compatibility hint. ABI v4 ignores this value
    /// and derives shaping-run direction from paragraph UBA levels.
    pub direction: TextDirection,
    pub language: Option<String>,
    pub features: Vec<FontFeature>,
    pub variations: Vec<FontVariation>,
    pub fill_rgba: u32,
    pub underline: bool,
    pub strikethrough: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RichTextRun {
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub style: RichTextStyle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RichParagraph {
    pub id: String,
    pub text: String,
    pub base_direction: ParagraphDirection,
    pub writing_mode: WritingMode,
    pub text_orientation: TextOrientation,
    /// Runs are contiguous, non-overlapping, and cover the complete UTF-16 text.
    pub runs: Vec<RichTextRun>,
}

impl RichParagraph {
    pub fn utf16_len(&self) -> u32 {
        self.text.encode_utf16().count().min(u32::MAX as usize) as u32
    }
}
