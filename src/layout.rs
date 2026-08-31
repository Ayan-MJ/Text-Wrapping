use core::fmt;
use std::collections::{HashMap, HashSet};

use crate::fixed::{scale_block_offset, scale_ratio};
use crate::geometry::subtract_intervals;
use crate::{
    AnchorAffinity, Cluster, FlowFragment, FlowLayout, FlowLine, FlowObject, Interval,
    LayoutRequest, LayoutUnit, ObjectLayoutMode, Paragraph, ParagraphLayout, Rect, ResolvedObject,
};

const MAX_LINES_PER_PARAGRAPH: usize = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidContentWidth,
    DuplicateParagraphId(String),
    DuplicateObjectId(String),
    MissingAnchorParagraph {
        object_id: String,
        paragraph_id: String,
    },
    InvalidParagraphMetrics(String),
    InvalidClusterRange {
        paragraph_id: String,
        cluster_index: usize,
    },
    InvalidClusterAdvance {
        paragraph_id: String,
        cluster_index: usize,
    },
    InvalidAnchorOffset {
        object_id: String,
        offset: u32,
        paragraph_len: u32,
    },
    InvalidObjectSize(String),
    InvalidExclusionMargin(String),
    LayoutDidNotAdvance(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContentWidth => write!(f, "content width must be positive"),
            Self::DuplicateParagraphId(id) => write!(f, "duplicate paragraph id {id}"),
            Self::DuplicateObjectId(id) => write!(f, "duplicate object id {id}"),
            Self::MissingAnchorParagraph {
                object_id,
                paragraph_id,
            } => write!(
                f,
                "object {object_id} references missing paragraph {paragraph_id}"
            ),
            Self::InvalidParagraphMetrics(id) => {
                write!(f, "paragraph {id} has invalid vertical metrics")
            }
            Self::InvalidClusterRange {
                paragraph_id,
                cluster_index,
            } => write!(
                f,
                "paragraph {paragraph_id} has an invalid cluster at index {cluster_index}"
            ),
            Self::InvalidClusterAdvance {
                paragraph_id,
                cluster_index,
            } => write!(
                f,
                "paragraph {paragraph_id} has a negative advance at cluster {cluster_index}"
            ),
            Self::InvalidAnchorOffset {
                object_id,
                offset,
                paragraph_len,
            } => write!(
                f,
                "object {object_id} anchor offset {offset} exceeds paragraph length {paragraph_len}"
            ),
            Self::InvalidObjectSize(id) => write!(f, "object {id} has an invalid size"),
            Self::InvalidExclusionMargin(id) => {
                write!(f, "object {id} has a negative exclusion margin")
            }
            Self::LayoutDidNotAdvance(id) => {
                write!(f, "paragraph {id} exceeded the line-layout safety limit")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Computes LastDraft's complete flow layout. All choices that could otherwise
/// depend on floating-point behavior or platform iteration order are explicitly
/// sorted and evaluated with fixed-point arithmetic.
pub fn layout(request: &LayoutRequest) -> Result<FlowLayout, LayoutError> {
    validate(request)?;

    let mut objects_by_paragraph: HashMap<&str, Vec<&FlowObject>> = HashMap::new();
    for object in &request.objects {
        objects_by_paragraph
            .entry(object.anchor.paragraph_id.as_str())
            .or_default()
            .push(object);
    }
    for objects in objects_by_paragraph.values_mut() {
        objects.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let mut resolved_objects = Vec::with_capacity(request.objects.len());
    let mut paragraph_layouts = Vec::with_capacity(request.paragraphs.len());
    let mut cursor_y = request.content.y;
    let mut maximum_bottom = request.content.y;

    for paragraph in &request.paragraphs {
        let paragraph_top = cursor_y;

        if let Some(objects) = objects_by_paragraph.get(paragraph.id.as_str()) {
            for object in objects {
                let anchor_line = reference_line_for_anchor(
                    paragraph,
                    request.content.width,
                    object.anchor.utf16_offset,
                    object.anchor.affinity,
                );
                let resolved = resolve_object(
                    object,
                    request.content,
                    paragraph_top,
                    anchor_line,
                    paragraph.style.line_height,
                );
                maximum_bottom = maximum_bottom.max(resolved.frame.bottom());
                resolved_objects.push(resolved);
            }
        }

        let paragraph_layout =
            flow_paragraph(paragraph, paragraph_top, request.content, &resolved_objects)?;
        cursor_y = paragraph_layout.bottom + paragraph.style.space_after;
        maximum_bottom = maximum_bottom.max(paragraph_layout.bottom);
        paragraph_layouts.push(paragraph_layout);
    }

    resolved_objects.sort_by(|left, right| left.id.cmp(&right.id));
    let content_bottom = maximum_bottom.max(cursor_y);

    Ok(FlowLayout {
        paragraphs: paragraph_layouts,
        objects: resolved_objects,
        content_height: content_bottom - request.content.y,
    })
}

/// Converts a pointer-driven object position into LastDraft's portable local
/// placement. Clients persist the returned value with the semantic anchor; they
/// must not persist `proposed_x`, `proposed_y`, or the resolved object frame.
pub fn normalized_placement_for_drag(
    content: Rect,
    object_width: LayoutUnit,
    anchor_reference_top: LayoutUnit,
    line_height: LayoutUnit,
    proposed_x: LayoutUnit,
    proposed_y: LayoutUnit,
) -> crate::NormalizedPlacement {
    let travel = (content.width - object_width).max(LayoutUnit::ZERO);
    let inline_position = if travel == LayoutUnit::ZERO {
        crate::Normalized::CENTER
    } else {
        let local_x = (proposed_x - content.x).clamp(LayoutUnit::ZERO, travel);
        let numerator = local_x.raw() as i64 * u16::MAX as i64 + travel.raw() as i64 / 2;
        crate::Normalized::from_raw((numerator / travel.raw() as i64) as u16)
    };

    let block_delta = proposed_y - anchor_reference_top;
    let block_offset = if line_height <= LayoutUnit::ZERO {
        0
    } else {
        let product = block_delta.raw() as i64 * crate::BLOCK_OFFSET_SCALE as i64;
        let half = line_height.raw() as i64 / 2;
        let rounded = if product >= 0 {
            product + half
        } else {
            product - half
        };
        (rounded / line_height.raw() as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    };

    crate::NormalizedPlacement {
        inline_position,
        block_offset,
    }
}

fn validate(request: &LayoutRequest) -> Result<(), LayoutError> {
    if request.content.width <= LayoutUnit::ZERO {
        return Err(LayoutError::InvalidContentWidth);
    }

    let mut paragraph_ids: HashSet<&str> = HashSet::with_capacity(request.paragraphs.len());
    for paragraph in &request.paragraphs {
        if paragraph.id.is_empty() || !paragraph_ids.insert(paragraph.id.as_str()) {
            return Err(LayoutError::DuplicateParagraphId(paragraph.id.clone()));
        }
        if paragraph.style.line_height <= LayoutUnit::ZERO
            || paragraph.style.ascent < LayoutUnit::ZERO
            || paragraph.style.ascent > paragraph.style.line_height
            || paragraph.style.space_after < LayoutUnit::ZERO
        {
            return Err(LayoutError::InvalidParagraphMetrics(paragraph.id.clone()));
        }

        let mut previous_end = 0;
        for (index, cluster) in paragraph.clusters.iter().enumerate() {
            if cluster.utf16_start != previous_end
                || cluster.utf16_end <= cluster.utf16_start
                || cluster.utf16_end > paragraph.utf16_len
            {
                return Err(LayoutError::InvalidClusterRange {
                    paragraph_id: paragraph.id.clone(),
                    cluster_index: index,
                });
            }
            if cluster.advance < LayoutUnit::ZERO {
                return Err(LayoutError::InvalidClusterAdvance {
                    paragraph_id: paragraph.id.clone(),
                    cluster_index: index,
                });
            }
            previous_end = cluster.utf16_end;
        }
        if !paragraph.clusters.is_empty() && previous_end != paragraph.utf16_len {
            return Err(LayoutError::InvalidClusterRange {
                paragraph_id: paragraph.id.clone(),
                cluster_index: paragraph.clusters.len() - 1,
            });
        }
        if paragraph.clusters.is_empty() && paragraph.utf16_len != 0 {
            return Err(LayoutError::InvalidClusterRange {
                paragraph_id: paragraph.id.clone(),
                cluster_index: 0,
            });
        }
    }

    let paragraph_lengths: HashMap<&str, u32> = request
        .paragraphs
        .iter()
        .map(|paragraph| (paragraph.id.as_str(), paragraph.utf16_len))
        .collect();
    let mut object_ids: HashSet<&str> = HashSet::with_capacity(request.objects.len());
    for object in &request.objects {
        if object.id.is_empty() || !object_ids.insert(object.id.as_str()) {
            return Err(LayoutError::DuplicateObjectId(object.id.clone()));
        }
        let Some(paragraph_len) = paragraph_lengths.get(object.anchor.paragraph_id.as_str()) else {
            return Err(LayoutError::MissingAnchorParagraph {
                object_id: object.id.clone(),
                paragraph_id: object.anchor.paragraph_id.clone(),
            });
        };
        if object.anchor.utf16_offset > *paragraph_len {
            return Err(LayoutError::InvalidAnchorOffset {
                object_id: object.id.clone(),
                offset: object.anchor.utf16_offset,
                paragraph_len: *paragraph_len,
            });
        }
        if object.size.ideal_width <= LayoutUnit::ZERO
            || object.size.ideal_height <= LayoutUnit::ZERO
            || object.size.max_inline_fraction.raw() == 0
        {
            return Err(LayoutError::InvalidObjectSize(object.id.clone()));
        }
        let margin = object.exclusion.margin;
        if margin.start < LayoutUnit::ZERO
            || margin.top < LayoutUnit::ZERO
            || margin.end < LayoutUnit::ZERO
            || margin.bottom < LayoutUnit::ZERO
            || object.exclusion.minimum_fragment_width < LayoutUnit::ZERO
        {
            return Err(LayoutError::InvalidExclusionMargin(object.id.clone()));
        }
    }
    Ok(())
}

fn resolve_object(
    object: &FlowObject,
    content: Rect,
    paragraph_top: LayoutUnit,
    anchor_line: usize,
    line_height: LayoutUnit,
) -> ResolvedObject {
    let maximum_width = object
        .size
        .max_inline_fraction
        .of(content.width)
        .min(content.width);
    let width = object.size.ideal_width.min(maximum_width);
    let height = scale_ratio(
        object.size.ideal_height,
        width.raw(),
        object.size.ideal_width.raw(),
    );
    let horizontal_travel = content.width - width;
    let user_x = content.x + object.placement.inline_position.of(horizontal_travel);
    let anchor_top = paragraph_top
        + LayoutUnit::from_raw(
            (anchor_line as i64 * line_height.raw() as i64).clamp(i32::MIN as i64, i32::MAX as i64)
                as i32,
        );
    let y = anchor_top + scale_block_offset(line_height, object.placement.block_offset);
    let mut frame = Rect::new(user_x, y, width, height);
    let mut exclusion_frame = clipped_exclusion(frame, object, content);

    let start_corridor = exclusion_frame.x - content.x;
    let end_corridor = content.right() - exclusion_frame.right();
    let largest_corridor = start_corridor.max(end_corridor);
    let cannot_support_side_flow = largest_corridor <= LayoutUnit::ZERO
        || largest_corridor < object.exclusion.minimum_fragment_width;

    let mode = if cannot_support_side_flow {
        frame.x = content.x + LayoutUnit::from_raw(horizontal_travel.raw() / 2);
        let vertical_exclusion = frame.expanded(object.exclusion.margin);
        exclusion_frame = Rect::new(
            content.x,
            vertical_exclusion.y,
            content.width,
            vertical_exclusion.height,
        );
        ObjectLayoutMode::BlockFallback
    } else {
        ObjectLayoutMode::UserPositioned
    };

    ResolvedObject {
        id: object.id.clone(),
        anchor: object.anchor.clone(),
        anchor_reference_top: anchor_top,
        frame,
        exclusion_frame,
        mode,
        minimum_fragment_width: object.exclusion.minimum_fragment_width,
    }
}

fn clipped_exclusion(frame: Rect, object: &FlowObject, content: Rect) -> Rect {
    let expanded = frame.expanded(object.exclusion.margin);
    let start = expanded.x.max(content.x);
    let end = expanded.right().min(content.right());
    Rect::new(
        start,
        expanded.y,
        (end - start).max(LayoutUnit::ZERO),
        expanded.height,
    )
}

fn flow_paragraph(
    paragraph: &Paragraph,
    paragraph_top: LayoutUnit,
    content: Rect,
    objects: &[ResolvedObject],
) -> Result<ParagraphLayout, LayoutError> {
    let mut lines = Vec::new();
    let mut cluster_index = 0;
    let mut line_top = paragraph_top;

    // Empty paragraphs retain one line so their semantic anchor has geometry.
    if paragraph.clusters.is_empty() {
        lines.push(FlowLine {
            top: line_top,
            baseline: line_top + paragraph.style.ascent,
            fragments: Vec::new(),
        });
        return Ok(ParagraphLayout {
            paragraph_id: paragraph.id.clone(),
            top: paragraph_top,
            bottom: line_top + paragraph.style.line_height,
            lines,
        });
    }

    while cluster_index < paragraph.clusters.len() {
        if lines.len() >= MAX_LINES_PER_PARAGRAPH {
            return Err(LayoutError::LayoutDidNotAdvance(paragraph.id.clone()));
        }

        let line_bottom = line_top + paragraph.style.line_height;
        let (available, minimum_fragment_width) =
            available_intervals(content, line_top, line_bottom, objects);
        let available: Vec<Interval> = available
            .into_iter()
            .filter(|interval| interval.width() >= minimum_fragment_width)
            .collect();
        let mut fragments = Vec::new();
        let line_start_index = cluster_index;

        for (interval_index, interval) in available.iter().copied().enumerate() {
            if cluster_index >= paragraph.clusters.len() {
                break;
            }
            let widest_remaining = available[interval_index..]
                .iter()
                .map(|candidate| candidate.width())
                .max()
                .unwrap_or(LayoutUnit::ZERO);
            if let Some(fit) = fit_fragment(
                &paragraph.clusters,
                cluster_index,
                interval.width(),
                widest_remaining,
                content.width,
            ) {
                let start = paragraph.clusters[cluster_index].utf16_start;
                let end = paragraph.clusters[fit.end_index - 1].utf16_end;
                fragments.push(FlowFragment {
                    rect: Rect::new(
                        interval.start,
                        line_top,
                        fit.used_width,
                        paragraph.style.line_height,
                    ),
                    utf16_start: start,
                    utf16_end: end,
                });
                cluster_index = fit.end_index;
            }
        }

        lines.push(FlowLine {
            top: line_top,
            baseline: line_top + paragraph.style.ascent,
            fragments,
        });
        line_top += paragraph.style.line_height;

        // It is valid not to advance while a full-width fallback exclusion is
        // active. The line safety limit handles malformed unbounded geometry.
        if cluster_index == line_start_index {
            continue;
        }
    }

    Ok(ParagraphLayout {
        paragraph_id: paragraph.id.clone(),
        top: paragraph_top,
        bottom: line_top,
        lines,
    })
}

fn available_intervals(
    content: Rect,
    line_top: LayoutUnit,
    line_bottom: LayoutUnit,
    objects: &[ResolvedObject],
) -> (Vec<Interval>, LayoutUnit) {
    let mut blocked = Vec::new();
    let mut minimum_fragment_width = LayoutUnit::ZERO;
    for object in objects {
        if object
            .exclusion_frame
            .intersects_vertical_band(line_top, line_bottom)
        {
            blocked.push(Interval {
                start: object.exclusion_frame.x,
                end: object.exclusion_frame.right(),
            });
            minimum_fragment_width = minimum_fragment_width.max(object.minimum_fragment_width);
        }
    }
    let available = subtract_intervals(
        Interval {
            start: content.x,
            end: content.right(),
        },
        &mut blocked,
    );
    (available, minimum_fragment_width)
}

#[derive(Clone, Copy, Debug)]
struct FragmentFit {
    end_index: usize,
    used_width: LayoutUnit,
}

fn fit_fragment(
    clusters: &[Cluster],
    start_index: usize,
    width: LayoutUnit,
    widest_remaining: LayoutUnit,
    full_width: LayoutUnit,
) -> Option<FragmentFit> {
    if width <= LayoutUnit::ZERO || start_index >= clusters.len() {
        return None;
    }

    let mut used = LayoutUnit::ZERO;
    let mut index = start_index;
    let mut last_break: Option<(usize, LayoutUnit)> = None;
    while index < clusters.len() {
        let next = used + clusters[index].advance;
        if next > width {
            break;
        }
        used = next;
        index += 1;
        if clusters[index - 1].can_break_after {
            last_break = Some((index, used));
        }
    }

    if index == clusters.len() {
        return Some(FragmentFit {
            end_index: index,
            used_width: used,
        });
    }
    if let Some((end_index, break_width)) = last_break {
        return Some(FragmentFit {
            end_index,
            used_width: break_width,
        });
    }

    // Avoid breaking a word merely because the object left a narrow corridor.
    // Prefer a wider fragment on this line, or wait until a later line clears
    // the exclusion. Only words wider than the complete content width receive
    // an emergency grapheme break.
    let unbreakable_width = width_until_break(clusters, start_index);
    if unbreakable_width <= widest_remaining || unbreakable_width <= full_width {
        return None;
    }

    if index == start_index {
        index += 1;
        used = clusters[start_index].advance;
    }
    Some(FragmentFit {
        end_index: index,
        used_width: used,
    })
}

fn width_until_break(clusters: &[Cluster], start_index: usize) -> LayoutUnit {
    let mut width = LayoutUnit::ZERO;
    for cluster in &clusters[start_index..] {
        width += cluster.advance;
        if cluster.can_break_after {
            break;
        }
    }
    width
}

fn reference_line_for_anchor(
    paragraph: &Paragraph,
    width: LayoutUnit,
    utf16_offset: u32,
    affinity: AnchorAffinity,
) -> usize {
    if paragraph.clusters.is_empty() {
        return 0;
    }

    let ranges = reference_line_ranges(&paragraph.clusters, width);
    for (line_index, (_, end_index)) in ranges.iter().copied().enumerate() {
        let end_offset = paragraph.clusters[end_index - 1].utf16_end;
        if utf16_offset < end_offset {
            return line_index;
        }
        if utf16_offset == end_offset && affinity == AnchorAffinity::Upstream {
            return line_index;
        }
    }
    ranges.len().saturating_sub(1)
}

fn reference_line_ranges(clusters: &[Cluster], width: LayoutUnit) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < clusters.len() {
        let start = index;
        let fit = fit_fragment(clusters, index, width, width, width).unwrap_or(FragmentFit {
            end_index: index + 1,
            used_width: clusters[index].advance,
        });
        index = fit.end_index;
        ranges.push((start, index));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_subtraction_is_order_independent() {
        let content = Rect::new(
            LayoutUnit::ZERO,
            LayoutUnit::ZERO,
            LayoutUnit::from_raw(100),
            LayoutUnit::from_raw(100),
        );
        let objects = vec![
            ResolvedObject {
                id: "2".into(),
                anchor: Default::default(),
                anchor_reference_top: LayoutUnit::ZERO,
                frame: Rect::default(),
                exclusion_frame: Rect::new(
                    LayoutUnit::from_raw(50),
                    LayoutUnit::ZERO,
                    LayoutUnit::from_raw(20),
                    LayoutUnit::from_raw(20),
                ),
                mode: ObjectLayoutMode::UserPositioned,
                minimum_fragment_width: LayoutUnit::ZERO,
            },
            ResolvedObject {
                id: "1".into(),
                anchor: Default::default(),
                anchor_reference_top: LayoutUnit::ZERO,
                frame: Rect::default(),
                exclusion_frame: Rect::new(
                    LayoutUnit::from_raw(20),
                    LayoutUnit::ZERO,
                    LayoutUnit::from_raw(40),
                    LayoutUnit::from_raw(20),
                ),
                mode: ObjectLayoutMode::UserPositioned,
                minimum_fragment_width: LayoutUnit::ZERO,
            },
        ];
        let (available, _) = available_intervals(
            content,
            LayoutUnit::ZERO,
            LayoutUnit::from_raw(10),
            &objects,
        );
        assert_eq!(
            available,
            vec![
                Interval {
                    start: LayoutUnit::ZERO,
                    end: LayoutUnit::from_raw(20),
                },
                Interval {
                    start: LayoutUnit::from_raw(70),
                    end: LayoutUnit::from_raw(100),
                }
            ]
        );
    }
}
