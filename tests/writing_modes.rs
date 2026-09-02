use lastdraft_flow::{
    layout, AnchorAffinity, CanonicalShaper, CaretMovementDirection, EditorGeometrySnapshot,
    EditorTextPosition, ExclusionRules, FlowObject, GlyphOrientation, LayoutRequest, LogicalInsets,
    LayoutUnit, Normalized, NormalizedPlacement, ObjectSize, ParagraphDirection, ParagraphStyle,
    Rect, RichParagraph, RichTextRun, RichTextStyle, ShapedParagraph, TextAlignment, TextAnchor,
    TextDirection, TextOrientation, WritingMode,
};

fn q(value: i32) -> LayoutUnit {
    LayoutUnit::from_raw(value)
}

fn style(font_id: &str, language: &str) -> RichTextStyle {
    RichTextStyle {
        font_id: font_id.into(),
        font_size: q(18 * 64),
        // Retained for source compatibility. Paragraph UBA itemization ignores
        // this legacy shaping-direction hint.
        direction: TextDirection::LeftToRight,
        language: Some(language.into()),
        features: Vec::new(),
        variations: Vec::new(),
        fill_rgba: 0x101010ff,
        underline: false,
        strikethrough: false,
    }
}

fn paragraph(
    id: &str,
    segments: &[(&str, &str, &str)],
    base_direction: ParagraphDirection,
    writing_mode: WritingMode,
    text_orientation: TextOrientation,
) -> RichParagraph {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut offset = 0_u32;
    for (content, font_id, language) in segments {
        text.push_str(content);
        let end = offset + content.encode_utf16().count() as u32;
        runs.push(RichTextRun {
            utf16_start: offset,
            utf16_end: end,
            style: style(font_id, language),
        });
        offset = end;
    }
    RichParagraph {
        id: id.into(),
        text,
        base_direction,
        writing_mode,
        text_orientation,
        runs,
    }
}

fn shaper() -> CanonicalShaper {
    let mut shaper = CanonicalShaper::new();
    shaper
        .register_font(
            "latin",
            &include_bytes!("../fixtures/fonts/NotoSans-Variable.ttf")[..],
            0,
        )
        .unwrap();
    shaper
        .register_font(
            "arabic",
            &include_bytes!("../fixtures/fonts/NotoSansArabic-Variable.ttf")[..],
            0,
        )
        .unwrap();
    shaper
        .register_font(
            "hebrew",
            &include_bytes!("../fixtures/fonts/NotoSansHebrew-Variable.ttf")[..],
            0,
        )
        .unwrap();
    shaper
        .register_font(
            "jp",
            &include_bytes!("../fixtures/fonts/NotoSansJP-Variable.ttf")[..],
            0,
        )
        .unwrap();
    shaper
}

fn paragraph_style() -> ParagraphStyle {
    ParagraphStyle {
        line_height: q(24 * 64),
        ascent: q(18 * 64),
        space_after: LayoutUnit::ZERO,
        ..Default::default()
    }
}

#[test]
fn english_in_explicit_rtl_paragraph_stays_ltr_and_right_aligned() {
    let shaped = shaper()
        .shape_paragraph(&paragraph(
            "english-rtl",
            &[("English remains readable 123.", "latin", "en")],
            ParagraphDirection::RightToLeft,
            WritingMode::HorizontalTb,
            TextOrientation::Mixed,
        ))
        .unwrap();
    assert_eq!(shaped.base_direction, TextDirection::RightToLeft);
    assert!(shaped.runs.iter().any(|run| {
        run.utf16_start == 0
            && run.utf16_end >= "English".encode_utf16().count() as u32
            && run.direction == TextDirection::LeftToRight
    }));

    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(420 * 64), q(300 * 64));
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![shaped.to_flow_paragraph(paragraph_style())],
        objects: Vec::new(),
    })
    .unwrap();
    let final_fragment = flow.paragraphs[0]
        .lines
        .last()
        .unwrap()
        .fragments
        .last()
        .unwrap();
    assert_eq!(final_fragment.rect.right(), content.right());
}

