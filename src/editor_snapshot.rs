use core::fmt;
use std::collections::{HashMap, HashSet};

use unicode_bidi::{BidiInfo, Level};

use crate::{
    AnchorAffinity, FlowLayout, GlyphOrientation, LayoutUnit, Rect, ShapedCluster, ShapedParagraph,
    TextDirection, WritingMode,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorTextPosition {
    pub paragraph_id: String,
    pub utf16_offset: u32,
    pub affinity: AnchorAffinity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaretMovementDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorCaretGeometry {
    pub position: EditorTextPosition,
    pub rect: Rect,
    pub line_index: usize,
    pub fragment_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorSelectionGeometry {
    pub paragraph_id: String,
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub rects: Vec<Rect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaretMovement {
    pub position: EditorTextPosition,
    pub rect: Rect,
    pub preferred_x: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedLine {
    pub paragraph_id: String,
    pub line_index: usize,
    pub document_line_index: usize,
    pub top: LayoutUnit,
    pub baseline: LayoutUnit,
    pub writing_mode: WritingMode,
    pub block_start: LayoutUnit,
    pub inline_start: LayoutUnit,
    pub fragment_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedCaretStop {
    pub position: EditorTextPosition,
    pub rect: Rect,
    pub line_index: usize,
    pub document_line_index: usize,
    pub fragment_index: usize,
    pub visual_index: usize,
    pub writing_mode: WritingMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedCluster {
    pub paragraph_id: String,
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub direction: TextDirection,
    pub bidi_level: u8,
    pub orientation: GlyphOrientation,
    pub writing_mode: WritingMode,
    pub rect: Rect,
    pub line_index: usize,
    pub document_line_index: usize,
    pub fragment_index: usize,
    pub visual_index: usize,
    pub caret_stops: Vec<PositionedCaretStop>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedGlyph {
    pub paragraph_id: String,
    pub run_index: usize,
    pub glyph_index: usize,
    pub glyph_id: u32,
    pub cluster_utf16: u32,
    pub page_x: LayoutUnit,
    pub page_y: LayoutUnit,
    pub x_advance: LayoutUnit,
    pub y_advance: LayoutUnit,
    pub x_offset: LayoutUnit,
    pub y_offset: LayoutUnit,
    pub orientation: GlyphOrientation,
    pub writing_mode: WritingMode,
    pub line_index: usize,
    pub fragment_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorGeometrySnapshot {
    pub layout: FlowLayout,
    pub lines: Vec<PositionedLine>,
    pub clusters: Vec<PositionedCluster>,
    pub glyphs: Vec<PositionedGlyph>,
    pub caret_stops: Vec<PositionedCaretStop>,
    caret_width: LayoutUnit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditorGeometryError {
    DuplicateParagraph(String),
    MissingShapedParagraph(String),
    UnexpectedShapedParagraph(String),
    ParagraphLengthMismatch {
        paragraph_id: String,
        layout_length: u32,
        shaped_length: u32,
    },
    InvalidFragmentRange {
        paragraph_id: String,
        line_index: usize,
        fragment_index: usize,
    },
    InvalidCaretWidth,
    InvalidPosition(EditorTextPosition),
    InvalidSelectionRange {
        paragraph_id: String,
        utf16_start: u32,
        utf16_end: u32,
    },
}

impl fmt::Display for EditorGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateParagraph(id) => write!(f, "duplicate shaped paragraph {id}"),
            Self::MissingShapedParagraph(id) => write!(f, "missing shaped paragraph {id}"),
            Self::UnexpectedShapedParagraph(id) => write!(f, "unexpected shaped paragraph {id}"),
            Self::ParagraphLengthMismatch { paragraph_id, layout_length, shaped_length } => write!(f, "paragraph {paragraph_id} length mismatch: layout {layout_length}, shaped {shaped_length}"),
            Self::InvalidFragmentRange { paragraph_id, line_index, fragment_index } => write!(f, "paragraph {paragraph_id} line {line_index} fragment {fragment_index} has an invalid shaped range"),
            Self::InvalidCaretWidth => write!(f, "caret width must be positive"),
            Self::InvalidPosition(position) => write!(f, "invalid position {}:{} ({:?})", position.paragraph_id, position.utf16_offset, position.affinity),
            Self::InvalidSelectionRange { paragraph_id, utf16_start, utf16_end } => write!(f, "invalid selection range {paragraph_id}:{utf16_start}..{utf16_end}"),
        }
    }
}

impl std::error::Error for EditorGeometryError {}

impl EditorGeometrySnapshot {
    pub fn new(
        layout: FlowLayout,
        shaped_paragraphs: Vec<ShapedParagraph>,
        content: Rect,
        caret_width: LayoutUnit,
    ) -> Result<Self, EditorGeometryError> {
        if caret_width <= LayoutUnit::ZERO {
            return Err(EditorGeometryError::InvalidCaretWidth);
        }
        let mut shaped_by_id = HashMap::with_capacity(shaped_paragraphs.len());
        for paragraph in &shaped_paragraphs {
            if shaped_by_id
                .insert(paragraph.id.as_str(), paragraph)
                .is_some()
            {
                return Err(EditorGeometryError::DuplicateParagraph(
                    paragraph.id.clone(),
                ));
            }
        }
        let layout_ids: HashSet<&str> = layout
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.paragraph_id.as_str())
            .collect();
        if let Some(unexpected) = shaped_paragraphs
            .iter()
            .find(|paragraph| !layout_ids.contains(paragraph.id.as_str()))
        {
            return Err(EditorGeometryError::UnexpectedShapedParagraph(
                unexpected.id.clone(),
            ));
        }

        let mut lines = Vec::new();
        let mut clusters = Vec::new();
        let mut glyphs = Vec::new();
        let mut caret_stops = Vec::new();
        let mut document_line_index = 0;

        for paragraph_layout in &layout.paragraphs {
            let shaped = shaped_by_id
                .get(paragraph_layout.paragraph_id.as_str())
                .copied()
                .ok_or_else(|| {
                    EditorGeometryError::MissingShapedParagraph(
                        paragraph_layout.paragraph_id.clone(),
                    )
                })?;
            let layout_length = paragraph_layout
                .lines
                .iter()
                .flat_map(|line| &line.fragments)
                .map(|fragment| fragment.utf16_end)
                .max()
                .unwrap_or(0);
            if layout_length != shaped.utf16_len {
                return Err(EditorGeometryError::ParagraphLengthMismatch {
                    paragraph_id: shaped.id.clone(),
                    layout_length,
                    shaped_length: shaped.utf16_len,
                });
            }

            for (line_index, line) in paragraph_layout.lines.iter().enumerate() {
                lines.push(PositionedLine {
                    paragraph_id: shaped.id.clone(),
                    line_index,
                    document_line_index,
                    top: line.top,
                    baseline: line.baseline,
                    writing_mode: line.writing_mode,
                    block_start: line.block_start,
                    inline_start: line.inline_start,
                    fragment_count: line.fragments.len(),
                });
                if line.fragments.is_empty() && shaped.utf16_len == 0 {
                    let empty_rect = match shaped.writing_mode {
                        WritingMode::HorizontalTb => {
                            Rect::new(content.x, line.top, caret_width, line.baseline - line.top)
                        }
                        WritingMode::VerticalRl | WritingMode::VerticalLr => Rect::new(
                            line.block_start,
                            content.y,
                            (line.baseline - line.block_start).max(caret_width),
                            caret_width,
                        ),
                    };
                    caret_stops.push(PositionedCaretStop {
                        position: EditorTextPosition {
                            paragraph_id: shaped.id.clone(),
                            utf16_offset: 0,
                            affinity: AnchorAffinity::Downstream,
                        },
                        rect: empty_rect,
                        line_index,
                        document_line_index,
                        fragment_index: 0,
                        visual_index: 0,
                        writing_mode: shaped.writing_mode,
                    });
                }

                let mut line_visual_index = 0;
                for (fragment_index, fragment) in line.fragments.iter().enumerate() {
                    let logical_indices: Vec<usize> = shaped
                        .clusters
                        .iter()
                        .enumerate()
                        .filter(|(_, cluster)| {
                            cluster.utf16_start >= fragment.utf16_start
                                && cluster.utf16_end <= fragment.utf16_end
                        })
                        .map(|(index, _)| index)
                        .collect();
                    if logical_indices.is_empty()
                        || shaped.clusters[*logical_indices.first().unwrap()].utf16_start
                            != fragment.utf16_start
                        || shaped.clusters[*logical_indices.last().unwrap()].utf16_end
                            != fragment.utf16_end
                    {
                        return Err(EditorGeometryError::InvalidFragmentRange {
                            paragraph_id: shaped.id.clone(),
                            line_index,
                            fragment_index,
                        });
                    }
                    let visual_indices = visual_cluster_indices(shaped, &logical_indices);
                    let mut cluster_inline = match shaped.writing_mode {
                        WritingMode::HorizontalTb => fragment.rect.x,
                        WritingMode::VerticalRl | WritingMode::VerticalLr => fragment.rect.y,
                    };
                    for cluster_index in visual_indices {
                        let shaped_cluster = &shaped.clusters[cluster_index];
                        let cluster_rect = match shaped.writing_mode {
                            WritingMode::HorizontalTb => Rect::new(
                                cluster_inline,
                                line.top,
                                shaped_cluster.advance,
                                fragment.rect.height,
                            ),
                            WritingMode::VerticalRl | WritingMode::VerticalLr => Rect::new(
                                fragment.rect.x,
                                cluster_inline,
                                fragment.rect.width,
                                shaped_cluster.advance,
                            ),
                        };
                        let positioned_stops = positioned_cluster_stops(
                            shaped,
                            shaped_cluster,
                            cluster_rect,
                            line_index,
                            document_line_index,
                            fragment_index,
                            line_visual_index,
                            caret_width,
                        );
                        caret_stops.extend(positioned_stops.iter().cloned());
                        append_positioned_glyphs(
                            &mut glyphs,
                            shaped,
                            shaped_cluster,
                            cluster_rect,
                            line.baseline,
                            line_index,
                            fragment_index,
                        );
                        clusters.push(PositionedCluster {
                            paragraph_id: shaped.id.clone(),
                            utf16_start: shaped_cluster.utf16_start,
                            utf16_end: shaped_cluster.utf16_end,
                            direction: shaped_cluster.direction,
                            bidi_level: shaped_cluster.bidi_level,
                            orientation: shaped_cluster.orientation,
                            writing_mode: shaped.writing_mode,
                            rect: cluster_rect,
                            line_index,
                            document_line_index,
                            fragment_index,
                            visual_index: line_visual_index,
                            caret_stops: positioned_stops,
                        });
                        cluster_inline += shaped_cluster.advance;
                        line_visual_index += 1;
                    }
                }
                document_line_index += 1;
            }
        }

        caret_stops.sort_by_key(|stop| {
            (
                stop.document_line_index,
                caret_inline_coordinate(stop),
                stop.fragment_index,
                stop.position.utf16_offset,
                affinity_rank(stop.position.affinity),
            )
        });
        caret_stops.dedup_by(|left, right| {
            left.position == right.position
                && left.rect.x == right.rect.x
                && left.rect.y == right.rect.y
        });
        Ok(Self {
            layout,
            lines,
            clusters,
            glyphs,
            caret_stops,
            caret_width,
        })
    }

    pub fn caret(
        &self,
        position: &EditorTextPosition,
    ) -> Result<EditorCaretGeometry, EditorGeometryError> {
        let matching: Vec<&PositionedCaretStop> = self
            .caret_stops
            .iter()
            .filter(|stop| {
                stop.position.paragraph_id == position.paragraph_id
                    && stop.position.utf16_offset == position.utf16_offset
            })
            .collect();
        let stop = matching
            .iter()
            .copied()
            .find(|stop| stop.position.affinity == position.affinity)
            .or_else(|| matching.first().copied())
            .ok_or_else(|| EditorGeometryError::InvalidPosition(position.clone()))?;
        Ok(EditorCaretGeometry {
            position: position.clone(),
            rect: stop.rect,
            line_index: stop.line_index,
            fragment_index: stop.fragment_index,
        })
    }

    pub fn hit_test(&self, x: LayoutUnit, y: LayoutUnit) -> Option<EditorTextPosition> {
        self.caret_stops
            .iter()
            .min_by_key(|stop| {
                let vertical = distance_to_interval(y, stop.rect.y, stop.rect.bottom());
                let horizontal = distance_to_interval(x, stop.rect.x, stop.rect.right());
                (
                    vertical.saturating_add(horizontal),
                    if stop.writing_mode == WritingMode::HorizontalTb {
                        vertical
                    } else {
                        horizontal
                    },
                    caret_inline_coordinate(stop).raw(),
                    stop.document_line_index,
                    stop.fragment_index,
                    stop.visual_index,
                    affinity_rank(stop.position.affinity),
                )
            })
            .map(|stop| stop.position.clone())
    }

    pub fn selection(
        &self,
        paragraph_id: &str,
        utf16_start: u32,
        utf16_end: u32,
    ) -> Result<EditorSelectionGeometry, EditorGeometryError> {
        let paragraph_length = self
            .caret_stops
            .iter()
            .filter(|stop| stop.position.paragraph_id == paragraph_id)
            .map(|stop| stop.position.utf16_offset)
            .max();
        if utf16_start > utf16_end
            || paragraph_length.is_none()
            || utf16_end > paragraph_length.unwrap_or(0)
        {
            return Err(EditorGeometryError::InvalidSelectionRange {
                paragraph_id: paragraph_id.into(),
                utf16_start,
                utf16_end,
            });
        }

        let mut pieces: Vec<(usize, usize, WritingMode, Rect)> = Vec::new();
        for cluster in self
            .clusters
            .iter()
            .filter(|cluster| cluster.paragraph_id == paragraph_id)
        {
            let start = utf16_start.max(cluster.utf16_start);
            let end = utf16_end.min(cluster.utf16_end);
            if start >= end {
                continue;
            }
            let Some(start_inline) = cluster_inline_for_offset(cluster, start) else {
                continue;
            };
            let Some(end_inline) = cluster_inline_for_offset(cluster, end) else {
                continue;
            };
            let inline_start = start_inline.min(end_inline);
            let inline_extent =
                LayoutUnit::from_raw((end_inline.raw() - start_inline.raw()).saturating_abs());
            let rect = match cluster.writing_mode {
                WritingMode::HorizontalTb => Rect::new(
                    inline_start,
                    cluster.rect.y,
                    inline_extent,
                    cluster.rect.height,
                ),
                WritingMode::VerticalRl | WritingMode::VerticalLr => Rect::new(
                    cluster.rect.x,
                    inline_start,
                    cluster.rect.width,
                    inline_extent,
                ),
            };
            pieces.push((
                cluster.document_line_index,
                cluster.fragment_index,
                cluster.writing_mode,
                rect,
            ));
        }
        pieces.sort_by_key(|(line, fragment, writing_mode, rect)| {
            (*line, *fragment, rect_inline_start(*rect, *writing_mode))
        });

        let mut merged: Vec<(usize, usize, WritingMode, Rect)> = Vec::new();
        for (line, fragment, writing_mode, rect) in pieces {
            if let Some((last_line, last_fragment, last_mode, last)) = merged.last_mut() {
                if *last_line == line
                    && *last_fragment == fragment
                    && *last_mode == writing_mode
                    && selection_rects_touch(*last, rect, writing_mode)
                {
                    merge_selection_rect(last, rect, writing_mode);
                    continue;
                }
            }
            merged.push((line, fragment, writing_mode, rect));
        }
        Ok(EditorSelectionGeometry {
            paragraph_id: paragraph_id.into(),
            utf16_start,
            utf16_end,
            rects: merged.into_iter().map(|(_, _, _, rect)| rect).collect(),
        })
    }

    pub fn move_caret(
        &self,
        position: &EditorTextPosition,
        direction: CaretMovementDirection,
        preferred_x: Option<LayoutUnit>,
    ) -> Result<CaretMovement, EditorGeometryError> {
        let current = self.caret(position)?;
        let slots = self.movement_slots();
        let current_index = slots
            .iter()
            .position(|slot| {
                slot.position.paragraph_id == position.paragraph_id
                    && slot.position.utf16_offset == position.utf16_offset
                    && slot.rect.x == current.rect.x
                    && slot.rect.y == current.rect.y
            })
            .ok_or_else(|| EditorGeometryError::InvalidPosition(position.clone()))?;

        let current_slot = &slots[current_index];
        let mode = current_slot.writing_mode;
        let inline_motion = matches!(
            (mode, direction),
            (WritingMode::HorizontalTb, CaretMovementDirection::Left)
                | (WritingMode::HorizontalTb, CaretMovementDirection::Right)
                | (WritingMode::VerticalRl, CaretMovementDirection::Up)
                | (WritingMode::VerticalRl, CaretMovementDirection::Down)
                | (WritingMode::VerticalLr, CaretMovementDirection::Up)
                | (WritingMode::VerticalLr, CaretMovementDirection::Down)
        );
        let (target, sticky_inline) = if inline_motion {
            let moves_backward = matches!(
                direction,
                CaretMovementDirection::Left | CaretMovementDirection::Up
            );
            let candidate = if moves_backward {
                current_index
                    .checked_sub(1)
                    .and_then(|index| slots.get(index))
            } else {
                slots.get(current_index + 1)
            }
            .filter(|slot| slot.document_line_index == current_slot.document_line_index);
            (candidate.unwrap_or(current_slot), None)
        } else {
            let wanted_inline =
                preferred_x.unwrap_or_else(|| caret_inline_coordinate(current_slot));
            let mut text_lines: Vec<usize> =
                slots.iter().map(|slot| slot.document_line_index).collect();
            text_lines.sort_unstable();
            text_lines.dedup();
            let line_position = text_lines
                .iter()
                .position(|line| *line == current_slot.document_line_index);
            let toward_next_line = match (mode, direction) {
                (WritingMode::HorizontalTb, CaretMovementDirection::Down) => true,
                (WritingMode::HorizontalTb, CaretMovementDirection::Up) => false,
                (WritingMode::VerticalLr, CaretMovementDirection::Right) => true,
                (WritingMode::VerticalLr, CaretMovementDirection::Left) => false,
                (WritingMode::VerticalRl, CaretMovementDirection::Left) => true,
                (WritingMode::VerticalRl, CaretMovementDirection::Right) => false,
                _ => false,
            };
            let target_line = line_position.and_then(|position| {
                if toward_next_line {
                    text_lines.get(position + 1).copied()
                } else {
                    position.checked_sub(1).map(|index| text_lines[index])
                }
            });
            let candidate = target_line.and_then(|line| {
                slots
                    .iter()
                    .filter(|slot| slot.document_line_index == line)
                    .min_by_key(|slot| {
                        (
                            (i64::from(caret_inline_coordinate(slot).raw())
                                - i64::from(wanted_inline.raw()))
                            .abs(),
                            caret_inline_coordinate(slot),
                            slot.visual_index,
                        )
                    })
            });
            (candidate.unwrap_or(current_slot), Some(wanted_inline))
        };
        Ok(CaretMovement {
            position: target.position.clone(),
            rect: target.rect,
            preferred_x: sticky_inline.unwrap_or_else(|| caret_inline_coordinate(target)),
        })
    }

    pub fn caret_width(&self) -> LayoutUnit {
        self.caret_width
    }

    fn movement_slots(&self) -> Vec<PositionedCaretStop> {
        let mut slots = self.caret_stops.clone();
        slots.sort_by_key(|slot| {
            (
                slot.document_line_index,
                caret_inline_coordinate(slot),
                slot.fragment_index,
                slot.visual_index,
                affinity_rank(slot.position.affinity),
            )
        });
        slots.dedup_by(|left, right| {
            if left.document_line_index == right.document_line_index
                && caret_inline_coordinate(left) == caret_inline_coordinate(right)
                && left.position.paragraph_id == right.position.paragraph_id
                && left.position.utf16_offset == right.position.utf16_offset
            {
                if right.position.affinity == AnchorAffinity::Downstream {
                    left.position.affinity = AnchorAffinity::Downstream;
                }
                true
            } else {
                false
            }
        });
        slots
    }
}

fn visual_cluster_indices(shaped: &ShapedParagraph, logical: &[usize]) -> Vec<usize> {
    let base_level = match shaped.base_direction {
        TextDirection::LeftToRight => 0,
        TextDirection::RightToLeft => 1,
    };
    let trailing_non_whitespace = logical
        .iter()
        .rposition(|index| !shaped.clusters[*index].is_whitespace);
    let levels: Vec<Level> = logical
        .iter()
        .enumerate()
        .map(|(position, index)| {
            let number = if trailing_non_whitespace.is_none_or(|last| position > last) {
                base_level
            } else {
                shaped.clusters[*index].bidi_level
            };
            Level::new(number).expect("Unicode bidi levels are bounded")
        })
        .collect();
    BidiInfo::reorder_visual(&levels)
        .into_iter()
        .map(|visual_index| logical[visual_index])
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn positioned_cluster_stops(
    shaped: &ShapedParagraph,
    cluster: &ShapedCluster,
    rect: Rect,
    line_index: usize,
    document_line_index: usize,
    fragment_index: usize,
    visual_index: usize,
    caret_width: LayoutUnit,
) -> Vec<PositionedCaretStop> {
    let mut result = Vec::new();
    for stop in &cluster.caret_stops {
        let affinities: &[AnchorAffinity] = if stop.utf16_offset == cluster.utf16_start {
            &[AnchorAffinity::Downstream]
        } else if stop.utf16_offset == cluster.utf16_end {
            &[AnchorAffinity::Upstream]
        } else {
            &[AnchorAffinity::Upstream, AnchorAffinity::Downstream]
        };
        for affinity in affinities {
            result.push(PositionedCaretStop {
                position: EditorTextPosition {
                    paragraph_id: shaped.id.clone(),
                    utf16_offset: stop.utf16_offset,
                    affinity: *affinity,
                },
                rect: match shaped.writing_mode {
                    WritingMode::HorizontalTb => Rect::new(
                        rect.x + stop.inline_offset,
                        rect.y,
                        caret_width,
                        rect.height,
                    ),
                    WritingMode::VerticalRl | WritingMode::VerticalLr => {
                        Rect::new(rect.x, rect.y + stop.inline_offset, rect.width, caret_width)
                    }
                },
                line_index,
                document_line_index,
                fragment_index,
                visual_index,
                writing_mode: shaped.writing_mode,
            });
        }
    }
    result
}

fn append_positioned_glyphs(
    output: &mut Vec<PositionedGlyph>,
    shaped: &ShapedParagraph,
    cluster: &ShapedCluster,
    rect: Rect,
    baseline: LayoutUnit,
    line_index: usize,
    fragment_index: usize,
) {
    let Some((run_index, run)) = shaped.runs.iter().enumerate().find(|(_, run)| {
        cluster.utf16_start >= run.utf16_start && cluster.utf16_end <= run.utf16_end
    }) else {
        return;
    };
    let matching: Vec<(usize, _)> = run
        .glyphs
        .iter()
        .enumerate()
        .filter(|(_, glyph)| glyph.cluster_utf16 == cluster.utf16_start)
        .collect();
    let mut pen_inline = match (shaped.writing_mode, cluster.direction) {
        (WritingMode::HorizontalTb, TextDirection::LeftToRight) => rect.x,
        (WritingMode::HorizontalTb, TextDirection::RightToLeft) => rect.right(),
        (_, TextDirection::LeftToRight) => rect.y,
        (_, TextDirection::RightToLeft) => rect.bottom(),
    };
    for (glyph_index, glyph) in matching {
        let uses_vertical_metrics =
            shaped.writing_mode.is_vertical() && cluster.orientation == GlyphOrientation::Upright;
        let raw_advance = if uses_vertical_metrics {
            glyph.y_advance
        } else {
            glyph.x_advance
        };
        let absolute_advance = LayoutUnit::from_raw(raw_advance.raw().saturating_abs());
        if cluster.direction == TextDirection::RightToLeft {
            pen_inline -= absolute_advance;
        }
        let (page_x, page_y) = match shaped.writing_mode {
            WritingMode::HorizontalTb => (pen_inline + glyph.x_offset, baseline - glyph.y_offset),
            WritingMode::VerticalRl | WritingMode::VerticalLr
                if cluster.orientation == GlyphOrientation::Upright =>
            {
                (baseline + glyph.x_offset, pen_inline - glyph.y_offset)
            }
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                // A sideways item is rotated clockwise around its own origin;
                // the paragraph itself is never post-rotated.
                (baseline + glyph.y_offset, pen_inline + glyph.x_offset)
            }
        };
        output.push(PositionedGlyph {
            paragraph_id: shaped.id.clone(),
            run_index,
            glyph_index,
            glyph_id: glyph.glyph_id,
            cluster_utf16: glyph.cluster_utf16,
            page_x,
            page_y,
            x_advance: glyph.x_advance,
            y_advance: glyph.y_advance,
            x_offset: glyph.x_offset,
            y_offset: glyph.y_offset,
            orientation: cluster.orientation,
            writing_mode: shaped.writing_mode,
            line_index,
            fragment_index,
        });
        if cluster.direction == TextDirection::LeftToRight {
            pen_inline += absolute_advance;
        }
    }
}

fn cluster_inline_for_offset(cluster: &PositionedCluster, offset: u32) -> Option<LayoutUnit> {
    cluster
        .caret_stops
        .iter()
        .find(|stop| stop.position.utf16_offset == offset)
        .map(|stop| match cluster.writing_mode {
            WritingMode::HorizontalTb => stop.rect.x,
            WritingMode::VerticalRl | WritingMode::VerticalLr => stop.rect.y,
        })
}

fn caret_inline_coordinate(stop: &PositionedCaretStop) -> LayoutUnit {
    match stop.writing_mode {
        WritingMode::HorizontalTb => stop.rect.x,
        WritingMode::VerticalRl | WritingMode::VerticalLr => stop.rect.y,
    }
}

fn rect_inline_start(rect: Rect, writing_mode: WritingMode) -> LayoutUnit {
    match writing_mode {
        WritingMode::HorizontalTb => rect.x,
        WritingMode::VerticalRl | WritingMode::VerticalLr => rect.y,
    }
}

fn selection_rects_touch(left: Rect, right: Rect, writing_mode: WritingMode) -> bool {
    match writing_mode {
        WritingMode::HorizontalTb => {
            left.y == right.y && left.height == right.height && right.x <= left.right()
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            left.x == right.x && left.width == right.width && right.y <= left.bottom()
        }
    }
}

fn merge_selection_rect(target: &mut Rect, other: Rect, writing_mode: WritingMode) {
    match writing_mode {
        WritingMode::HorizontalTb => {
            let right = target.right().max(other.right());
            target.width = right - target.x;
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let bottom = target.bottom().max(other.bottom());
            target.height = bottom - target.y;
        }
    }
}

fn distance_to_interval(value: LayoutUnit, start: LayoutUnit, end: LayoutUnit) -> i64 {
    if value < start {
        i64::from(start.raw()) - i64::from(value.raw())
    } else if value > end {
        i64::from(value.raw()) - i64::from(end.raw())
    } else {
        0
    }
}

fn affinity_rank(affinity: AnchorAffinity) -> u8 {
    match affinity {
        AnchorAffinity::Upstream => 0,
        AnchorAffinity::Downstream => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClusterCaretStop, FlowFragment, FlowLine, ParagraphLayout, ShapedRun};

    fn cluster(start: u32) -> ShapedCluster {
        ShapedCluster {
            utf16_start: start,
            utf16_end: start + 1,
            advance: LayoutUnit::from_raw(10),
            can_break_after: true,
            is_whitespace: false,
            bidi_level: 0,
            direction: TextDirection::LeftToRight,
            orientation: GlyphOrientation::Upright,
            caret_stops: vec![
                ClusterCaretStop {
                    utf16_offset: start,
                    inline_offset: LayoutUnit::ZERO,
                },
                ClusterCaretStop {
                    utf16_offset: start + 1,
                    inline_offset: LayoutUnit::from_raw(10),
                },
            ],
        }
    }

    fn snapshot() -> EditorGeometrySnapshot {
        let layout = FlowLayout {
            paragraphs: vec![ParagraphLayout {
                paragraph_id: "p".into(),
                top: LayoutUnit::ZERO,
                bottom: LayoutUnit::from_raw(20),
                bounds: Rect::new(
                    LayoutUnit::ZERO,
                    LayoutUnit::ZERO,
                    LayoutUnit::from_raw(60),
                    LayoutUnit::from_raw(20),
                ),
                base_direction: TextDirection::LeftToRight,
                writing_mode: WritingMode::HorizontalTb,
                lines: vec![
                    FlowLine {
                        top: LayoutUnit::ZERO,
                        baseline: LayoutUnit::from_raw(8),
                        writing_mode: WritingMode::HorizontalTb,
                        block_start: LayoutUnit::ZERO,
                        inline_start: LayoutUnit::ZERO,
                        fragments: vec![
                            FlowFragment {
                                rect: Rect::new(
                                    LayoutUnit::ZERO,
                                    LayoutUnit::ZERO,
                                    LayoutUnit::from_raw(20),
                                    LayoutUnit::from_raw(10),
                                ),
                                utf16_start: 0,
                                utf16_end: 2,
                            },
                            FlowFragment {
                                rect: Rect::new(
                                    LayoutUnit::from_raw(40),
                                    LayoutUnit::ZERO,
                                    LayoutUnit::from_raw(20),
                                    LayoutUnit::from_raw(10),
                                ),
                                utf16_start: 2,
                                utf16_end: 4,
                            },
                        ],
                    },
                    FlowLine {
                        top: LayoutUnit::from_raw(10),
                        baseline: LayoutUnit::from_raw(18),
                        writing_mode: WritingMode::HorizontalTb,
                        block_start: LayoutUnit::from_raw(10),
                        inline_start: LayoutUnit::ZERO,
                        fragments: vec![FlowFragment {
                            rect: Rect::new(
                                LayoutUnit::ZERO,
                                LayoutUnit::from_raw(10),
                                LayoutUnit::from_raw(20),
                                LayoutUnit::from_raw(10),
                            ),
                            utf16_start: 4,
                            utf16_end: 6,
                        }],
                    },
                ],
            }],
            objects: vec![],
            content_height: LayoutUnit::from_raw(20),
            content_width: LayoutUnit::from_raw(60),
        };
        let shaped = ShapedParagraph {
            id: "p".into(),
            utf16_len: 6,
            requested_base_direction: crate::ParagraphDirection::LeftToRight,
            base_direction: TextDirection::LeftToRight,
            writing_mode: WritingMode::HorizontalTb,
            text_orientation: crate::TextOrientation::Mixed,
            runs: vec![ShapedRun {
                utf16_start: 0,
                utf16_end: 6,
                font: crate::FontDescriptor {
                    id: "fixture".into(),
                    sha256: "0".repeat(64),
                    face_index: 0,
                    units_per_em: 1000,
                },
                font_size: LayoutUnit::from_raw(16 * 64),
                bidi_level: 0,
                direction: TextDirection::LeftToRight,
                orientation: GlyphOrientation::Upright,
                glyphs: vec![],
                advance: LayoutUnit::from_raw(60),
            }],
            clusters: (0..6).map(cluster).collect(),
        };
        EditorGeometrySnapshot::new(
            layout,
            vec![shaped],
            Rect::new(
                LayoutUnit::ZERO,
                LayoutUnit::ZERO,
                LayoutUnit::from_raw(100),
                LayoutUnit::from_raw(100),
            ),
            LayoutUnit::from_raw(1),
        )
        .unwrap()
    }

    #[test]
    fn exclusion_boundary_has_distinct_affinity_geometry() {
        let snapshot = snapshot();
        let upstream = snapshot
            .caret(&EditorTextPosition {
                paragraph_id: "p".into(),
                utf16_offset: 2,
                affinity: AnchorAffinity::Upstream,
            })
            .unwrap();
        let downstream = snapshot
            .caret(&EditorTextPosition {
                paragraph_id: "p".into(),
                utf16_offset: 2,
                affinity: AnchorAffinity::Downstream,
            })
            .unwrap();
        assert_eq!(upstream.rect.x, LayoutUnit::from_raw(20));
        assert_eq!(downstream.rect.x, LayoutUnit::from_raw(40));
        assert_eq!(
            snapshot
                .hit_test(LayoutUnit::from_raw(21), LayoutUnit::from_raw(5))
                .unwrap()
                .affinity,
            AnchorAffinity::Upstream
        );
        assert_eq!(
            snapshot
                .hit_test(LayoutUnit::from_raw(39), LayoutUnit::from_raw(5))
                .unwrap()
                .affinity,
            AnchorAffinity::Downstream
        );
    }

    #[test]
    fn selection_crosses_both_fragments_and_the_next_line() {
        let selection = snapshot().selection("p", 1, 5).unwrap();
        assert_eq!(selection.rects.len(), 3);
        assert_eq!(selection.rects[0].x, LayoutUnit::from_raw(10));
        assert_eq!(selection.rects[1].x, LayoutUnit::from_raw(40));
        assert_eq!(selection.rects[2].y, LayoutUnit::from_raw(10));
    }

    #[test]
    fn visual_movement_crosses_the_exclusion_gap() {
        let snapshot = snapshot();
        let movement = snapshot
            .move_caret(
                &EditorTextPosition {
                    paragraph_id: "p".into(),
                    utf16_offset: 2,
                    affinity: AnchorAffinity::Upstream,
                },
                CaretMovementDirection::Right,
                None,
            )
            .unwrap();
        assert_eq!(movement.position.affinity, AnchorAffinity::Downstream);
        assert_eq!(movement.rect.x, LayoutUnit::from_raw(40));
    }
}
