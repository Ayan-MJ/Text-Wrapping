use lastdraft_flow::{
    ObjectFlow,
    layout, normalized_placement_for_drag, AnchorAffinity, Cluster, ExclusionRules, FlowObject,
    GlyphOrientation, LayoutRequest, LayoutUnit, LogicalInsets, Normalized, NormalizedPlacement,
    ObjectLayoutMode, ObjectSize, Paragraph, ParagraphDirection, ParagraphStyle, Rect, TextAnchor,
    TextDirection, TextOrientation, WritingMode, BLOCK_OFFSET_SCALE,
};

fn u(value: i32) -> LayoutUnit {
    LayoutUnit::from_raw(value)
}

fn paragraph(id: u64, cluster_count: u32) -> Paragraph {
    Paragraph {
        id: id.to_string(),
        utf16_len: cluster_count,
        style: ParagraphStyle {
            line_height: u(10),
            ascent: u(8),
            space_after: LayoutUnit::ZERO,
            ..Default::default()
        },
        requested_base_direction: ParagraphDirection::LeftToRight,
        base_direction: TextDirection::LeftToRight,
        writing_mode: WritingMode::HorizontalTb,
        text_orientation: TextOrientation::Mixed,
        clusters: (0..cluster_count)
            .map(|offset| Cluster {
                utf16_start: offset,
                utf16_end: offset + 1,
                advance: u(10),
                can_break_after: true,
                is_whitespace: false,
                bidi_level: 0,
                direction: TextDirection::LeftToRight,
                orientation: GlyphOrientation::Upright,
            })
            .collect(),
    }
}

fn image(
    id: u64,
    paragraph_id: u64,
    anchor_offset: u32,
    inline_position: Normalized,
    width: i32,
    height: i32,
    minimum_fragment_width: i32,
) -> FlowObject {
    FlowObject {
        id: id.to_string(),
        // Every existing test is a WRAPPED object, which is the default and
        // what these tests have always meant.
        flow: ObjectFlow::Wrap,
        anchor: TextAnchor {
            paragraph_id: paragraph_id.to_string(),
            utf16_offset: anchor_offset,
            affinity: AnchorAffinity::Downstream,
        },
        placement: NormalizedPlacement {
            inline_position,
            block_offset: 0,
        },
        size: ObjectSize {
            ideal_width: u(width),
            ideal_height: u(height),
            max_inline_fraction: Normalized::END,
        },
        exclusion: ExclusionRules {
            margin: LogicalInsets::default(),
            minimum_fragment_width: u(minimum_fragment_width),
        },
    }
}

fn request(width: i32, paragraphs: Vec<Paragraph>, objects: Vec<FlowObject>) -> LayoutRequest {
    LayoutRequest {
        content: Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, u(width), u(10_000)),
        paragraphs,
        objects,
    }
}

#[test]
fn text_uses_both_sides_of_a_user_positioned_image() {
    let result = layout(&request(
        100,
        vec![paragraph(1, 20)],
        vec![image(9, 1, 0, Normalized::CENTER, 40, 20, 10)],
    ))
    .unwrap();

    assert_eq!(result.objects[0].mode, ObjectLayoutMode::UserPositioned);
    assert_eq!(result.objects[0].frame.x, u(30));

    let first_line = &result.paragraphs[0].lines[0];
    assert_eq!(first_line.fragments.len(), 2);
    assert_eq!(first_line.fragments[0].rect.x, u(0));
    assert_eq!(first_line.fragments[0].rect.width, u(30));
    assert_eq!(first_line.fragments[0].utf16_start, 0);
    assert_eq!(first_line.fragments[0].utf16_end, 3);
    assert_eq!(first_line.fragments[1].rect.x, u(70));
    assert_eq!(first_line.fragments[1].rect.width, u(30));
    assert_eq!(first_line.fragments[1].utf16_start, 3);
    assert_eq!(first_line.fragments[1].utf16_end, 6);
}

#[test]
fn horizontal_drag_is_continuous_and_has_no_side_mode() {
    let start = layout(&request(
        100,
        vec![paragraph(1, 20)],
        vec![image(1, 1, 0, Normalized::START, 40, 10, 10)],
    ))
    .unwrap();
    let end = layout(&request(
        100,
        vec![paragraph(1, 20)],
        vec![image(1, 1, 0, Normalized::END, 40, 10, 10)],
    ))
    .unwrap();

    assert_eq!(start.objects[0].frame.x, u(0));
    assert_eq!(end.objects[0].frame.x, u(60));
    assert_eq!(start.objects[0].mode, ObjectLayoutMode::UserPositioned);
    assert_eq!(end.objects[0].mode, ObjectLayoutMode::UserPositioned);
}