#[test]
fn arabic_hebrew_and_mixed_numbers_receive_uba_levels_not_style_direction() {
    let shaper = shaper();
    for (id, content, font, language) in [
        ("arabic", "العربية، ١٢٣.", "arabic", "ar"),
        ("hebrew", "עברית, 123.", "hebrew", "he"),
    ] {
        let shaped = shaper
            .shape_paragraph(&paragraph(
                id,
                &[(content, font, language)],
                ParagraphDirection::Auto,
                WritingMode::HorizontalTb,
                TextOrientation::Mixed,
            ))
            .unwrap();
        assert_eq!(shaped.base_direction, TextDirection::RightToLeft);
        assert!(shaped
            .clusters
            .iter()
            .any(|cluster| cluster.direction == TextDirection::RightToLeft));
    }

    let mixed = shaper
        .shape_paragraph(&paragraph(
            "mixed",
            &[
                ("مرحبا، ", "arabic", "ar"),
                ("שלום ", "hebrew", "he"),
                ("English 123!", "latin", "en"),
            ],
            ParagraphDirection::Auto,
            WritingMode::HorizontalTb,
            TextOrientation::Mixed,
        ))
        .unwrap();
    assert_eq!(mixed.base_direction, TextDirection::RightToLeft);
    assert!(mixed.runs.iter().any(|run| run.bidi_level % 2 == 0));
    assert!(mixed.runs.iter().any(|run| run.bidi_level % 2 == 1));

    let english_start = "مرحبا، שלום ".encode_utf16().count() as u32;
    assert!(mixed.runs.iter().any(|run| {
        run.utf16_start == english_start
            && run.utf16_end >= english_start + "English 123".encode_utf16().count() as u32
            && run.direction == TextDirection::LeftToRight
    }));
}

#[test]
fn rtl_exclusion_fragments_start_at_the_right_and_selection_crosses_both_sides() {
    let shaped = shaper()
        .shape_paragraph(&paragraph(
            "rtl-image",
            &[(
                "هذا نص عربي طويل يلتف حول الصورة ثم يستمر بعد الصورة مع English 123.",
                "arabic",
                "ar",
            )],
            ParagraphDirection::RightToLeft,
            WritingMode::HorizontalTb,
            TextOrientation::Mixed,
        ))
        .unwrap();
    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(520 * 64), q(500 * 64));
    let object = FlowObject {
        id: "picture".into(),
        anchor: TextAnchor {
            paragraph_id: shaped.id.clone(),
            utf16_offset: 0,
            affinity: AnchorAffinity::Downstream,
        },
        placement: NormalizedPlacement {
            inline_position: Normalized::CENTER,
            block_offset: 0,
        },
        size: ObjectSize {
            ideal_width: q(180 * 64),
            ideal_height: q(100 * 64),
            max_inline_fraction: Normalized::END,
        },
        exclusion: ExclusionRules {
            margin: LogicalInsets::uniform(q(8 * 64)),
            minimum_fragment_width: q(60 * 64),
        },
    };
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![shaped.to_flow_paragraph(paragraph_style())],
        objects: vec![object],
    })
    .unwrap();
    let split_line = flow.paragraphs[0]
        .lines
        .iter()
        .find(|line| line.fragments.len() == 2)
        .expect("RTL text should use both viable exclusion corridors");
    assert!(split_line.fragments[0].rect.x > split_line.fragments[1].rect.x);
    let boundary = split_line.fragments[0].utf16_end;

    let snapshot = EditorGeometrySnapshot::new(flow, vec![shaped], content, q(64)).unwrap();
    let selection = snapshot.selection("rtl-image", 1, 55).unwrap();
    assert!(selection.rects.len() >= 2);
    let upstream = snapshot
        .caret(&EditorTextPosition {
            paragraph_id: "rtl-image".into(),
            utf16_offset: boundary,
            affinity: AnchorAffinity::Upstream,
        })
        .unwrap();
    let hit = snapshot.hit_test(upstream.rect.x, upstream.rect.y).unwrap();
    assert_eq!(hit.utf16_offset, boundary);
}

