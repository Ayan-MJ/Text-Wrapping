use core::fmt;
use std::collections::{HashMap, HashSet};

use crate::fixed::{scale_block_offset, scale_ratio};
use crate::geometry::subtract_intervals;
use crate::{
    AnchorAffinity, Cluster, FlowFragment, FlowLayout, FlowLine, FlowObject, Interval,
    Insets, LayoutRequest, LayoutUnit, ObjectFlow, ObjectLayoutMode, Paragraph, ParagraphLayout, Rect,
    ResolvedObject, TextAlignment, TextDirection, WritingMode,
};

const MAX_LINES_PER_PARAGRAPH: usize = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidContentWidth,
    InvalidContentHeight,
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
            Self::InvalidContentHeight => {
                write!(f, "content height must be positive for vertical writing")
            }
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
    // How far we have already advanced along the BLOCK axis. Which physical
    // axis that is depends on the writing mode: down in horizontal writing,
    // left from the right edge in vertical-rl (contract §7). Offsetting `y`
    // unconditionally is what used to stack Japanese paragraphs down the page
    // instead of beside one another.
    let mut block_cursor = LayoutUnit::ZERO;
    let mut maximum_bottom = request.content.y;
    let mut maximum_right = request.content.x;
    let mut minimum_left = request.content.x;

    for paragraph in &request.paragraphs {
        let paragraph_region = block_region(request.content, block_cursor, paragraph.writing_mode);

        if let Some(objects) = objects_by_paragraph.get(paragraph.id.as_str()) {
            for object in objects {
                let anchor_line = reference_line_for_anchor(
                    paragraph,
                    inline_extent(paragraph_region, paragraph.writing_mode),
                    object.anchor.utf16_offset,
                    object.anchor.affinity,
                );
                let resolved = resolve_object(
                    object,
                    paragraph_region,
                    anchor_line,
                    paragraph.style.line_height,
                    paragraph.writing_mode,
                    paragraph.base_direction,
                );
                maximum_bottom = maximum_bottom.max(resolved.frame.bottom());
                maximum_right = maximum_right.max(resolved.frame.right());
                minimum_left = minimum_left.min(resolved.frame.x);
                resolved_objects.push(resolved);
            }
        }

        let paragraph_layout = flow_paragraph(paragraph, paragraph_region, &resolved_objects)?;
        block_cursor = block_cursor
            + block_extent(paragraph.style.line_height, paragraph_layout.lines.len())
            + paragraph.style.space_after;
        maximum_bottom = maximum_bottom.max(paragraph_layout.bottom);
        maximum_right = maximum_right.max(paragraph_layout.bounds.right());
        minimum_left = minimum_left.min(paragraph_layout.bounds.x);
        paragraph_layouts.push(paragraph_layout);
    }

    resolved_objects.sort_by(|left, right| left.id.cmp(&right.id));

    // The block cursor measures the axis the letter grows along, so it is the
    // one that carries a trailing `space_after`. In horizontal writing that is
    // the height, and this reduces to exactly the previous formula.
    let vertical = request
        .paragraphs
        .iter()
        .any(|paragraph| paragraph.writing_mode.is_vertical());
    let (content_width, content_height) = if vertical {
        (
            (maximum_right - minimum_left).max(block_cursor),
            maximum_bottom - request.content.y,
        )
    } else {
        (
            maximum_right - minimum_left,
            maximum_bottom.max(request.content.y + block_cursor) - request.content.y,
        )
    };

    Ok(FlowLayout {
        paragraphs: paragraph_layouts,
        objects: resolved_objects,
        content_height,
        content_width,
    })
}

