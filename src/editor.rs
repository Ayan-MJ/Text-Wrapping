use crate::{
    AnchorAffinity, FlowFragment, FlowLayout, FlowLine, LayoutUnit, Rect, ShapedParagraph,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextPosition {
    pub paragraph_id: String,
    pub utf16_offset: u32,
    pub affinity: AnchorAffinity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaretGeometry {
    pub position: TextPosition,
    pub rect: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionGeometry {
    pub paragraph_id: String,
    pub utf16_start: u32,
    pub utf16_end: u32,
    pub rects: Vec<Rect>,
}

#[derive(Clone, Copy)]
struct CaretCandidate {
    offset: u32,
    rect: Rect,
    line_index: usize,
}

pub fn caret_geometry(
    layout: &FlowLayout,
    shaped: &ShapedParagraph,
    position: &TextPosition,
    caret_width: LayoutUnit,
) -> Option<CaretGeometry> {
    if position.paragraph_id != shaped.id || position.utf16_offset > shaped.utf16_len {
        return None;
    }
    let candidates = caret_candidates(layout, shaped, caret_width);
    let mut matching: Vec<CaretCandidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.offset == position.utf16_offset)
        .collect();
    matching.sort_by_key(|candidate| candidate.line_index);
    let candidate = match position.affinity {
        AnchorAffinity::Upstream => matching.first(),
        AnchorAffinity::Downstream => matching.last(),
    }?;
    Some(CaretGeometry {
        position: position.clone(),
        rect: candidate.rect,
    })
}

pub fn selection_geometry(
    layout: &FlowLayout,
    shaped: &ShapedParagraph,
    utf16_start: u32,
    utf16_end: u32,
) -> Option<SelectionGeometry> {
    if utf16_start > utf16_end || utf16_end > shaped.utf16_len {
        return None;
    }
    let paragraph = layout
        .paragraphs
        .iter()
        .find(|paragraph| paragraph.paragraph_id == shaped.id)?;
    let mut rects = Vec::new();
    for line in &paragraph.lines {
        for fragment in &line.fragments {
            let start = utf16_start.max(fragment.utf16_start);
            let end = utf16_end.min(fragment.utf16_end);
            if start >= end {
                continue;
            }
            let start_x = inline_for_offset(shaped, fragment, start)?;
            let end_x = inline_for_offset(shaped, fragment, end)?;
            rects.push(Rect::new(
                start_x.min(end_x),
                line.top,
                LayoutUnit::from_raw((end_x.raw() - start_x.raw()).saturating_abs()),
                fragment.rect.height,
            ));
        }
    }
    Some(SelectionGeometry {
        paragraph_id: shaped.id.clone(),
        utf16_start,
        utf16_end,
        rects,
    })
}

pub fn text_position_at_point(
    layout: &FlowLayout,
    shaped: &ShapedParagraph,
    x: LayoutUnit,
    y: LayoutUnit,
) -> Option<TextPosition> {
    let candidates = caret_candidates(layout, shaped, LayoutUnit::ZERO);
    let candidate = candidates.into_iter().min_by_key(|candidate| {
        let vertical = distance_to_interval(y, candidate.rect.y, candidate.rect.bottom());
        let horizontal = (i64::from(x.raw()) - i64::from(candidate.rect.x.raw())).abs();
        (vertical, horizontal, candidate.line_index)
    })?;
    Some(TextPosition {
        paragraph_id: shaped.id.clone(),
        utf16_offset: candidate.offset,
        affinity: AnchorAffinity::Downstream,
    })
}

fn caret_candidates(
    layout: &FlowLayout,
    shaped: &ShapedParagraph,
    caret_width: LayoutUnit,
) -> Vec<CaretCandidate> {
    let Some(paragraph) = layout
        .paragraphs
        .iter()
        .find(|paragraph| paragraph.paragraph_id == shaped.id)
    else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (line_index, line) in paragraph.lines.iter().enumerate() {
        for fragment in &line.fragments {
            append_fragment_carets(&mut result, shaped, line, fragment, line_index, caret_width);
        }
    }
    result.dedup_by(|left, right| {
        left.offset == right.offset && left.rect.x == right.rect.x && left.rect.y == right.rect.y
    });
    result
}

fn append_fragment_carets(
    output: &mut Vec<CaretCandidate>,
    shaped: &ShapedParagraph,
    line: &FlowLine,
    fragment: &FlowFragment,
    line_index: usize,
    caret_width: LayoutUnit,
) {
    let mut cluster_x = fragment.rect.x;
    for cluster in shaped.clusters.iter().filter(|cluster| {
        cluster.utf16_start >= fragment.utf16_start && cluster.utf16_end <= fragment.utf16_end
    }) {
        for stop in &cluster.caret_stops {
            if stop.utf16_offset >= fragment.utf16_start && stop.utf16_offset <= fragment.utf16_end
            {
                output.push(CaretCandidate {
                    offset: stop.utf16_offset,
                    rect: Rect::new(
                        cluster_x + stop.inline_offset,
                        line.top,
                        caret_width,
                        fragment.rect.height,
                    ),
                    line_index,
                });
            }
        }
        cluster_x += cluster.advance;
    }
}

fn inline_for_offset(
    shaped: &ShapedParagraph,
    fragment: &FlowFragment,
    offset: u32,
) -> Option<LayoutUnit> {
    let mut cluster_x = fragment.rect.x;
    for cluster in shaped.clusters.iter().filter(|cluster| {
        cluster.utf16_start >= fragment.utf16_start && cluster.utf16_end <= fragment.utf16_end
    }) {
        if let Some(stop) = cluster
            .caret_stops
            .iter()
            .find(|stop| stop.utf16_offset == offset)
        {
            return Some(cluster_x + stop.inline_offset);
        }
        cluster_x += cluster.advance;
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FlowLine, ParagraphLayout, ShapedCluster, TextDirection};

    fn cluster(start: u32) -> ShapedCluster {
        ShapedCluster {
            utf16_start: start,
            utf16_end: start + 1,
            advance: LayoutUnit::from_raw(10),
            can_break_after: true,
            is_whitespace: false,
            bidi_level: 0,
            direction: TextDirection::LeftToRight,
            orientation: crate::GlyphOrientation::Upright,
            caret_stops: vec![
                crate::ClusterCaretStop {
                    utf16_offset: start,
                    inline_offset: LayoutUnit::ZERO,
                },
                crate::ClusterCaretStop {
                    utf16_offset: start + 1,
                    inline_offset: LayoutUnit::from_raw(10),
                },
            ],
        }
    }

    fn fixture() -> (FlowLayout, ShapedParagraph) {
        let first = FlowFragment {
            rect: Rect::new(
                LayoutUnit::ZERO,
                LayoutUnit::ZERO,
                LayoutUnit::from_raw(20),
                LayoutUnit::from_raw(10),
            ),
            utf16_start: 0,
            utf16_end: 2,
        };
        let second = FlowFragment {
            rect: Rect::new(
                LayoutUnit::from_raw(40),
                LayoutUnit::ZERO,
                LayoutUnit::from_raw(20),
                LayoutUnit::from_raw(10),
            ),
            utf16_start: 2,
            utf16_end: 4,
        };
        (
            FlowLayout {
                paragraphs: vec![ParagraphLayout {
                    paragraph_id: "p".into(),
                    top: LayoutUnit::ZERO,
                    bottom: LayoutUnit::from_raw(10),
                    bounds: Rect::new(
                        LayoutUnit::ZERO,
                        LayoutUnit::ZERO,
                        LayoutUnit::from_raw(60),
                        LayoutUnit::from_raw(10),
                    ),
                    base_direction: TextDirection::LeftToRight,
                    writing_mode: crate::WritingMode::HorizontalTb,
                    lines: vec![FlowLine {
                        top: LayoutUnit::ZERO,
                        baseline: LayoutUnit::from_raw(8),
                        writing_mode: crate::WritingMode::HorizontalTb,
                        block_start: LayoutUnit::ZERO,
                        inline_start: LayoutUnit::ZERO,
                        fragments: vec![first, second],
                    }],
                }],
                objects: Vec::new(),
                content_height: LayoutUnit::from_raw(10),
                content_width: LayoutUnit::from_raw(60),
            },
            ShapedParagraph {
                id: "p".into(),
                utf16_len: 4,
                requested_base_direction: crate::ParagraphDirection::LeftToRight,
                base_direction: TextDirection::LeftToRight,
                writing_mode: crate::WritingMode::HorizontalTb,
                text_orientation: crate::TextOrientation::Mixed,
                runs: Vec::new(),
                clusters: (0..4).map(cluster).collect(),
            },
        )
    }

    #[test]
    fn affinity_selects_the_correct_side_of_an_exclusion_gap() {
        let (layout, shaped) = fixture();
        let upstream = caret_geometry(
            &layout,
            &shaped,
            &TextPosition {
                paragraph_id: "p".into(),
                utf16_offset: 2,
                affinity: AnchorAffinity::Upstream,
            },
            LayoutUnit::from_raw(1),
        )
        .unwrap();
        let downstream = caret_geometry(
            &layout,
            &shaped,
            &TextPosition {
                paragraph_id: "p".into(),
                utf16_offset: 2,
                affinity: AnchorAffinity::Downstream,
            },
            LayoutUnit::from_raw(1),
        )
        .unwrap();
        assert_eq!(upstream.rect.x, LayoutUnit::from_raw(20));
        assert_eq!(downstream.rect.x, LayoutUnit::from_raw(40));
    }

    #[test]
    fn selection_is_split_across_exclusion_fragments() {
        let (layout, shaped) = fixture();
        let selection = selection_geometry(&layout, &shaped, 1, 3).unwrap();
        assert_eq!(selection.rects.len(), 2);
        assert_eq!(selection.rects[0].x, LayoutUnit::from_raw(10));
        assert_eq!(selection.rects[1].x, LayoutUnit::from_raw(40));
    }

    #[test]
    fn hit_testing_uses_shared_caret_stops() {
        let (layout, shaped) = fixture();
        let position = text_position_at_point(
            &layout,
            &shaped,
            LayoutUnit::from_raw(43),
            LayoutUnit::from_raw(5),
        )
        .unwrap();
        assert_eq!(position.utf16_offset, 2);
    }
}