#[test]
fn vertical_rl_and_lr_use_vertical_metrics_orientation_and_image_exclusions() {
    let shaper = shaper();
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let shaped = shaper
            .shape_paragraph(&paragraph(
                if mode == WritingMode::VerticalRl {
                    "vertical-rl"
                } else {
                    "vertical-lr"
                },
                &[("日本語ABC、縦書きテスト。", "jp", "ja")],
                ParagraphDirection::LeftToRight,
                mode,
                TextOrientation::Mixed,
            ))
            .unwrap();
        assert!(shaped
            .runs
            .iter()
            .filter(|run| run.orientation == GlyphOrientation::Upright)
            .flat_map(|run| &run.glyphs)
            .any(|glyph| glyph.y_advance != LayoutUnit::ZERO));
        assert!(shaped
            .runs
            .iter()
            .any(|run| run.orientation == GlyphOrientation::Sideways));

        let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(180 * 64), q(320 * 64));
        let object = FlowObject {
            id: format!("picture-{mode:?}"),
            anchor: TextAnchor {
                paragraph_id: shaped.id.clone(),
                utf16_offset: 0,
                affinity: AnchorAffinity::Downstream,
            },
            placement: NormalizedPlacement {
                inline_position: Normalized::CENTER,
                block_offset: 0,
            },
            size: ObjectSize {
                ideal_width: q(70 * 64),
                ideal_height: q(110 * 64),
                max_inline_fraction: Normalized::END,
            },
            exclusion: ExclusionRules {
                margin: LogicalInsets::uniform(q(6 * 64)),
                minimum_fragment_width: q(36 * 64),
            },
        };
        let flow = layout(&LayoutRequest {
            content,
            paragraphs: vec![shaped.to_flow_paragraph(paragraph_style())],
            objects: vec![object],
        })
        .unwrap();
        let split = flow.paragraphs[0]
            .lines
            .iter()
            .find(|line| line.fragments.len() == 2)
            .expect("vertical text should flow above and below the picture");
        assert!(split.fragments[0].rect.y < split.fragments[1].rect.y);
        assert!(split
            .fragments
            .iter()
            .all(|fragment| fragment.rect.width == paragraph_style().line_height));

        let snapshot = EditorGeometrySnapshot::new(flow, vec![shaped], content, q(64)).unwrap();
        assert!(snapshot
            .caret_stops
            .iter()
            .all(|stop| stop.rect.height == q(64)));
        let first = snapshot.caret_stops.first().unwrap();
        assert_eq!(
            snapshot
                .hit_test(first.rect.x, first.rect.y)
                .unwrap()
                .utf16_offset,
            first.position.utf16_offset
        );
        let moved = snapshot
            .move_caret(&first.position, CaretMovementDirection::Down, None)
            .unwrap();
        assert!(moved.rect.y >= first.rect.y);
        assert!(!snapshot
            .selection(&first.position.paragraph_id, 0, 6)
            .unwrap()
            .rects
            .is_empty());
    }

    for (text_orientation, expected) in [
        (TextOrientation::Upright, GlyphOrientation::Upright),
        (TextOrientation::Sideways, GlyphOrientation::Sideways),
    ] {
        let shaped = shaper
            .shape_paragraph(&paragraph(
                "vertical-orientation-override",
                &[("日本ABC", "jp", "ja")],
                ParagraphDirection::LeftToRight,
                WritingMode::VerticalRl,
                text_orientation,
            ))
            .unwrap();
        assert!(shaped
            .clusters
            .iter()
            .all(|cluster| cluster.orientation == expected));
    }
}

// Slice 4b — multi-paragraph vertical block progression.
//
// Contract §7 requires block progression to run LEFT in `vertical-rl`: the
// second paragraph sits to the left of the first and shares its block start.
// `layout()` used to advance a single `cursor_y` with no writing-mode branch,
// so paragraphs marched down the page and a three-paragraph Japanese letter
// rendered as three columns stacked below one another. Every other vertical
// test in this file lays out ONE paragraph, which is why it went unseen; this
// one exists to keep a second paragraph in the picture.
#[test]
fn vertical_rl_paragraphs_advance_leftward_not_downward() {
    let shaper = shaper();
    let first = shaper
        .shape_paragraph(&paragraph(
            "v-one",
            &[("縦書きの手紙です。", "jp", "ja")],
            ParagraphDirection::LeftToRight,
            WritingMode::VerticalRl,
            TextOrientation::Mixed,
        ))
        .unwrap();
    let second = shaper
        .shape_paragraph(&paragraph(
            "v-two",
            &[("二つ目の段落です。", "jp", "ja")],
            ParagraphDirection::LeftToRight,
            WritingMode::VerticalRl,
            TextOrientation::Mixed,
        ))
        .unwrap();

    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(200 * 64), q(240 * 64));
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![
            first.to_flow_paragraph(paragraph_style()),
            second.to_flow_paragraph(paragraph_style()),
        ],
        objects: Vec::new(),
    })
    .unwrap();

    let one = &flow.paragraphs[0];
    let two = &flow.paragraphs[1];

    // The first column correctly fills from the right edge inward.
    assert_eq!(one.bounds.x + one.bounds.width, content.width);

    // Both columns start at the same place down the page ...
    assert_eq!(two.bounds.y, one.bounds.y);
    assert_eq!(one.bounds.y, content.y);
    // ... and the second sits entirely to the left of the first.
    assert!(
        two.bounds.x + two.bounds.width <= one.bounds.x,
        "paragraph two should be left of paragraph one"
    );
    // The letter grows along the block axis, which here is the width.
    assert!(flow.content_width > one.bounds.width);
    assert_eq!(flow.content_height, content.height);
}

