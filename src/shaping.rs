use core::fmt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rustybuzz::ttf_parser::Tag;
use rustybuzz::{BufferClusterLevel, Direction, Face, Feature, UnicodeBuffer, Variation};
use sha2::{Digest, Sha256};
use unicode_bidi::{BidiInfo, Level, LTR_LEVEL, RTL_LEVEL};
use unicode_linebreak::linebreaks;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
use unicode_vo::{char_orientation, Orientation};

use crate::fixed::scale_ratio;
use crate::{
    Cluster, FontFeature, FontVariation, GlyphOrientation, LayoutUnit, OpenTypeTag, Paragraph,
    ParagraphDirection, ParagraphStyle, RichParagraph, TextDirection, TextOrientation, WritingMode,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontDescriptor {
    pub id: String,
    pub sha256: String,
    pub face_index: u32,
    pub units_per_em: u16,
}

#[derive(Clone, Debug)]
struct RegisteredFont {
    descriptor: FontDescriptor,
    data: Arc<[u8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShapingItemKey {
    style_index: usize,
    bidi_level: u8,
    script: Script,
    orientation: GlyphOrientation,
    /// Contract §6 tate-chu-yoko: a short digit run set upright as ONE
    /// horizontal group inside a single em cell. Internal only — it changes how
    /// the item is shaped and positioned, never what the wire reports, so it
    /// costs no ABI change. The glyphs stay `Upright`, which is the truth: they
    /// are not rotated, only packed sideways within their cell.
    tate_chu_yoko: bool,
}

#[derive(Clone, Copy, Debug)]
struct ShapingItem {
    key: ShapingItemKey,
    utf16_start: u32,
    utf16_end: u32,
    byte_start: usize,
    byte_end: usize,
}

#[derive(Clone, Copy, Debug)]
struct ItemCharacter {
    byte_start: usize,
    byte_end: usize,
    utf16_start: u32,
    utf16_end: u32,
    style_index: usize,
    bidi_level: u8,
    script: Script,
    orientation: GlyphOrientation,
    tate_chu_yoko: bool,
}

/// Shaping is by far the most expensive part of laying a letter out, and a
/// keystroke changes exactly one paragraph. Every other paragraph in the letter
/// is shaped again from scratch for no reason, so the shaper remembers what it
/// has already shaped.
///
/// The key is the paragraph itself -- its text and every style that steers
/// shaping -- so a paragraph the author has not touched hits, and one they have
/// misses by construction. There is no invalidation to get wrong.
const SHAPE_CACHE_LIMIT: usize = 512;

#[derive(Default)]
pub struct CanonicalShaper {
    fonts: HashMap<String, RegisteredFont>,
    shaped: RefCell<HashMap<RichParagraph, ShapedParagraph>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterCaretStop {
    pub utf16_offset: u32,
    /// Physical distance from the cluster's inline-start edge.
    pub inline_offset: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub cluster_utf16: u32,
    pub x_advance: LayoutUnit,
    pub y_advance: LayoutUnit,
    pub x_offset: LayoutUnit,
    pub y_offset: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedRun {
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub font: FontDescriptor,
    pub font_size: LayoutUnit,
    pub bidi_level: u8,
    pub direction: TextDirection,
    pub orientation: GlyphOrientation,
    pub glyphs: Vec<ShapedGlyph>,
    pub advance: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedCluster {
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub advance: LayoutUnit,
    pub can_break_after: bool,
    pub is_whitespace: bool,
    pub bidi_level: u8,
    pub direction: TextDirection,
    pub orientation: GlyphOrientation,
    /// A tate-chu-yoko group: digits packed SIDE BY SIDE inside one em cell.
    /// Internal, and deliberately not on any wire — it changes how a caret is
    /// drawn inside the cell, which is geometry the engine already owns.
    pub tate_chu_yoko: bool,
    pub caret_stops: Vec<ClusterCaretStop>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedParagraph {
    pub id: String,
    pub utf16_len: u32,
    pub requested_base_direction: ParagraphDirection,
    pub base_direction: TextDirection,
    pub writing_mode: WritingMode,
    pub text_orientation: TextOrientation,
    pub runs: Vec<ShapedRun>,
    /// Logical-order clusters consumed by the existing exclusion flow algorithm.
    pub clusters: Vec<ShapedCluster>,
}

impl ShapedParagraph {
    pub fn to_flow_paragraph(&self, style: ParagraphStyle) -> Paragraph {
        Paragraph {
            id: self.id.clone(),
            utf16_len: self.utf16_len,
            style,
            requested_base_direction: self.requested_base_direction,
            base_direction: self.base_direction,
            writing_mode: self.writing_mode,
            text_orientation: self.text_orientation,
            clusters: self
                .clusters
                .iter()
                .map(|cluster| Cluster {
                    utf16_start: cluster.utf16_start,
                    utf16_end: cluster.utf16_end,
                    advance: cluster.advance,
                    can_break_after: cluster.can_break_after,
                    is_whitespace: cluster.is_whitespace,
                    bidi_level: cluster.bidi_level,
                    direction: cluster.direction,
                    orientation: cluster.orientation,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShapingError {
    EmptyFontId,
    InvalidFont { id: String, face_index: u32 },
    ConflictingFontId(String),
    EmptyParagraphId,
    InvalidRunRange { run_index: usize },
    InvalidFontSize { run_index: usize },
    MissingFont(String),
    InvalidLanguage(String),
}

impl fmt::Display for ShapingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFontId => write!(f, "font ID must not be empty"),
            Self::InvalidFont { id, face_index } => {
                write!(f, "font {id} has no face at index {face_index}")
            }
            Self::ConflictingFontId(id) => {
                write!(f, "font ID {id} is already registered with different bytes")
            }
            Self::EmptyParagraphId => write!(f, "paragraph ID must not be empty"),
            Self::InvalidRunRange { run_index } => {
                write!(
                    f,
                    "rich-text run {run_index} is not a valid contiguous UTF-16 range"
                )
            }
            Self::InvalidFontSize { run_index } => {
                write!(f, "rich-text run {run_index} has a non-positive font size")
            }
            Self::MissingFont(id) => write!(f, "font {id} is not registered"),
            Self::InvalidLanguage(language) => write!(f, "invalid shaping language {language}"),
        }
    }
}

impl std::error::Error for ShapingError {}

impl CanonicalShaper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers immutable font bytes. The returned SHA-256 identity must be
    /// persisted or transmitted with the document's font manifest.
    pub fn register_font(
        &mut self,
        id: impl Into<String>,
        data: impl Into<Arc<[u8]>>,
        face_index: u32,
    ) -> Result<FontDescriptor, ShapingError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ShapingError::EmptyFontId);
        }
        let data = data.into();
        let Some(face) = Face::from_slice(&data, face_index) else {
            return Err(ShapingError::InvalidFont { id, face_index });
        };
        let descriptor = FontDescriptor {
            id: id.clone(),
            sha256: sha256_hex(&data),
            face_index,
            units_per_em: face.units_per_em().clamp(1, u16::MAX as i32) as u16,
        };
        if let Some(existing) = self.fonts.get(&id) {
            if existing.descriptor == descriptor {
                return Ok(descriptor);
            }
            return Err(ShapingError::ConflictingFontId(id));
        }
        self.fonts.insert(
            id,
            RegisteredFont {
                descriptor: descriptor.clone(),
                data,
            },
        );
        Ok(descriptor)
    }

    pub fn font_descriptor(&self, id: &str) -> Option<&FontDescriptor> {
        self.fonts.get(id).map(|font| &font.descriptor)
    }

    /// Shape a paragraph, reusing the last result for one that has not changed.
    ///
    /// A keystroke edits one paragraph; without this every other paragraph in
    /// the letter is shaped again for nothing, which is the single largest cost
    /// in laying out a long letter. Cloning a shaped paragraph is a memcpy of
    /// its clusters and glyphs -- far cheaper than shaping it again.
    pub fn shape_paragraph(
        &self,
        paragraph: &RichParagraph,
    ) -> Result<ShapedParagraph, ShapingError> {
        if let Some(hit) = self.shaped.borrow().get(paragraph) {
            return Ok(hit.clone());
        }
        let shaped = self.shape_paragraph_uncached(paragraph)?;
        let mut cache = self.shaped.borrow_mut();
        // A font's bytes can never change under an id -- register_font refuses a
        // conflicting one -- so an entry can never go stale. The only bound
        // needed is on size.
        if cache.len() >= SHAPE_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(paragraph.clone(), shaped.clone());
        Ok(shaped)
    }

    /// How many paragraphs the shaper is currently remembering.
    pub fn shape_cache_len(&self) -> usize {
        self.shaped.borrow().len()
    }

    fn shape_paragraph_uncached(
        &self,
        paragraph: &RichParagraph,
    ) -> Result<ShapedParagraph, ShapingError> {
        validate_paragraph(paragraph)?;
        let utf16_len = paragraph.utf16_len();
        let requested_level = match paragraph.base_direction {
            ParagraphDirection::Auto => None,
            ParagraphDirection::LeftToRight => Some(LTR_LEVEL),
            ParagraphDirection::RightToLeft => Some(RTL_LEVEL),
        };
        let bidi = BidiInfo::new(&paragraph.text, requested_level);
        let resolved_level = bidi
            .paragraphs
            .first()
            .map(|paragraph| paragraph.level)
            .unwrap_or_else(|| requested_level.unwrap_or(LTR_LEVEL));
        let base_direction = text_direction(resolved_level);
        if utf16_len == 0 {
            return Ok(ShapedParagraph {
                id: paragraph.id.clone(),
                utf16_len,
                requested_base_direction: paragraph.base_direction,
                base_direction,
                writing_mode: paragraph.writing_mode,
                text_orientation: paragraph.text_orientation,
                runs: Vec::new(),
                clusters: Vec::new(),
            });
        }

        let break_offsets: HashSet<u32> = linebreaks(&paragraph.text)
            .filter_map(|(byte_offset, _)| byte_to_utf16(&paragraph.text, byte_offset))
            .collect();
        let items = shaping_items(paragraph, &bidi);
        let mut shaped_runs = Vec::with_capacity(items.len());
        let mut shaped_clusters = Vec::new();

        for item in items {
            let run = &paragraph.runs[item.key.style_index];
            let font = self
                .fonts
                .get(&run.style.font_id)
                .ok_or_else(|| ShapingError::MissingFont(run.style.font_id.clone()))?;
            let run_text = &paragraph.text[item.byte_start..item.byte_end];
            let mut face = Face::from_slice(&font.data, font.descriptor.face_index)
                .expect("registered font remains valid");
            let variations: Vec<Variation> = run
                .style
                .variations
                .iter()
                .map(to_rustybuzz_variation)
                .collect();
            face.set_variations(&variations);

            let mut buffer = UnicodeBuffer::new();
            let mut utf16_offset = item.utf16_start;
            for character in run_text.chars() {
                buffer.add(character, utf16_offset);
                utf16_offset += character.len_utf16() as u32;
            }
            buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);
            let shaping_direction = if paragraph.writing_mode.is_vertical()
                && item.key.orientation == GlyphOrientation::Upright
                && !item.key.tate_chu_yoko
            {
                Direction::TopToBottom
            } else {
                match text_direction_from_level(item.key.bidi_level) {
                    TextDirection::LeftToRight => Direction::LeftToRight,
                    TextDirection::RightToLeft => Direction::RightToLeft,
                }
            };
            buffer.set_direction(shaping_direction);
            if let Some(language) = &run.style.language {
                let language = language
                    .parse()
                    .map_err(|_| ShapingError::InvalidLanguage(language.clone()))?;
                buffer.set_language(language);
            }
            buffer.guess_segment_properties();

            let mut features: Vec<Feature> = run
                .style
                .features
                .iter()
                .map(to_rustybuzz_feature)
                .collect();
            // A tate-chu-yoko group is horizontal text that happens to sit in a
            // vertical column, so it must NOT take the vertical glyph variants.
            if paragraph.writing_mode.is_vertical()
                && item.key.orientation == GlyphOrientation::Upright
                && !item.key.tate_chu_yoko
            {
                features.push(Feature::new(Tag::from_bytes(b"vert"), 1, ..));
                features.push(Feature::new(Tag::from_bytes(b"vrt2"), 1, ..));
            }
            let glyph_buffer = rustybuzz::shape(&face, &features, buffer);
            let infos = glyph_buffer.glyph_infos();
            let positions = glyph_buffer.glyph_positions();
            let mut glyphs: Vec<ShapedGlyph> = infos
                .iter()
                .zip(positions)
                .map(|(info, position)| ShapedGlyph {
                    glyph_id: info.glyph_id,
                    cluster_utf16: info.cluster,
                    x_advance: scale_font_unit(
                        position.x_advance,
                        run.style.font_size,
                        font.descriptor.units_per_em,
                    ),
                    y_advance: scale_font_unit(
                        position.y_advance,
                        run.style.font_size,
                        font.descriptor.units_per_em,
                    ),
                    x_offset: scale_font_unit(
                        position.x_offset,
                        run.style.font_size,
                        font.descriptor.units_per_em,
                    ),
                    y_offset: scale_font_unit(
                        position.y_offset,
                        run.style.font_size,
                        font.descriptor.units_per_em,
                    ),
                })
                .collect();
            if item.key.tate_chu_yoko {
                // A vertical upright glyph carries a y_offset from the font that
                // drops its baseline into the em cell -- the same value for every
                // glyph, because it is the face's vertical origin. A packed group
                // has to sit on that same line or it renders a whole cell high,
                // overlapping the character before it. Ask the font rather than
                // assume a number.
                let upright_y_offset =
                    upright_vertical_y_offset(&face, run_text, run.style.font_size, &font.descriptor);
                pack_tate_chu_yoko(
                    &mut glyphs,
                    run.style.font_size,
                    item.utf16_start,
                    upright_y_offset,
                );
            }
            let uses_vertical_metrics = paragraph.writing_mode.is_vertical()
                && item.key.orientation == GlyphOrientation::Upright;
            let run_advance = glyphs.iter().fold(LayoutUnit::ZERO, |sum, glyph| {
                sum + glyph_inline_advance(glyph, uses_vertical_metrics)
            });

            let mut cluster_starts: Vec<u32> = if item.key.tate_chu_yoko {
                // One cell, so one cluster: line breaking must never split it,
                // and the caret stops spread across the digits inside it.
                vec![item.utf16_start]
            } else {
                infos.iter().map(|info| info.cluster).collect()
            };
            cluster_starts.sort_unstable();
            cluster_starts.dedup();
            for (index, start) in cluster_starts.iter().copied().enumerate() {
                let end = cluster_starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(item.utf16_end);
                let advance = if item.key.tate_chu_yoko {
                    run.style.font_size
                } else {
                    infos
                    .iter()
                    .zip(positions)
                    .filter(|(info, _)| info.cluster == start)
                    .fold(LayoutUnit::ZERO, |sum, (_, position)| {
                        let font_advance = if uses_vertical_metrics {
                            position.y_advance
                        } else {
                            position.x_advance
                        };
                        let advance = scale_font_unit(
                            font_advance,
                            run.style.font_size,
                            font.descriptor.units_per_em,
                        );
                        sum + LayoutUnit::from_raw(advance.raw().saturating_abs())
                    })
                };
                let cluster_start_byte =
                    utf16_to_byte(&paragraph.text, start).expect("shaper returned valid cluster");
                let cluster_end_byte =
                    utf16_to_byte(&paragraph.text, end).expect("shaper returned valid cluster");
                let cluster_text = &paragraph.text[cluster_start_byte..cluster_end_byte];
                shaped_clusters.push(ShapedCluster {
                    utf16_start: start,
                    utf16_end: end,
                    advance,
                    can_break_after: break_offsets.contains(&end),
                    is_whitespace: cluster_text.chars().all(char::is_whitespace),
                    bidi_level: item.key.bidi_level,
                    direction: text_direction_from_level(item.key.bidi_level),
                    orientation: item.key.orientation,
                    tate_chu_yoko: item.key.tate_chu_yoko,
                    caret_stops: caret_stops(
                        cluster_text,
                        start,
                        advance,
                        item.key.bidi_level % 2 == 1,
                    ),
                });
            }

            shaped_runs.push(ShapedRun {
                utf16_start: item.utf16_start,
                utf16_end: item.utf16_end,
                font: font.descriptor.clone(),
                font_size: run.style.font_size,
                bidi_level: item.key.bidi_level,
                direction: text_direction_from_level(item.key.bidi_level),
                orientation: item.key.orientation,
                glyphs,
                advance: run_advance,
            });
        }
        shaped_clusters.sort_by_key(|cluster| cluster.utf16_start);

        Ok(ShapedParagraph {
            id: paragraph.id.clone(),
            utf16_len,
            requested_base_direction: paragraph.base_direction,
            base_direction,
            writing_mode: paragraph.writing_mode,
            text_orientation: paragraph.text_orientation,
            runs: shaped_runs,
            clusters: shaped_clusters,
        })
    }
}

fn shaping_items(paragraph: &RichParagraph, bidi: &BidiInfo<'_>) -> Vec<ShapingItem> {
    let mut characters = Vec::with_capacity(paragraph.text.chars().count());
    let mut utf16_offset = 0_u32;
    let mut style_index = 0_usize;
    for (byte_start, character) in paragraph.text.char_indices() {
        while style_index + 1 < paragraph.runs.len()
            && utf16_offset >= paragraph.runs[style_index].utf16_end
        {
            style_index += 1;
        }
        let byte_end = byte_start + character.len_utf8();
        let utf16_end = utf16_offset + character.len_utf16() as u32;
        characters.push(ItemCharacter {
            byte_start,
            byte_end,
            utf16_start: utf16_offset,
            utf16_end,
            style_index,
            bidi_level: bidi.levels[byte_start].number(),
            script: character.script(),
            orientation: glyph_orientation(
                character,
                paragraph.writing_mode,
                paragraph.text_orientation,
            ),
            tate_chu_yoko: false,
        });
        utf16_offset = utf16_end;
    }

    mark_tate_chu_yoko(&mut characters, paragraph);

    // Common and inherited characters adopt their nearest strong script so
    // punctuation and combining marks remain in the shaping item that owns
    // their context. Bidi levels still take precedence as item boundaries.
    let mut previous_script = None;
    for character in &mut characters {
        if is_neutral_script(character.script) {
            if let Some(script) = previous_script {
                character.script = script;
            }
        } else {
            previous_script = Some(character.script);
        }
    }
    let mut next_script = None;
    for character in characters.iter_mut().rev() {
        if is_neutral_script(character.script) {
            if let Some(script) = next_script {
                character.script = script;
            }
        } else {
            next_script = Some(character.script);
        }
    }

    let mut items: Vec<ShapingItem> = Vec::new();
    for character in characters {
        let key = ShapingItemKey {
            style_index: character.style_index,
            bidi_level: character.bidi_level,
            script: character.script,
            orientation: character.orientation,
            tate_chu_yoko: character.tate_chu_yoko,
        };
        if let Some(current) = items.last_mut() {
            if current.key == key {
                current.byte_end = character.byte_end;
                current.utf16_end = character.utf16_end;
                continue;
            }
        }
        items.push(ShapingItem {
            key,
            utf16_start: character.utf16_start,
            utf16_end: character.utf16_end,
            byte_start: character.byte_start,
            byte_end: character.byte_end,
        });
    }
    items
}

/// Mark short ASCII digit runs for tate-chu-yoko (contract §6).
///
/// One or two digits are set upright as a single horizontal group. Three or
/// more stay an ordinary vertical sequence, so a year like `2026` reads down
/// the column while a time like `12` sits upright in one cell. The run must be
/// MAXIMAL: it is the length of the whole digit run that decides, not the
/// length of some window inside it.
fn mark_tate_chu_yoko(characters: &mut [ItemCharacter], paragraph: &RichParagraph) {
    if !paragraph.writing_mode.is_vertical()
        || paragraph.text_orientation != TextOrientation::Mixed
    {
        return;
    }
    let is_digit = |character: &ItemCharacter| {
        paragraph.text[character.byte_start..character.byte_end]
            .chars()
            .all(|value| value.is_ascii_digit())
            && character.byte_end - character.byte_start == 1
    };

    let mut index = 0;
    while index < characters.len() {
        if !is_digit(&characters[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < characters.len() && is_digit(&characters[index]) {
            index += 1;
        }
        let run = &mut characters[start..index];
        if run.len() <= 2 {
            for character in run.iter_mut() {
                // Upright is what the wire reports, and it is accurate: the
                // digits are not rotated. Only their packing is special.
                character.orientation = GlyphOrientation::Upright;
                character.tate_chu_yoko = true;
            }
        }
    }
}

fn is_neutral_script(script: Script) -> bool {
    matches!(script, Script::Common | Script::Inherited | Script::Unknown)
}

fn glyph_orientation(
    character: char,
    writing_mode: WritingMode,
    text_orientation: TextOrientation,
) -> GlyphOrientation {
    if !writing_mode.is_vertical() {
        return GlyphOrientation::Upright;
    }
    match text_orientation {
        TextOrientation::Upright => GlyphOrientation::Upright,
        TextOrientation::Sideways => GlyphOrientation::Sideways,
        TextOrientation::Mixed => match char_orientation(character) {
            Orientation::Upright | Orientation::TransformedOrUpright => GlyphOrientation::Upright,
            Orientation::Rotated | Orientation::TransformedOrRotated => GlyphOrientation::Sideways,
        },
    }
}

fn text_direction(level: Level) -> TextDirection {
    if level.is_rtl() {
        TextDirection::RightToLeft
    } else {
        TextDirection::LeftToRight
    }
}

fn text_direction_from_level(level: u8) -> TextDirection {
    if level % 2 == 1 {
        TextDirection::RightToLeft
    } else {
        TextDirection::LeftToRight
    }
}

fn validate_paragraph(paragraph: &RichParagraph) -> Result<(), ShapingError> {
    if paragraph.id.is_empty() {
        return Err(ShapingError::EmptyParagraphId);
    }
    let length = paragraph.utf16_len();
    if length == 0 {
        if paragraph.runs.is_empty() {
            return Ok(());
        }
        return Err(ShapingError::InvalidRunRange { run_index: 0 });
    }
    let mut expected_start = 0;
    for (index, run) in paragraph.runs.iter().enumerate() {
        if run.utf16_start != expected_start
            || run.utf16_end <= run.utf16_start
            || run.utf16_end > length
            || utf16_to_byte(&paragraph.text, run.utf16_start).is_none()
            || utf16_to_byte(&paragraph.text, run.utf16_end).is_none()
        {
            return Err(ShapingError::InvalidRunRange { run_index: index });
        }
        if run.style.font_size <= LayoutUnit::ZERO {
            return Err(ShapingError::InvalidFontSize { run_index: index });
        }
        expected_start = run.utf16_end;
    }
    if expected_start != length {
        return Err(ShapingError::InvalidRunRange {
            run_index: paragraph.runs.len(),
        });
    }
    Ok(())
}

fn to_rustybuzz_feature(feature: &FontFeature) -> Feature {
    Feature::new(tag(feature.tag), feature.value, ..)
}

fn to_rustybuzz_variation(variation: &FontVariation) -> Variation {
    Variation {
        tag: tag(variation.tag),
        value: variation.value_16_16 as f32 / 65_536.0,
    }
}

fn tag(value: OpenTypeTag) -> Tag {
    Tag::from_bytes(&value.bytes())
}

fn scale_font_unit(value: i32, font_size: LayoutUnit, units_per_em: u16) -> LayoutUnit {
    let denominator = i64::from(units_per_em.max(1));
    let product = i64::from(value) * i64::from(font_size.raw());
    let rounded = if product >= 0 {
        product + denominator / 2
    } else {
        product - denominator / 2
    };
    LayoutUnit::from_raw((rounded / denominator).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// Lay a tate-chu-yoko group out inside one em cell.
///
/// The digits keep their own glyphs and stay upright; only their placement is
/// synthetic. They are positioned across the column by `x_offset`, and the
/// group consumes exactly one em down the column, which it does by giving every
/// glyph but the last a zero vertical advance. Nothing here needs a per-glyph
/// scale, so nothing here needs a wire change.
///
/// A CJK face sets ASCII digits at about 0.52 em, so two of them are slightly
/// wider than the cell. They are then pinned to the cell's two edges and their
/// advances tighten by that few percent — which is what tate-chu-yoko looks
/// like anyway. Glyph ink is narrower than its advance, so they do not collide.
/// The y_offset this face gives an upright glyph in vertical writing.
///
/// It is a property of the face, not of the character, so shaping the group's
/// own text vertically is enough to learn it, and it stays correct for a font
/// whose vertical origin differs from the usual 0.88 em.
fn upright_vertical_y_offset(
    face: &Face<'_>,
    text: &str,
    font_size: LayoutUnit,
    descriptor: &FontDescriptor,
) -> LayoutUnit {
    let mut buffer = UnicodeBuffer::new();
    for (index, character) in text.chars().enumerate() {
        buffer.add(character, index as u32);
    }
    buffer.set_direction(Direction::TopToBottom);
    buffer.guess_segment_properties();
    let features = [
        Feature::new(Tag::from_bytes(b"vert"), 1, ..),
        Feature::new(Tag::from_bytes(b"vrt2"), 1, ..),
    ];
    let shaped = rustybuzz::shape(face, &features, buffer);
    let offset = shaped
        .glyph_positions()
        .first()
        .map(|position| position.y_offset)
        .unwrap_or(0);
    scale_font_unit(offset, font_size, descriptor.units_per_em)
}

fn pack_tate_chu_yoko(
    glyphs: &mut [ShapedGlyph],
    em: LayoutUnit,
    group_start: u32,
    y_offset: LayoutUnit,
) {
    let count = glyphs.len();
    if count == 0 {
        return;
    }
    let widths: Vec<LayoutUnit> = glyphs
        .iter()
        .map(|glyph| LayoutUnit::from_raw(glyph.x_advance.raw().saturating_abs()))
        .collect();
    let total = widths
        .iter()
        .fold(LayoutUnit::ZERO, |sum, width| sum + *width);

    let mut positions: Vec<LayoutUnit> = Vec::with_capacity(count);
    if count == 1 || total <= em {
        // Narrower than the cell: centre the group in it.
        let mut pen = LayoutUnit::from_raw((em.raw() - total.raw()) / 2);
        for width in &widths {
            positions.push(pen);
            pen += *width;
        }
    } else {
        // Wider than the cell: pin first and last to its edges.
        let span = (em.raw() - widths[count - 1].raw()) as i64;
        for index in 0..count {
            positions.push(LayoutUnit::from_raw(
                (index as i64 * span / (count as i64 - 1)) as i32,
            ));
        }
    }

    // An upright vertical glyph carries an x_offset of -em/2 from the font, so
    // that it CENTRES on the baseline rather than starting at it. The cell
    // therefore spans [-em/2, +em/2] about the baseline, and a group packed
    // from zero would sit half an em to the right of its own column.
    let cell_origin = LayoutUnit::from_raw(-em.raw() / 2);
    for (index, glyph) in glyphs.iter_mut().enumerate() {
        // Every glyph answers to the group's single cluster.
        glyph.cluster_utf16 = group_start;
        glyph.x_offset = cell_origin + positions[index];
        glyph.y_offset = y_offset;
        glyph.x_advance = LayoutUnit::ZERO;
        glyph.y_advance = if index + 1 == count {
            LayoutUnit::from_raw(-em.raw())
        } else {
            LayoutUnit::ZERO
        };
    }
}

fn glyph_inline_advance(glyph: &ShapedGlyph, uses_vertical_metrics: bool) -> LayoutUnit {
    let advance = if uses_vertical_metrics {
        glyph.y_advance
    } else {
        glyph.x_advance
    };
    LayoutUnit::from_raw(advance.raw().saturating_abs())
}

fn caret_stops(
    text: &str,
    utf16_start: u32,
    advance: LayoutUnit,
    reverse_inline: bool,
) -> Vec<ClusterCaretStop> {
    let graphemes: Vec<&str> = text.graphemes(true).collect();
    let count = graphemes.len().max(1);
    let mut stops = Vec::with_capacity(count + 1);
    let mut offset = utf16_start;
    for index in 0..=count {
        let logical_inline = scale_ratio(advance, index as i32, count as i32);
        let inline_offset = if reverse_inline {
            advance - logical_inline
        } else {
            logical_inline
        };
        stops.push(ClusterCaretStop {
            utf16_offset: offset,
            inline_offset,
        });
        if let Some(grapheme) = graphemes.get(index) {
            offset += grapheme.encode_utf16().count() as u32;
        }
    }
    stops
}

fn utf16_to_byte(text: &str, target: u32) -> Option<usize> {
    let mut utf16 = 0_u32;
    for (byte, character) in text.char_indices() {
        if utf16 == target {
            return Some(byte);
        }
        utf16 = utf16.checked_add(character.len_utf16() as u32)?;
        if utf16 > target {
            return None;
        }
    }
    (utf16 == target).then_some(text.len())
}

fn byte_to_utf16(text: &str, target: usize) -> Option<u32> {
    if target > text.len() || !text.is_char_boundary(target) {
        return None;
    }
    Some(text[..target].encode_utf16().count().min(u32::MAX as usize) as u32)
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use core::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RichTextRun, RichTextStyle, TextDirection};

    fn style() -> RichTextStyle {
        RichTextStyle {
            font_id: "font".into(),
            font_size: LayoutUnit::from_raw(16 * 64),
            direction: TextDirection::LeftToRight,
            language: Some("en".into()),
            features: Vec::new(),
            variations: Vec::new(),
            fill_rgba: 0x000000ff,
            underline: false,
            strikethrough: false,
        }
    }

    #[test]
    fn rejects_a_run_boundary_inside_a_surrogate_pair() {
        let paragraph = RichParagraph {
            id: "p".into(),
            text: "a😀b".into(),
            base_direction: ParagraphDirection::Auto,
            writing_mode: WritingMode::HorizontalTb,
            text_orientation: TextOrientation::Mixed,
            runs: vec![
                RichTextRun {
                    utf16_start: 0,
                    utf16_end: 2,
                    style: style(),
                },
                RichTextRun {
                    utf16_start: 2,
                    utf16_end: 4,
                    style: style(),
                },
            ],
        };
        assert_eq!(
            validate_paragraph(&paragraph),
            Err(ShapingError::InvalidRunRange { run_index: 0 })
        );
    }

    #[test]
    fn grapheme_caret_stops_are_deterministic() {
        let stops = caret_stops("a\u{0301}b", 4, LayoutUnit::from_raw(100), false);
        assert_eq!(
            stops,
            vec![
                ClusterCaretStop {
                    utf16_offset: 4,
                    inline_offset: LayoutUnit::ZERO,
                },
                ClusterCaretStop {
                    utf16_offset: 6,
                    inline_offset: LayoutUnit::from_raw(50),
                },
                ClusterCaretStop {
                    utf16_offset: 7,
                    inline_offset: LayoutUnit::from_raw(100),
                },
            ]
        );
    }

    // ---- Vertical Japanese: the Slice 4 foundation ------------------------
    // Slice 4 rests entirely on the shaper driving the vertical OpenType
    // machinery, and on the shipped font artifact carrying the tables it needs.
    // Google's webfont builds have been known to strip vertical features from a
    // CJK subset, so this is asserted against the exact committed bytes rather
    // than assumed from the upstream font's documentation.

    const JP_FONT: &[u8] = include_bytes!("../fixtures/fonts/NotoSansJP-Variable.ttf");

    fn sfnt_tables(bytes: &[u8]) -> Vec<String> {
        let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        (0..count)
            .map(|index| {
                let at = 12 + index * 16;
                String::from_utf8_lossy(&bytes[at..at + 4]).to_string()
            })
            .collect()
    }

    fn shape_upright(text: &str, vertical: bool) -> Vec<(u32, i32, i32)> {
        let face = Face::from_slice(JP_FONT, 0).expect("jp face");
        let mut buffer = UnicodeBuffer::new();
        for (index, character) in text.chars().enumerate() {
            buffer.add(character, index as u32);
        }
        buffer.set_direction(if vertical {
            Direction::TopToBottom
        } else {
            Direction::LeftToRight
        });
        buffer.guess_segment_properties();
        let features: Vec<Feature> = if vertical {
            vec![
                Feature::new(Tag::from_bytes(b"vert"), 1, ..),
                Feature::new(Tag::from_bytes(b"vrt2"), 1, ..),
            ]
        } else {
            Vec::new()
        };
        let shaped = rustybuzz::shape(&face, &features, buffer);
        shaped
            .glyph_infos()
            .iter()
            .zip(shaped.glyph_positions())
            .map(|(info, position)| (info.glyph_id, position.x_advance, position.y_advance))
            .collect()
    }

    #[test]
    fn shipped_japanese_font_carries_the_vertical_tables() {
        let tables = sfnt_tables(JP_FONT);
        assert!(tables.iter().any(|t| t == "vmtx"), "no vmtx: {tables:?}");
        assert!(tables.iter().any(|t| t == "vhea"), "no vhea: {tables:?}");
        assert!(tables.iter().any(|t| t == "GSUB"), "no GSUB: {tables:?}");
        // VORG is a CFF-outline table. This artifact has glyf outlines, so the
        // vertical origin is derived from vhea/vmtx instead. Its absence is
        // expected and is NOT a gap.
        assert!(!tables.iter().any(|t| t == "VORG"));
    }

    #[test]
    fn vertical_substitution_fires_for_japanese_punctuation() {
        // Each of these MUST take a different glyph in vertical writing, or the
        // marks sit in the wrong corner of their cell and the letter is visibly
        // wrong to a Japanese reader.
        for text in ["\u{3002}", "\u{3001}", "\u{300c}", "\u{300d}", "\u{30fc}"] {
            let horizontal = shape_upright(text, false);
            let vertical = shape_upright(text, true);
            assert_eq!(horizontal.len(), 1);
            assert_eq!(vertical.len(), 1);
            assert_ne!(
                horizontal[0].0, vertical[0].0,
                "vert/vrt2 did not substitute {text:?}"
            );
        }
    }

    #[test]
    fn vertical_advances_come_from_the_font_not_a_fallback() {
        // Full-width kana: one em down the column, taken from vmtx. Negative
        // because the column advances downward.
        let vertical = shape_upright("\u{3042}", true);
        assert_eq!(vertical[0].2, -1000);
        assert_eq!(vertical[0].1, 0);

        // The same glyph horizontally uses hmtx and moves along x instead.
        let horizontal = shape_upright("\u{3042}", false);
        assert_eq!(horizontal[0].1, 1000);
        assert_eq!(horizontal[0].2, 0);
    }

    #[test]
    fn tate_chu_yoko_is_engine_work_no_font_or_shaper_supplies_it() {
        // Why this has to be built rather than asked for: forced upright, "12"
        // takes two FULL-em cells from the font, one digit stacked above the
        // other, and per-character orientation resolution would lay each digit
        // sideways. Neither is what contract §6 asks for.
        let vertical = shape_upright("12", true);
        assert_eq!(vertical.len(), 2);
        assert_eq!(vertical[0].2, -1000);
        assert_eq!(vertical[1].2, -1000);
        assert_eq!(
            glyph_orientation('1', WritingMode::VerticalRl, TextOrientation::Mixed),
            GlyphOrientation::Sideways
        );
        assert_eq!(
            glyph_orientation('\u{3042}', WritingMode::VerticalRl, TextOrientation::Mixed),
            GlyphOrientation::Upright
        );
    }

    fn vertical_japanese(text: &str) -> ShapedParagraph {
        let mut shaper = CanonicalShaper::new();
        shaper.register_font("jp", JP_FONT, 0).unwrap();
        let mut style = style();
        style.font_id = "jp".into();
        style.language = Some("ja".into());
        shaper
            .shape_paragraph(&RichParagraph {
                id: "tcy".into(),
                text: text.into(),
                base_direction: ParagraphDirection::LeftToRight,
                writing_mode: WritingMode::VerticalRl,
                text_orientation: TextOrientation::Mixed,
                runs: vec![RichTextRun {
                    utf16_start: 0,
                    utf16_end: text.encode_utf16().count() as u32,
                    style,
                }],
            })
            .unwrap()
    }

    fn cluster_at(shaped: &ShapedParagraph, start: u32) -> &ShapedCluster {
        shaped
            .clusters
            .iter()
            .find(|cluster| cluster.utf16_start == start)
            .expect("cluster")
    }

    #[test]
    fn a_short_digit_run_becomes_one_upright_cell_of_exactly_one_em() {
        // "12時" — the two digits are one cell, the kanji is the next.
        let shaped = vertical_japanese("12\u{6642}");
        let group = cluster_at(&shaped, 0);
        assert_eq!(group.utf16_end, 2, "both digits belong to one cluster");
        assert_eq!(group.orientation, GlyphOrientation::Upright);
        // One em, the same cell a full-width kanji occupies.
        assert_eq!(group.advance, cluster_at(&shaped, 2).advance);

        // A caret can still be placed either side of the group and between the
        // digits, so editing granularity is not lost.
        assert_eq!(group.caret_stops.len(), 3);
    }

    #[test]
    fn a_single_digit_is_centred_in_its_cell() {
        let shaped = vertical_japanese("8\u{5E74}");
        let group = cluster_at(&shaped, 0);
        assert_eq!(group.utf16_end, 1);
        assert_eq!(group.advance, cluster_at(&shaped, 1).advance);

        let run = &shaped.runs[0];
        assert_eq!(run.glyphs.len(), 1);
        // Centred in a cell that straddles the baseline, so a single narrow
        // digit starts left of it, like a full-width glyph does.
        assert!(run.glyphs[0].x_offset < LayoutUnit::ZERO);
    }

    #[test]
    fn three_or_more_digits_stay_an_ordinary_vertical_sequence() {
        // Contract §6 draws the line at two: a year reads down the column.
        let shaped = vertical_japanese("2026\u{5E74}");
        for start in 0..4 {
            let digit = cluster_at(&shaped, start);
            assert_eq!(digit.utf16_end, start + 1, "digit {start} stayed its own cluster");
            assert_eq!(digit.orientation, GlyphOrientation::Sideways);
        }
    }

    #[test]
    fn the_digits_of_a_group_sit_side_by_side_within_one_em() {
        let shaped = vertical_japanese("12\u{6642}");
        let em = shaped.runs[0].font_size;
        let group: Vec<_> = shaped
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.cluster_utf16 == 0)
            .collect();
        assert_eq!(group.len(), 2);

        // Spread across the column, not stacked down it.
        assert!(group[1].x_offset > group[0].x_offset);
        assert_eq!(group[0].y_advance, LayoutUnit::ZERO);
        // The pair sits on the face's own vertical origin, so it shares a
        // baseline with the upright characters around it.
        let upright = shaped
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .find(|glyph| glyph.cluster_utf16 == 2)
            .expect("the kanji after the group");
        assert_eq!(group[0].y_offset, upright.y_offset);
        assert!(group[0].y_offset < LayoutUnit::ZERO);
        // The pair advances exactly one em, and no further.
        assert_eq!(group[1].y_advance, LayoutUnit::from_raw(-em.raw()));
        // Both stay inside the cell, which is centred on the baseline.
        let half = LayoutUnit::from_raw(em.raw() / 2);
        assert!(group[0].x_offset >= LayoutUnit::from_raw(-half.raw()));
        assert!(group[1].x_offset <= half);
    }
}