/// The content box a paragraph occupies once `advanced` block units are spent.
///
/// Every downstream reader (`line_geometry`, `paragraph_bounds`,
/// `resolve_object`) already derives its block start from the correct edge of
/// this rect per writing mode, so placing the rect correctly is the whole fix.
fn block_region(content: Rect, advanced: LayoutUnit, writing_mode: WritingMode) -> Rect {
    match writing_mode {
        WritingMode::HorizontalTb => Rect::new(
            content.x,
            content.y + advanced,
            content.width,
            content.height,
        ),
        // Columns march rightward from the left edge.
        WritingMode::VerticalLr => Rect::new(
            content.x + advanced,
            content.y,
            content.width - advanced,
            content.height,
        ),
        // Columns march leftward from the right edge, so shrinking the width
        // moves `content.right()` — the origin every vertical-rl reader uses.
        WritingMode::VerticalRl => Rect::new(
            content.x,
            content.y,
            content.width - advanced,
            content.height,
        ),
    }
}

/// How far `line_count` lines advance along the block axis.
fn block_extent(line_height: LayoutUnit, line_count: usize) -> LayoutUnit {
    LayoutUnit::from_raw(
        (line_count as i64 * line_height.raw() as i64).clamp(0, i32::MAX as i64) as i32,
    )
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
    normalized_placement_for_drag_in_mode(
        content,
        object_width,
        LayoutUnit::ZERO,
        anchor_reference_top,
        line_height,
        proposed_x,
        proposed_y,
        WritingMode::HorizontalTb,
    )
}