#[test]
fn object_moves_with_its_semantic_paragraph_anchor_after_reflow() {
    let short = layout(&request(
        100,
        vec![paragraph(1, 10), paragraph(2, 10)],
        vec![image(1, 2, 0, Normalized::CENTER, 20, 10, 5)],
    ))
    .unwrap();
    let long = layout(&request(
        100,
        vec![paragraph(1, 30), paragraph(2, 10)],
        vec![image(1, 2, 0, Normalized::CENTER, 20, 10, 5)],
    ))
    .unwrap();

    assert_eq!(short.objects[0].frame.y, u(10));
    assert_eq!(long.objects[0].frame.y, u(30));
    assert_eq!(long.objects[0].frame.y - short.objects[0].frame.y, u(20));
}

#[test]
fn anchor_offset_and_vertical_drag_are_resolved_on_the_anchor_track() {
    let mut object = image(1, 1, 15, Normalized::CENTER, 20, 10, 5);
    object.placement.block_offset = -BLOCK_OFFSET_SCALE;
    let result = layout(&request(100, vec![paragraph(1, 30)], vec![object])).unwrap();

    // Offset 15 is on reference line 2 (top=10). A -1 line local drag
    // places the object at y=0 without storing a document-global y value.
    assert_eq!(result.objects[0].frame.y, u(0));
}

#[test]
fn narrow_view_uses_reversible_centered_block_fallback() {
    let object = image(1, 1, 0, Normalized::from_raw(52_428), 60, 20, 30);
    let wide = layout(&request(120, vec![paragraph(1, 20)], vec![object.clone()])).unwrap();
    let narrow = layout(&request(80, vec![paragraph(1, 20)], vec![object.clone()])).unwrap();
    let wide_again = layout(&request(120, vec![paragraph(1, 20)], vec![object])).unwrap();

    assert_eq!(wide.objects[0].mode, ObjectLayoutMode::UserPositioned);
    assert_eq!(wide.objects[0].frame.x, u(48));
    assert_eq!(narrow.objects[0].mode, ObjectLayoutMode::BlockFallback);
    assert_eq!(narrow.objects[0].frame.x, u(10));
    assert_eq!(narrow.objects[0].exclusion_frame.x, u(0));
    assert_eq!(narrow.objects[0].exclusion_frame.width, u(80));
    assert!(narrow.paragraphs[0].lines[0].fragments.is_empty());
    assert!(narrow.paragraphs[0].lines[1].fragments.is_empty());
    assert_eq!(narrow.paragraphs[0].lines[2].fragments[0].rect.x, u(0));
    assert_eq!(wide.objects[0].frame, wide_again.objects[0].frame);
}

#[test]
fn oversized_image_scales_proportionally_to_the_viewport() {
    let result = layout(&request(
        50,
        vec![paragraph(1, 10)],
        vec![image(1, 1, 0, Normalized::CENTER, 80, 40, 20)],
    ))
    .unwrap();

    assert_eq!(result.objects[0].frame.width, u(50));
    assert_eq!(result.objects[0].frame.height, u(25));
    assert_eq!(result.objects[0].frame.x, u(0));
    assert_eq!(result.objects[0].mode, ObjectLayoutMode::BlockFallback);
}

#[test]
fn repeated_layout_is_bit_for_bit_deterministic() {
    let input = request(
        137,
        vec![paragraph(1, 47), paragraph(2, 31)],
        vec![
            image(2, 1, 11, Normalized::from_raw(7_777), 53, 27, 12),
            image(1, 2, 9, Normalized::from_raw(49_999), 41, 33, 12),
        ],
    );

    assert_eq!(layout(&input).unwrap(), layout(&input).unwrap());
}

#[test]
fn pointer_drag_is_serialized_as_anchor_local_normalized_placement() {
    let placement = normalized_placement_for_drag(
        Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, u(100), u(1_000)),
        u(40),
        u(30),
        u(10),
        u(48),
        u(50),
    );

    assert_eq!(placement.inline_position, Normalized::from_raw(52_428));
    assert_eq!(placement.block_offset, 2 * BLOCK_OFFSET_SCALE);
}