// Slice 4c — vertical caret, selection and hit-testing across MORE THAN ONE
// paragraph.
//
// Every other vertical test in this file, and every vertical test in
// `editor_snapshot.rs`, lays out a single paragraph. That blind spot is what
// hid the block-progression defect until Slice 4a went looking, so it is worth
// closing rather than narrowing: with columns now side by side, an editor
// question like "what is under this point" or "where does Down go from here"
// only becomes interesting once a second column exists.
fn vertical_pair() -> (EditorGeometrySnapshot, Rect, Vec<ShapedParagraph>) {
    let shaper = shaper();
    // Long enough to wrap at this column length, so the second paragraph is not
    // the only thing the caret can cross into.
    let first = shaper
        .shape_paragraph(&paragraph(
            "v-one",
            &[("縦書きの手紙です。今日は良い天気ですね。", "jp", "ja")],
            ParagraphDirection::LeftToRight,
            WritingMode::VerticalRl,
            TextOrientation::Mixed,
        ))
        .unwrap();
    let second = shaper
        .shape_paragraph(&paragraph(
            "v-two",
            &[("二つ目の段落です。", "jp", "ja")],
            ParagraphDirection::LeftToRight,
            WritingMode::VerticalRl,
            TextOrientation::Mixed,
        ))
        .unwrap();

    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(240 * 64), q(200 * 64));
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![
            first.to_flow_paragraph(paragraph_style()),
            second.to_flow_paragraph(paragraph_style()),
        ],
        objects: Vec::new(),
    })
    .unwrap();
    let shaped = vec![first, second];
    let snapshot =
        EditorGeometrySnapshot::new(flow, shaped.clone(), content, q(64)).unwrap();
    (snapshot, content, shaped)
}

fn stops_of<'a>(
    snapshot: &'a EditorGeometrySnapshot,
    paragraph_id: &str,
) -> Vec<&'a lastdraft_flow::PositionedCaretStop> {
    snapshot
        .caret_stops
        .iter()
        .filter(|stop| stop.position.paragraph_id == paragraph_id)
        .collect()
}

#[test]
fn vertical_hit_testing_lands_in_the_column_under_the_point() {
    let (snapshot, _, _) = vertical_pair();
    let one = stops_of(&snapshot, "v-one");
    let two = stops_of(&snapshot, "v-two");
    assert!(!one.is_empty() && !two.is_empty());

    // The second paragraph's column sits to the LEFT of the first's, so a point
    // inside it must not resolve into the first. Before the block-progression
    // fix the two columns overlapped in x and this could not be asked at all.
    let target = two[two.len() / 2];
    let middle_x = target.rect.x + LayoutUnit::from_raw(target.rect.width.raw() / 2);
    let middle_y = target.rect.y + LayoutUnit::from_raw(target.rect.height.raw() / 2);
    let hit = snapshot.hit_test(middle_x, middle_y).expect("a position");
    assert_eq!(hit.paragraph_id, "v-two");

    // And the same question in the first paragraph's column.
    let other = one[one.len() / 2];
    let hit = snapshot
        .hit_test(
            other.rect.x + LayoutUnit::from_raw(other.rect.width.raw() / 2),
            other.rect.y + LayoutUnit::from_raw(other.rect.height.raw() / 2),
        )
        .expect("a position");
    assert_eq!(hit.paragraph_id, "v-one");
}

fn horizontal_pair() -> EditorGeometrySnapshot {
    let shaper = shaper();
    let first = shaper
        .shape_paragraph(&paragraph(
            "h-one",
            &[("The quick brown fox jumps over the lazy dog again", "latin", "en")],
            ParagraphDirection::LeftToRight,
            WritingMode::HorizontalTb,
            TextOrientation::Mixed,
        ))
        .unwrap();
    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(160 * 64), q(400 * 64));
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![first.to_flow_paragraph(paragraph_style())],
        objects: Vec::new(),
    })
    .unwrap();
    EditorGeometrySnapshot::new(flow, vec![first], content, q(64)).unwrap()
}

/// Walk one direction until the caret stops, returning every position visited.
fn walk(
    snapshot: &EditorGeometrySnapshot,
    from: &EditorTextPosition,
    direction: CaretMovementDirection,
    limit: usize,
) -> Vec<EditorTextPosition> {
    let mut visited = vec![from.clone()];
    let mut position = from.clone();
    let mut preferred = None;
    for _ in 0..limit {
        let moved = snapshot.move_caret(&position, direction, preferred).unwrap();
        if moved.position == position {
            break;
        }
        preferred = Some(moved.preferred_x);
        position = moved.position.clone();
        visited.push(position.clone());
    }
    visited
}