/// Axis-aware ABI-v4 drag conversion. The anchor reference is a physical
/// coordinate on the block axis; positive block offsets always follow the
/// writing mode's line-progression direction.
#[allow(clippy::too_many_arguments)]
pub fn normalized_placement_for_drag_in_mode(
    content: Rect,
    object_width: LayoutUnit,
    object_height: LayoutUnit,
    anchor_reference_block_start: LayoutUnit,
    line_height: LayoutUnit,
    proposed_x: LayoutUnit,
    proposed_y: LayoutUnit,
    writing_mode: WritingMode,
) -> crate::NormalizedPlacement {
    let (travel, local_inline, block_delta) = match writing_mode {
        WritingMode::HorizontalTb => (
            (content.width - object_width).max(LayoutUnit::ZERO),
            proposed_x - content.x,
            proposed_y - anchor_reference_block_start,
        ),
        WritingMode::VerticalLr => (
            (content.height - object_height).max(LayoutUnit::ZERO),
            proposed_y - content.y,
            proposed_x - anchor_reference_block_start,
        ),
        WritingMode::VerticalRl => (
            (content.height - object_height).max(LayoutUnit::ZERO),
            proposed_y - content.y,
            anchor_reference_block_start - (proposed_x + object_width),
        ),
    };
    let inline_position = if travel == LayoutUnit::ZERO {
        crate::Normalized::CENTER
    } else {
        let local_inline = local_inline.clamp(LayoutUnit::ZERO, travel);
        let numerator = local_inline.raw() as i64 * u16::MAX as i64 + travel.raw() as i64 / 2;
        crate::Normalized::from_raw((numerator / travel.raw() as i64) as u16)
    };

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
        if paragraph.writing_mode.is_vertical() && request.content.height <= LayoutUnit::ZERO {
            return Err(LayoutError::InvalidContentHeight);
        }
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
        if margin.inline_start < LayoutUnit::ZERO
            || margin.block_start < LayoutUnit::ZERO
            || margin.inline_end < LayoutUnit::ZERO
            || margin.block_end < LayoutUnit::ZERO
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
    anchor_line: usize,
    line_height: LayoutUnit,
    writing_mode: WritingMode,
    base_direction: TextDirection,
) -> ResolvedObject {
    // The document stores margins logically; which physical side each lands on
    // depends on the paragraph's RESOLVED direction, which only exists here.
    let margin = object.exclusion.margin.resolve(writing_mode, base_direction);
    let (width, height) = match writing_mode {
        WritingMode::HorizontalTb => {
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
            (width, height)
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let maximum_height = object
                .size
                .max_inline_fraction
                .of(content.height)
                .min(content.height);
            let height = object.size.ideal_height.min(maximum_height);
            let width = scale_ratio(
                object.size.ideal_width,
                height.raw(),
                object.size.ideal_height.raw(),
            );
            (width, height)
        }
    };
    let line_delta = LayoutUnit::from_raw(
        (anchor_line as i64 * line_height.raw() as i64).clamp(i32::MIN as i64, i32::MAX as i64)
            as i32,
    );
    let block_delta = scale_block_offset(line_height, object.placement.block_offset);
    let (anchor_reference, mut frame) = match writing_mode {
        WritingMode::HorizontalTb => {
            let travel = content.width - width;
            let x = content.x + object.placement.inline_position.of(travel);
            let reference = content.y + line_delta;
            (
                reference,
                Rect::new(x, reference + block_delta, width, height),
            )
        }
        WritingMode::VerticalLr => {
            let travel = content.height - height;
            let y = content.y + object.placement.inline_position.of(travel);
            let reference = content.x + line_delta;
            (
                reference,
                Rect::new(reference + block_delta, y, width, height),
            )
        }
        WritingMode::VerticalRl => {
            let travel = content.height - height;
            let y = content.y + object.placement.inline_position.of(travel);
            let reference = content.right() - line_delta;
            (
                reference,
                Rect::new(reference - width - block_delta, y, width, height),
            )
        }
    };
    // A FLOATING object stops here. It excludes nothing, so there are no
    // fragments to test and no reason to fall back to a centred block. Its
    // exclusion frame is the empty rect at its own origin: `blocked_intervals`
    // already skips a band with no area in both writing modes, so a floating
    // object cannot reach the line breaker at all.
    if object.flow == ObjectFlow::Float {
        return ResolvedObject {
            id: object.id.clone(),
            anchor: object.anchor.clone(),
            anchor_reference_top: anchor_reference,
            anchor_reference_block_start: anchor_reference,
            frame,
            exclusion_frame: Rect::new(frame.x, frame.y, LayoutUnit::ZERO, LayoutUnit::ZERO),
            mode: ObjectLayoutMode::Floating,
            minimum_fragment_width: LayoutUnit::ZERO,
        };
    }

    let mut exclusion_frame = clipped_exclusion(frame, margin, content, writing_mode);

    let (start_corridor, end_corridor) = match writing_mode {
        WritingMode::HorizontalTb => (
            exclusion_frame.x - content.x,
            content.right() - exclusion_frame.right(),
        ),
        WritingMode::VerticalRl | WritingMode::VerticalLr => (
            exclusion_frame.y - content.y,
            content.bottom() - exclusion_frame.bottom(),
        ),
    };
    let largest_corridor = start_corridor.max(end_corridor);
    let cannot_support_side_flow = largest_corridor <= LayoutUnit::ZERO
        || largest_corridor < object.exclusion.minimum_fragment_width;

    let mode = if cannot_support_side_flow {
        match writing_mode {
            WritingMode::HorizontalTb => {
                let travel = content.width - width;
                frame.x = content.x + LayoutUnit::from_raw(travel.raw() / 2);
                let expanded = frame.expanded(margin);
                exclusion_frame = Rect::new(content.x, expanded.y, content.width, expanded.height);
            }
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                let travel = content.height - height;
                frame.y = content.y + LayoutUnit::from_raw(travel.raw() / 2);
                let expanded = frame.expanded(margin);
                exclusion_frame = Rect::new(expanded.x, content.y, expanded.width, content.height);
            }
        }
        ObjectLayoutMode::BlockFallback
    } else {
        ObjectLayoutMode::UserPositioned
    };

    ResolvedObject {
        id: object.id.clone(),
        anchor: object.anchor.clone(),
        anchor_reference_top: anchor_reference,
        anchor_reference_block_start: anchor_reference,
        frame,
        exclusion_frame,
        mode,
        minimum_fragment_width: object.exclusion.minimum_fragment_width,
    }
}

fn clipped_exclusion(
    frame: Rect,
    margin: Insets,
    content: Rect,
    writing_mode: WritingMode,
) -> Rect {
    let expanded = frame.expanded(margin);
    match writing_mode {
        WritingMode::HorizontalTb => {
            let start = expanded.x.max(content.x);
            let end = expanded.right().min(content.right());
            Rect::new(
                start,
                expanded.y,
                (end - start).max(LayoutUnit::ZERO),
                expanded.height,
            )
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let start = expanded.y.max(content.y);
            let end = expanded.bottom().min(content.bottom());
            Rect::new(
                expanded.x,
                start,
                expanded.width,
                (end - start).max(LayoutUnit::ZERO),
            )
        }
    }
}

fn flow_paragraph(
    paragraph: &Paragraph,
    content: Rect,
    objects: &[ResolvedObject],
) -> Result<ParagraphLayout, LayoutError> {
    let mut lines = Vec::new();
    let mut cluster_index = 0;
    let mut line_index = 0_usize;

    // Empty paragraphs retain one line so their semantic anchor has geometry.
    if paragraph.clusters.is_empty() {
        let geometry = line_geometry(paragraph, content, 0);
        lines.push(FlowLine {
            top: geometry.top,
            baseline: geometry.baseline,
            writing_mode: paragraph.writing_mode,
            block_start: geometry.block_start,
            inline_start: geometry.inline_start,
            fragments: Vec::new(),
        });
        let bounds = paragraph_bounds(paragraph, content, 1);
        return Ok(ParagraphLayout {
            paragraph_id: paragraph.id.clone(),
            top: bounds.y,
            bottom: bounds.bottom(),
            bounds,
            base_direction: paragraph.base_direction,
            writing_mode: paragraph.writing_mode,
            lines,
        });
    }

    while cluster_index < paragraph.clusters.len() {
        if lines.len() >= MAX_LINES_PER_PARAGRAPH {
            return Err(LayoutError::LayoutDidNotAdvance(paragraph.id.clone()));
        }

        let geometry = line_geometry(paragraph, content, line_index);
        let (mut available, minimum_fragment_width) = available_intervals(
            content,
            geometry.band_start,
            geometry.band_end,
            paragraph.writing_mode,
            paragraph.base_direction,
            paragraph.style.indent,
            objects,
        );
        if paragraph.base_direction == TextDirection::RightToLeft {
            available.reverse();
        }
        let available: Vec<Interval> = available
            .into_iter()
            .filter(|interval| interval.width() >= minimum_fragment_width)
            .collect();
        let mut fragments = Vec::new();
        let mut fragment_intervals: Vec<LayoutUnit> = Vec::new();
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
                inline_extent(content, paragraph.writing_mode),
            ) {
                let start = paragraph.clusters[cluster_index].utf16_start;
                let end = paragraph.clusters[fit.end_index - 1].utf16_end;
                let inline_start = aligned_inline_start(
                    interval,
                    fit.used_width,
                    paragraph.style.alignment,
                    paragraph.base_direction,
                );
                let fragment_rect = match paragraph.writing_mode {
                    WritingMode::HorizontalTb => Rect::new(
                        inline_start,
                        geometry.block_start,
                        fit.used_width,
                        paragraph.style.line_height,
                    ),
                    WritingMode::VerticalRl | WritingMode::VerticalLr => Rect::new(
                        geometry.block_start,
                        inline_start,
                        paragraph.style.line_height,
                        fit.used_width,
                    ),
                };
                fragments.push(FlowFragment {
                    rect: fragment_rect,
                    utf16_start: start,
                    utf16_end: end,
                });
                fragment_intervals.push(interval.width());
                cluster_index = fit.end_index;
            }
        }

        // Justify only once the whole line is known, because the LAST line of a
        // paragraph is never stretched -- that is universal typographic
        // behaviour, and stretching it is the classic sign of a broken
        // justifier. A line is the last one when it consumed every remaining
        // cluster.
        if paragraph.style.alignment == TextAlignment::Justify
            && cluster_index < paragraph.clusters.len()
        {
            justify_line(&mut fragments, &fragment_intervals, paragraph.writing_mode);
        }

        lines.push(FlowLine {
            top: geometry.top,
            baseline: geometry.baseline,
            writing_mode: paragraph.writing_mode,
            block_start: geometry.block_start,
            inline_start: geometry.inline_start,
            fragments,
        });
        line_index += 1;

        // It is valid not to advance while a full-width fallback exclusion is
        // active. The line safety limit handles malformed unbounded geometry.
        if cluster_index == line_start_index {
            continue;
        }
    }

    let bounds = paragraph_bounds(paragraph, content, line_index);
    Ok(ParagraphLayout {
        paragraph_id: paragraph.id.clone(),
        top: bounds.y,
        bottom: bounds.bottom(),
        bounds,
        base_direction: paragraph.base_direction,
        writing_mode: paragraph.writing_mode,
        lines,
    })
}