#[test]
fn vertical_block_movement_crosses_columns_and_reaches_the_next_paragraph() {
    // Left is the BLOCK direction in vertical-rl, so it is the one that walks
    // from column to column — and, once paragraphs sit side by side, from one
    // paragraph into the next. That could not be asked at all while every
    // vertical test laid out a single paragraph.
    let (snapshot, _, _) = vertical_pair();
    let start = snapshot.caret_stops.first().unwrap().position.clone();

    let visited = walk(&snapshot, &start, CaretMovementDirection::Left, 40);
    assert!(visited.len() > 2, "the caret never left its first column");
    assert_eq!(
        visited.last().unwrap().paragraph_id,
        "v-two",
        "block movement should reach the last paragraph"
    );

    // Each step moves strictly leftward: that is what vertical-rl means.
    let mut previous = snapshot.caret(&start).unwrap().rect.x;
    for position in visited.iter().skip(1) {
        let rect = snapshot.caret(position).unwrap().rect;
        assert!(rect.x < previous, "block movement must go left");
        previous = rect.x;
    }

    // Right retraces it, back into the first paragraph.
    let back = walk(
        &snapshot,
        visited.last().unwrap(),
        CaretMovementDirection::Right,
        40,
    );
    assert_eq!(back.last().unwrap().paragraph_id, "v-one");
}

#[test]
fn inline_movement_stops_at_a_line_end_in_both_writing_modes() {
    // KNOWN LIMITATION, and it is NOT vertical-specific: `move_caret` confines
    // inline motion to the current line, so at a wrapped line's end the caret
    // stays put instead of continuing into the next line. A real editor
    // continues. This test exists to prove vertical behaves exactly as
    // horizontal does, so the gap is one shared behaviour to fix rather than a
    // vertical defect — the browser input host (Slice 5) is what owns it.
    let horizontal = horizontal_pair();
    let start = horizontal.caret_stops.first().unwrap().position.clone();
    let visited = walk(&horizontal, &start, CaretMovementDirection::Right, 200);
    let first_line = horizontal
        .caret_stops
        .iter()
        .find(|stop| stop.position == *visited.last().unwrap())
        .map(|stop| stop.document_line_index)
        .unwrap();
    assert_eq!(first_line, 0, "horizontal inline motion stayed on line 0");
    assert!(
        horizontal
            .caret_stops
            .iter()
            .any(|stop| stop.document_line_index > 0),
        "the horizontal fixture really does wrap"
    );

    let (vertical, _, _) = vertical_pair();
    let start = vertical.caret_stops.first().unwrap().position.clone();
    let visited = walk(&vertical, &start, CaretMovementDirection::Down, 200);
    let line = vertical
        .caret_stops
        .iter()
        .find(|stop| stop.position == *visited.last().unwrap())
        .map(|stop| stop.document_line_index)
        .unwrap();
    assert_eq!(line, 0, "vertical inline motion stops the same way");
}

#[test]
fn vertical_caret_returns_by_the_way_it_came() {
    // Retracing must be exact across a column boundary AND across a paragraph
    // boundary, which is the pair of steps a single-paragraph fixture can never
    // reach.
    let (snapshot, _, _) = vertical_pair();
    let start = snapshot.caret_stops.first().unwrap().position.clone();

    let forward = walk(&snapshot, &start, CaretMovementDirection::Left, 40);
    assert!(forward.len() > 2);
    assert!(forward.iter().any(|position| position.paragraph_id == "v-two"));

    let mut position = forward.last().unwrap().clone();
    let mut preferred = None;
    for expected in forward.iter().rev().skip(1) {
        let moved = snapshot
            .move_caret(&position, CaretMovementDirection::Right, preferred)
            .unwrap();
        assert_eq!(&moved.position, expected, "the way back diverged");
        preferred = Some(moved.preferred_x);
        position = moved.position;
    }
    assert_eq!(position, start);
}

#[test]
fn vertical_block_movement_keeps_its_place_down_the_column() {
    // Slice 4 asks for preferred-axis movement: stepping between columns should
    // hold the caret's place ALONG the column, not drop it back to the top. In
    // vertical writing that preferred axis is y, where horizontally it is x.
    let (snapshot, _, _) = vertical_pair();

    // Start partway down the first column.
    let start = snapshot.caret_stops[8].position.clone();
    let from = snapshot.caret(&start).unwrap().rect;
    assert!(from.y > LayoutUnit::ZERO, "not actually partway down");

    let moved = snapshot
        .move_caret(&start, CaretMovementDirection::Left, None)
        .unwrap();
    assert!(moved.rect.x < from.x, "should have stepped to the next column");
    // Same place down the column, within one line's thickness.
    let drift = (i64::from(moved.rect.y.raw()) - i64::from(from.y.raw())).abs();
    assert!(
        drift <= i64::from(paragraph_style().line_height.raw()),
        "the caret lost its place down the column: drifted {drift}"
    );
    assert_eq!(moved.preferred_x, from.y, "the preferred axis is y here");
}

#[test]
fn a_vertical_selection_stays_inside_its_own_paragraph() {
    let (snapshot, _, shaped) = vertical_pair();
    let one = shaped[0].utf16_len;
    let two = shaped[1].utf16_len;

    let first = snapshot.selection("v-one", 0, one).unwrap();
    let second = snapshot.selection("v-two", 0, two).unwrap();
    assert!(!first.rects.is_empty() && !second.rects.is_empty());

    // The first paragraph wraps, so its selection spans more than one column.
    assert!(
        first.rects.len() > 1,
        "a wrapped vertical paragraph selects one rect per column"
    );

    // The second paragraph's rects are entirely left of the first's columns.
    let leftmost_of_first = first
        .rects
        .iter()
        .map(|rect| rect.x)
        .min()
        .unwrap();
    for rect in &second.rects {
        assert!(
            rect.x + rect.width <= leftmost_of_first,
            "the second paragraph's selection leaked into the first's columns"
        );
    }
}

#[test]
fn a_vertical_selection_rect_runs_down_its_column_not_across_the_page() {
    let (snapshot, content, shaped) = vertical_pair();
    let selection = snapshot
        .selection("v-one", 0, shaped[0].utf16_len)
        .unwrap();

    for rect in &selection.rects {
        // One column thick, and running along the inline (vertical) axis.
        assert_eq!(rect.width, paragraph_style().line_height);
        assert!(rect.height > LayoutUnit::ZERO);
        // Inside the paper, on the axis the words run along.
        assert!(rect.y >= content.y);
        assert!(rect.bottom() <= content.bottom());
    }
}

// Alignment and indentation.
//
// Both are LOGICAL in Scribe and physical in the engine's geometry, so both
// have to be checked in each writing mode rather than only in horizontal-ltr,
// where the two happen to coincide.
fn aligned(
    text: &str,
    mode: WritingMode,
    base: ParagraphDirection,
    alignment: TextAlignment,
    indent: LayoutUnit,
) -> (lastdraft_flow::FlowLayout, Rect) {
    let shaper = shaper();
    let font = if mode == WritingMode::HorizontalTb { "latin" } else { "jp" };
    let language = if font == "latin" { "en" } else { "ja" };
    let shaped = shaper
        .shape_paragraph(&paragraph(
            "aligned",
            &[(text, font, language)],
            base,
            mode,
            TextOrientation::Mixed,
        ))
        .unwrap();
    let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(400 * 64), q(400 * 64));
    let style = ParagraphStyle {
        alignment,
        indent,
        ..paragraph_style()
    };
    let flow = layout(&LayoutRequest {
        content,
        paragraphs: vec![shaped.to_flow_paragraph(style)],
        objects: Vec::new(),
    })
    .unwrap();
    (flow, content)
}

fn only_fragment(flow: &lastdraft_flow::FlowLayout) -> Rect {
    let line = &flow.paragraphs[0].lines[0];
    assert_eq!(line.fragments.len(), 1, "expected one unbroken fragment");
    line.fragments[0].rect
}

#[test]
fn horizontal_alignment_places_a_short_line_in_its_available_space() {
    let short = "Yours,";
    let zero = LayoutUnit::ZERO;

    let (start, content) = aligned(
        short, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Start, zero,
    );
    let (centre, _) = aligned(
        short, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Center, zero,
    );
    let (end, _) = aligned(
        short, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::End, zero,
    );

    let (s, c, e) = (only_fragment(&start), only_fragment(&centre), only_fragment(&end));
    // Same words, so the same width wherever they sit.
    assert_eq!(s.width, c.width);
    assert_eq!(s.width, e.width);
    assert!(s.width < content.width, "the line must not fill the column");

    assert_eq!(s.x, content.x);
    assert!(c.x > s.x && c.x < e.x, "centre sits between start and end");
    assert_eq!(e.right(), content.right());
    // Centred means equal slack on both sides, within a rounding unit.
    let left = c.x - content.x;
    let right = content.right() - c.right();
    assert!((left.raw() - right.raw()).abs() <= 1);
}

#[test]
fn alignment_is_logical_so_rtl_mirrors_it() {
    let short = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
    let zero = LayoutUnit::ZERO;
    let shaper_font = |alignment| {
        let shaper = shaper();
        let shaped = shaper
            .shape_paragraph(&paragraph(
                "rtl-aligned",
                &[(short, "hebrew", "he")],
                ParagraphDirection::RightToLeft,
                WritingMode::HorizontalTb,
                TextOrientation::Mixed,
            ))
            .unwrap();
        let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(400 * 64), q(400 * 64));
        let flow = layout(&LayoutRequest {
            content,
            paragraphs: vec![shaped.to_flow_paragraph(ParagraphStyle {
                alignment,
                indent: zero,
                ..paragraph_style()
            })],
            objects: Vec::new(),
        })
        .unwrap();
        (only_fragment(&flow), content)
    };

    let (start, content) = shaper_font(TextAlignment::Start);
    let (end, _) = shaper_font(TextAlignment::End);
    // In rtl the logical start is the RIGHT edge, and the logical end the left.
    assert_eq!(start.right(), content.right());
    assert_eq!(end.x, content.x);
}