/// Where a fragment starts inside the space available to it.
///
/// Alignment is LOGICAL, so it resolves against the paragraph's direction:
/// `Start` is the near edge of the interval in ltr and the far edge in rtl, and
/// the same rule carries into vertical writing, where the inline axis runs down
/// the column.
///
/// `Justify` fills the interval, so its fragment starts at the near edge in
/// both directions. The slack is spread between the words when the clusters are
/// placed; see `justify_line` below for which lines get it.
fn aligned_inline_start(
    interval: Interval,
    used_width: LayoutUnit,
    alignment: TextAlignment,
    base_direction: TextDirection,
) -> LayoutUnit {
    let slack = (interval.width() - used_width).max(LayoutUnit::ZERO);
    let rtl = base_direction == TextDirection::RightToLeft;
    match alignment {
        TextAlignment::Center => interval.start + LayoutUnit::from_raw(slack.raw() / 2),
        TextAlignment::Justify => interval.start,
        TextAlignment::Start => {
            if rtl {
                interval.end - used_width
            } else {
                interval.start
            }
        }
        TextAlignment::End => {
            if rtl {
                interval.start
            } else {
                interval.end - used_width
            }
        }
    }
}

/// Stretch each fragment on a justified line to fill the space it was given.
///
/// Only the fragment's extent changes here. WHERE the slack goes between the
/// words is decided when the clusters are placed, and it is derived from this
/// extent rather than carried on the wire: the slack is the fragment's extent
/// minus the sum of its cluster advances, which is exactly zero on every line
/// that is not justified. That is why justification needed no ABI change.
fn justify_line(
    fragments: &mut [FlowFragment],
    intervals: &[LayoutUnit],
    writing_mode: WritingMode,
) {
    for (fragment, available) in fragments.iter_mut().zip(intervals.iter().copied()) {
        match writing_mode {
            WritingMode::HorizontalTb => {
                if available > fragment.rect.width {
                    fragment.rect.width = available;
                }
            }
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                if available > fragment.rect.height {
                    fragment.rect.height = available;
                }
            }
        }
    }
}