#[test]
fn vertical_alignment_runs_down_the_column() {
    let short = "\u{6BCD}\u{3088}\u{308A}";
    let zero = LayoutUnit::ZERO;
    let (start, content) = aligned(
        short, WritingMode::VerticalRl, ParagraphDirection::LeftToRight,
        TextAlignment::Start, zero,
    );
    let (end, _) = aligned(
        short, WritingMode::VerticalRl, ParagraphDirection::LeftToRight,
        TextAlignment::End, zero,
    );

    let (s, e) = (only_fragment(&start), only_fragment(&end));
    // The inline axis is vertical here, so alignment moves the line DOWN the
    // column, never across the page.
    assert_eq!(s.y, content.y);
    assert_eq!(e.bottom(), content.bottom());
    assert_eq!(s.x, e.x, "alignment must not move the column itself");
}

#[test]
fn an_indent_shortens_the_line_from_its_inline_start() {
    let text = "Dear friend";
    let none = LayoutUnit::ZERO;
    let step = q(40 * 64);

    let (plain, content) = aligned(
        text, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Start, none,
    );
    let (indented, _) = aligned(
        text, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Start, step,
    );
    assert_eq!(only_fragment(&plain).x, content.x);
    assert_eq!(only_fragment(&indented).x, content.x + step);

    // An indented line also has less room, so an end-aligned one still finishes
    // at the same edge: the indent eats the START, not the end.
    let (end, _) = aligned(
        text, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::End, step,
    );
    assert_eq!(only_fragment(&end).right(), content.right());
}

#[test]
fn a_vertical_indent_starts_the_column_lower_not_further_left() {
    let text = "\u{6BCD}\u{3088}\u{308A}";
    let step = q(40 * 64);
    let (plain, content) = aligned(
        text, WritingMode::VerticalRl, ParagraphDirection::LeftToRight,
        TextAlignment::Start, LayoutUnit::ZERO,
    );
    let (indented, _) = aligned(
        text, WritingMode::VerticalRl, ParagraphDirection::LeftToRight,
        TextAlignment::Start, step,
    );
    let (p, i) = (only_fragment(&plain), only_fragment(&indented));
    assert_eq!(p.y, content.y);
    assert_eq!(i.y, content.y + step);
    assert_eq!(p.x, i.x, "an indent must not move the column sideways");
}

#[test]
fn justify_is_not_implemented_and_currently_sets_flush_at_the_start() {
    // KNOWN GAP. Contract §3 lists `justify` among the alignments Scribe owns.
    // Stretching a line needs slack distributed between its clusters, which is
    // glyph-positioning work in the snapshot rather than interval arithmetic in
    // layout(), so it is not built. Until it is, a justified paragraph is set
    // flush at its start edge -- the same as `Start`.
    //
    // This asserts the CURRENT behaviour on purpose so the gap cannot be
    // mistaken for finished. Flip it when justification lands.
    let text = "The quick brown fox jumps over the lazy dog and keeps running";
    let (justified, _) = aligned(
        text, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Justify, LayoutUnit::ZERO,
    );
    let (start, _) = aligned(
        text, WritingMode::HorizontalTb, ParagraphDirection::LeftToRight,
        TextAlignment::Start, LayoutUnit::ZERO,
    );
    let j = &justified.paragraphs[0].lines;
    let s = &start.paragraphs[0].lines;
    assert!(j.len() > 1, "the fixture must actually wrap");
    assert_eq!(j.len(), s.len());
    for (left, right) in j.iter().zip(s.iter()) {
        assert_eq!(left.fragments[0].rect, right.fragments[0].rect);
    }
}

// Exclusion margins are LOGICAL in the document and physical in the geometry.
//
// The resolution lives in the engine rather than in a platform adapter for one
// concrete reason: with `baseDirection: auto` — the default for a letter — the
// adapter does not yet know whether a paragraph is ltr or rtl, because the
// engine is what runs the bidi algorithm. An adapter-side mapping therefore
// cannot be right in the common case.
fn distinct_margins() -> lastdraft_flow::LogicalInsets {
    // Four different values, so no wrong mapping can pass by symmetry.
    lastdraft_flow::LogicalInsets {
        inline_start: q(64),
        block_start: q(2 * 64),
        inline_end: q(3 * 64),
        block_end: q(4 * 64),
    }
}