fn available_intervals(
    content: Rect,
    band_start: LayoutUnit,
    band_end: LayoutUnit,
    writing_mode: WritingMode,
    base_direction: TextDirection,
    indent: LayoutUnit,
    objects: &[ResolvedObject],
) -> (Vec<Interval>, LayoutUnit) {
    let mut blocked = Vec::new();
    let mut minimum_fragment_width = LayoutUnit::ZERO;
    for object in objects {
        let intersects = match writing_mode {
            WritingMode::HorizontalTb => object
                .exclusion_frame
                .intersects_vertical_band(band_start, band_end),
            WritingMode::VerticalRl | WritingMode::VerticalLr => {
                object.exclusion_frame.width > LayoutUnit::ZERO
                    && object.exclusion_frame.x < band_end
                    && object.exclusion_frame.right() > band_start
            }
        };
        if intersects {
            blocked.push(match writing_mode {
                WritingMode::HorizontalTb => Interval {
                    start: object.exclusion_frame.x,
                    end: object.exclusion_frame.right(),
                },
                WritingMode::VerticalRl | WritingMode::VerticalLr => Interval {
                    start: object.exclusion_frame.y,
                    end: object.exclusion_frame.bottom(),
                },
            });
            minimum_fragment_width = minimum_fragment_width.max(object.minimum_fragment_width);
        }
    }
    let mut container = match writing_mode {
        WritingMode::HorizontalTb => Interval {
            start: content.x,
            end: content.right(),
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => Interval {
            start: content.y,
            end: content.bottom(),
        },
    };
    // An indent eats into the INLINE START of the line, which is the near edge
    // in ltr and the far edge in rtl. In vertical writing the inline axis runs
    // down the column, so the same rule indents from the top.
    if indent > LayoutUnit::ZERO {
        let indent = indent.min(container.width());
        if base_direction == TextDirection::RightToLeft {
            container.end -= indent;
        } else {
            container.start += indent;
        }
    }
    let available = subtract_intervals(container, &mut blocked);
    (available, minimum_fragment_width)
}

#[derive(Clone, Copy)]
struct LineGeometry {
    top: LayoutUnit,
    baseline: LayoutUnit,
    block_start: LayoutUnit,
    inline_start: LayoutUnit,
    band_start: LayoutUnit,
    band_end: LayoutUnit,
}

fn line_geometry(paragraph: &Paragraph, content: Rect, line_index: usize) -> LineGeometry {
    let delta = LayoutUnit::from_raw(
        (line_index as i64 * paragraph.style.line_height.raw() as i64)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    );
    match paragraph.writing_mode {
        WritingMode::HorizontalTb => {
            let top = content.y + delta;
            LineGeometry {
                top,
                baseline: top + paragraph.style.ascent,
                block_start: top,
                inline_start: if paragraph.base_direction == TextDirection::RightToLeft {
                    content.right()
                } else {
                    content.x
                },
                band_start: top,
                band_end: top + paragraph.style.line_height,
            }
        }
        WritingMode::VerticalLr => {
            let x = content.x + delta;
            LineGeometry {
                top: content.y,
                baseline: x + paragraph.style.ascent,
                block_start: x,
                inline_start: if paragraph.base_direction == TextDirection::RightToLeft {
                    content.bottom()
                } else {
                    content.y
                },
                band_start: x,
                band_end: x + paragraph.style.line_height,
            }
        }
        WritingMode::VerticalRl => {
            let right = content.right() - delta;
            let x = right - paragraph.style.line_height;
            LineGeometry {
                top: content.y,
                baseline: x + paragraph.style.ascent,
                block_start: x,
                inline_start: if paragraph.base_direction == TextDirection::RightToLeft {
                    content.bottom()
                } else {
                    content.y
                },
                band_start: x,
                band_end: right,
            }
        }
    }
}

fn paragraph_bounds(paragraph: &Paragraph, content: Rect, line_count: usize) -> Rect {
    let extent = block_extent(paragraph.style.line_height, line_count);
    match paragraph.writing_mode {
        WritingMode::HorizontalTb => Rect::new(content.x, content.y, content.width, extent),
        WritingMode::VerticalLr => Rect::new(content.x, content.y, extent, content.height),
        WritingMode::VerticalRl => Rect::new(
            content.right() - extent,
            content.y,
            extent,
            content.height,
        ),
    }
}

fn inline_extent(content: Rect, writing_mode: WritingMode) -> LayoutUnit {
    match writing_mode {
        WritingMode::HorizontalTb => content.width,
        WritingMode::VerticalRl | WritingMode::VerticalLr => content.height,
    }
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
                anchor_reference_block_start: LayoutUnit::ZERO,
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
                anchor_reference_block_start: LayoutUnit::ZERO,
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
            WritingMode::HorizontalTb,
            TextDirection::LeftToRight,
            LayoutUnit::ZERO,
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