#[test]
fn logical_margins_resolve_onto_physical_sides_per_direction() {
    let margins = distinct_margins();

    // Horizontal ltr: the two coincide, which is why a mapping that ignored
    // direction looked correct for so long.
    let ltr = margins.resolve(WritingMode::HorizontalTb, TextDirection::LeftToRight);
    assert_eq!(ltr.start, margins.inline_start);
    assert_eq!(ltr.end, margins.inline_end);
    assert_eq!(ltr.top, margins.block_start);
    assert_eq!(ltr.bottom, margins.block_end);

    // Horizontal rtl: the inline pair swaps, because logical start is the RIGHT
    // edge. The block pair does not, because lines still advance downward.
    let rtl = margins.resolve(WritingMode::HorizontalTb, TextDirection::RightToLeft);
    assert_eq!(rtl.end, margins.inline_start);
    assert_eq!(rtl.start, margins.inline_end);
    assert_eq!(rtl.top, margins.block_start);
    assert_eq!(rtl.bottom, margins.block_end);

    // vertical-rl: the inline axis runs DOWN and the block axis runs LEFT, so
    // inline start is the top and block start is the right.
    let vertical = margins.resolve(WritingMode::VerticalRl, TextDirection::LeftToRight);
    assert_eq!(vertical.top, margins.inline_start);
    assert_eq!(vertical.bottom, margins.inline_end);
    assert_eq!(vertical.end, margins.block_start);
    assert_eq!(vertical.start, margins.block_end);

    // vertical-lr: same inline axis, block axis runs the other way.
    let vertical_lr = margins.resolve(WritingMode::VerticalLr, TextDirection::LeftToRight);
    assert_eq!(vertical_lr.top, margins.inline_start);
    assert_eq!(vertical_lr.bottom, margins.inline_end);
    assert_eq!(vertical_lr.start, margins.block_start);
    assert_eq!(vertical_lr.end, margins.block_end);
}

#[test]
fn an_rtl_paragraph_excludes_on_the_mirrored_side() {
    // End to end, not just the mapping function: the same stored margins must
    // produce mirrored exclusion frames in ltr and rtl.
    let shaper = shaper();
    let frames: Vec<Rect> = [
        (ParagraphDirection::LeftToRight, "latin", "en", "The quick brown fox jumps over it"),
        (ParagraphDirection::RightToLeft, "hebrew", "he", "\u{05E9}\u{05DC}\u{05D5}\u{05DD} \u{05E2}\u{05D5}\u{05DC}\u{05DD} \u{05D5}\u{05E2}\u{05D5}\u{05D3}"),
    ]
    .into_iter()
    .map(|(direction, font, language, text)| {
        let shaped = shaper
            .shape_paragraph(&paragraph(
                "margins",
                &[(text, font, language)],
                direction,
                WritingMode::HorizontalTb,
                TextOrientation::Mixed,
            ))
            .unwrap();
        let content = Rect::new(LayoutUnit::ZERO, LayoutUnit::ZERO, q(300 * 64), q(300 * 64));
        let object = FlowObject {
            id: "picture".into(),
            anchor: TextAnchor {
                paragraph_id: shaped.id.clone(),
                utf16_offset: 0,
                affinity: AnchorAffinity::Downstream,
            },
            placement: NormalizedPlacement {
                inline_position: Normalized::CENTER,
                block_offset: 0,
            },
            size: ObjectSize {
                ideal_width: q(60 * 64),
                ideal_height: q(60 * 64),
                max_inline_fraction: Normalized::END,
            },
            exclusion: ExclusionRules {
                margin: distinct_margins(),
                minimum_fragment_width: q(20 * 64),
            },
        };
        let flow = layout(&LayoutRequest {
            content,
            paragraphs: vec![shaped.to_flow_paragraph(paragraph_style())],
            objects: vec![object],
        })
        .unwrap();
        flow.objects[0].exclusion_frame
    })
    .collect();

    let (ltr, rtl) = (frames[0], frames[1]);
    // The picture itself is the same size and in the same place ...
    assert_eq!(ltr.width, rtl.width);
    // ... but the margin around it is mirrored: 1 unit of inline-start sits on
    // the left in ltr and on the right in rtl, and 3 units of inline-end the
    // other way about. So the two frames cannot be identical.
    assert_ne!(ltr.x, rtl.x);
    // Total width is unchanged, because the same two margins are simply swapped.
    assert_eq!(ltr.width, rtl.width);
    // And the block margins are untouched by direction.
    assert_eq!(ltr.y, rtl.y);
    assert_eq!(ltr.height, rtl.height);
}
