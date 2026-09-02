import Foundation
import Testing
@testable import LastDraftFlowApple

private func repositoryFixtures() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent("fixtures")
}

private func geometryFingerprint(_ geometry: EditorGeometrySnapshotData) -> String {
    var tokens = [
        "lastdraft-geometry-v2",
        String(geometry.layout.contentHeightQ26_6),
        String(geometry.layout.contentWidthQ26_6 ?? 0),
    ]
    func add(_ values: Any...) {
        tokens.append(contentsOf: values.map { String(describing: $0) })
    }
    for paragraph in geometry.layout.paragraphs {
        add(
            "p", paragraph.paragraphId, paragraph.baseDirection?.rawValue ?? "",
            paragraph.writingMode?.rawValue ?? "", paragraph.topQ26_6, paragraph.bottomQ26_6
        )
        for line in paragraph.lines {
            add(
                "l", line.writingMode?.rawValue ?? "", line.topQ26_6, line.baselineQ26_6,
                line.blockStartQ26_6 ?? 0, line.inlineStartQ26_6 ?? 0
            )
            for fragment in line.fragments {
                add(
                    "f", fragment.utf16Start, fragment.utf16End,
                    fragment.rect.xQ26_6, fragment.rect.yQ26_6,
                    fragment.rect.widthQ26_6, fragment.rect.heightQ26_6
                )
            }
        }
    }
    for object in geometry.layout.objects {
        add(
            "o", object.id, object.anchor.paragraphId, object.anchor.utf16Offset,
            object.anchor.affinity.rawValue, object.anchorReferenceBlockStartQ26_6 ?? 0,
            object.frame.xQ26_6, object.frame.yQ26_6,
            object.frame.widthQ26_6, object.frame.heightQ26_6,
            object.exclusionFrame.xQ26_6, object.exclusionFrame.yQ26_6,
            object.exclusionFrame.widthQ26_6, object.exclusionFrame.heightQ26_6,
            object.mode.rawValue
        )
    }
    for cluster in geometry.clusters {
        add(
            "c", cluster.paragraphId, cluster.utf16Start, cluster.utf16End,
            cluster.bidiLevel, cluster.direction.rawValue, cluster.orientation.rawValue,
            cluster.writingMode.rawValue, cluster.rect.xQ26_6, cluster.rect.yQ26_6,
            cluster.rect.widthQ26_6, cluster.rect.heightQ26_6
        )
    }
    for glyph in geometry.glyphs {
        add(
            "g", glyph.paragraphId, glyph.runIndex, glyph.glyphIndex, glyph.glyphId,
            glyph.clusterUtf16, glyph.pageXQ26_6, glyph.pageYQ26_6,
            glyph.xAdvanceQ26_6, glyph.yAdvanceQ26_6,
            glyph.xOffsetQ26_6, glyph.yOffsetQ26_6,
            glyph.orientation.rawValue, glyph.writingMode.rawValue,
            glyph.lineIndex, glyph.fragmentIndex
        )
    }
    for stop in geometry.caretStops {
        add(
            "s", stop.position.paragraphId, stop.position.utf16Offset,
            stop.position.affinity.rawValue, stop.writingMode.rawValue,
            stop.rect.xQ26_6, stop.rect.yQ26_6,
            stop.rect.widthQ26_6, stop.rect.heightQ26_6,
            stop.lineIndex, stop.documentLineIndex, stop.fragmentIndex, stop.visualIndex
        )
    }
    var hash: UInt64 = 0xcbf29ce484222325
    for byte in tokens.joined(separator: "|").utf8 {
        hash ^= UInt64(byte)
        hash = hash &* 0x100000001b3
    }
    return String(format: "%016llx", hash)
}

@Test("Swift/C bridge preserves block IDs and two-sided fragments")
func swiftBridgeLayout() throws {
    let paragraphID = "lastdraft-prosemirror-block"
    let paragraph = FlowParagraph(
        id: paragraphID,
        utf16Length: 4,
        style: FlowParagraphStyle(lineHeightQ26_6: 10, ascentQ26_6: 8),
        clusters: (0..<4).map { index in
            FlowCluster(
                utf16Start: UInt32(index),
                utf16End: UInt32(index + 1),
                advanceQ26_6: 10,
                canBreakAfter: true
            )
        }
    )
    let object = AnchoredFlowObjectV1(
        id: "fl_uuid",
        anchor: FlowAnchor(paragraphId: paragraphID, utf16Offset: 0),
        placement: NormalizedFlowPlacement(inlineU16: 32_768, blockOffset1024: 0),
        size: FlowObjectSize(idealWidthQ26_6: 40, idealHeightQ26_6: 10),
        exclusion: FlowExclusion(minimumFragmentWidthQ26_6: 10)
    )
    let request = FlowLayoutRequestV1(
        content: FlowRect(xQ26_6: 0, yQ26_6: 0, widthQ26_6: 100, heightQ26_6: 10_000),
        paragraphs: [paragraph],
        objects: [object]
    )

    let layout = try LastDraftFlowEngine().layout(request)
    #expect(layout.paragraphs[0].paragraphId == paragraphID)
    #expect(layout.paragraphs[0].lines[0].fragments.count == 2)
    #expect(layout.objects[0].id == "fl_uuid")
}

@Test("Swift delegates drag normalization to Rust")
func swiftBridgeDrag() throws {
    let placement = try LastDraftFlowEngine().normalizedPlacementForDrag(
        contentXQ26_6: 0,
        contentWidthQ26_6: 100,
        objectWidthQ26_6: 40,
        anchorReferenceTopQ26_6: 30,
        lineHeightQ26_6: 10,
        proposedXQ26_6: 48,
        proposedYQ26_6: 50
    )
    #expect(placement == NormalizedFlowPlacement(inlineU16: 52_428, blockOffset1024: 2_048))
}

@Test("Swift registers exact font bytes and shapes through the shared engine")
func swiftCanonicalShaping() throws {
    let fontURL = URL(fileURLWithPath: "/System/Library/Fonts/Supplemental/Arial.ttf")
    guard FileManager.default.fileExists(atPath: fontURL.path) else { return }
    let shaper = try LastDraftCanonicalShaper()
    let descriptor = try shaper.registerFont(id: "test-arial", data: Data(contentsOf: fontURL))
    #expect(descriptor.sha256.count == 64)

    let text = "office"
    let paragraph = CanonicalRichParagraph(
        id: "rich-block",
        text: text,
        runs: [
            CanonicalRichTextRun(
                utf16Start: 0,
                utf16End: UInt32(text.utf16.count),
                style: CanonicalRichTextStyle(
                    fontId: "test-arial",
                    fontSizeQ26_6: flowUnit(16),
                    direction: .leftToRight,
                    language: "en",
                    features: [CanonicalFontFeature(tag: "liga", value: 1)]
                )
            ),
        ]
    )
    let shaped = try shaper.shape(paragraph)
    #expect(shaped.id == paragraph.id)
    #expect(shaped.runs.first?.glyphs.isEmpty == false)
    #expect(shaped.clusters.last?.utf16End == UInt32(text.utf16.count))
}

@Test("Swift/C bridge returns Unicode caret, selection, hit-test, movement, and page glyph geometry")
func swiftEditorGeometry() throws {
    let repository = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
    let fixtures = repository.appendingPathComponent("fixtures")
    let shaper = try LastDraftCanonicalShaper()
    for (id, file) in [
        ("fixture-noto-sans", "NotoSans-Variable.ttf"),
        ("fixture-noto-sans-italic", "NotoSans-Italic-Variable.ttf"),
        ("fixture-noto-devanagari", "NotoSansDevanagari-Variable.ttf"),
        ("fixture-noto-arabic", "NotoSansArabic-Variable.ttf"),
    ] {
        _ = try shaper.registerFont(
            id: id,
            data: Data(contentsOf: fixtures.appendingPathComponent("fonts/\(file)"))
        )
    }
    let request = try JSONDecoder().decode(
        EditorSnapshotRequestV1.self,
        from: Data(contentsOf: fixtures.appendingPathComponent("editor/unicode-exclusion-v1.json"))
    )
    let snapshot = try shaper.createEditorSnapshot(request)
    #expect(snapshot.geometry.layout.paragraphs[0].lines[0].fragments.count == 2)
    #expect(snapshot.geometry.glyphs.isEmpty == false)
    #expect(snapshot.geometry.clusters.contains { $0.direction == .rightToLeft })
    #expect(snapshot.geometry.caretStops.contains { $0.position.utf16Offset == 26 } == false)

    let fragments = snapshot.geometry.layout.paragraphs[0].lines[0].fragments
    let boundary = fragments[0].utf16End
    #expect(fragments[1].utf16Start == boundary)
    let upstream = try snapshot.caret(at: CanonicalTextPosition(
        paragraphId: "unicode-exclusion", utf16Offset: boundary, affinity: .upstream
    ))
    let downstream = try snapshot.caret(at: CanonicalTextPosition(
        paragraphId: "unicode-exclusion", utf16Offset: boundary, affinity: .downstream
    ))
    #expect(upstream.rect.xQ26_6 < downstream.rect.xQ26_6)
    #expect(try snapshot.hitTest(
        xQ26_6: upstream.rect.xQ26_6 + 1,
        yQ26_6: upstream.rect.yQ26_6
    )?.affinity == .upstream)
    #expect(try snapshot.selection(
        paragraphId: "unicode-exclusion", utf16Start: 1, utf16End: 90
    ).rects.count >= 3)
    #expect(try snapshot.moveCaret(from: upstream.position, direction: .right).position == downstream.position)
}

@Test("Swift ABI v4 matches the Web mixed-bidi geometry snapshot")
func swiftBidiGeometryV4() throws {
    let fixtures = repositoryFixtures()
    let shaper = try LastDraftCanonicalShaper()
    for (id, file) in [
        ("fixture-noto-sans", "NotoSans-Variable.ttf"),
        ("fixture-noto-arabic", "NotoSansArabic-Variable.ttf"),
        ("fixture-noto-hebrew", "NotoSansHebrew-Variable.ttf"),
    ] {
        _ = try shaper.registerFont(
            id: id,
            data: Data(contentsOf: fixtures.appendingPathComponent("fonts/" + file))
        )
    }
    let request = try JSONDecoder().decode(
        EditorSnapshotRequestV2.self,
        from: Data(contentsOf: fixtures.appendingPathComponent("editor/bidi-horizontal-v2.json"))
    )
    let snapshot = try shaper.createEditorSnapshot(request)
    let paragraph = snapshot.geometry.layout.paragraphs[0]
    #expect(paragraph.baseDirection == .rightToLeft)
    #expect(paragraph.writingMode == .horizontalTb)
    let split = paragraph.lines.first { $0.fragments.count == 2 }
    #expect(split != nil)
    #expect(split!.fragments[0].rect.xQ26_6 > split!.fragments[1].rect.xQ26_6)
    #expect(snapshot.geometry.clusters.contains { $0.bidiLevel % 2 == 0 })
    #expect(snapshot.geometry.clusters.contains { $0.bidiLevel % 2 == 1 })
    #expect(geometryFingerprint(snapshot.geometry) == "afaf495e47e79478")
}

@Test("Swift ABI v4 matches Web vertical geometry and editor/drag axes")
func swiftVerticalGeometryV4() throws {
    let fixtures = repositoryFixtures()
    let shaper = try LastDraftCanonicalShaper()
    _ = try shaper.registerFont(
        id: "fixture-noto-jp",
        data: Data(contentsOf: fixtures.appendingPathComponent("fonts/NotoSansJP-Variable.ttf"))
    )
    let request = try JSONDecoder().decode(
        EditorSnapshotRequestV2.self,
        from: Data(contentsOf: fixtures.appendingPathComponent("editor/vertical-v2.json"))
    )
    let first = try shaper.createEditorSnapshot(request)
    let second = try shaper.createEditorSnapshot(request)
    #expect(first.geometry == second.geometry)
    #expect(first.geometry.layout.paragraphs.map(\.writingMode) == [.verticalRl, .verticalLr])
    #expect(first.geometry.layout.paragraphs.allSatisfy {
        $0.lines.contains { $0.fragments.count == 2 }
    })
    #expect(first.geometry.glyphs.contains {
        $0.orientation == .upright && $0.yAdvanceQ26_6 != 0
    })
    #expect(first.geometry.glyphs.contains { $0.orientation == .sideways })
    #expect(first.geometry.caretStops.allSatisfy { $0.rect.heightQ26_6 == 64 })
    let stop = first.geometry.caretStops[0]
    #expect(try first.hitTest(
        xQ26_6: stop.rect.xQ26_6,
        yQ26_6: stop.rect.yQ26_6
    )?.utf16Offset == stop.position.utf16Offset)
    #expect(try first.selection(
        paragraphId: "vertical-rl",
        utf16Start: 0,
        utf16End: 6
    ).rects.isEmpty == false)
    #expect(try first.moveCaret(
        from: stop.position,
        direction: .down
    ).rect.yQ26_6 >= stop.rect.yQ26_6)
    #expect(geometryFingerprint(first.geometry) == "4a6e9d22d58d71c6")

    let placement = try LastDraftFlowEngine().normalizedPlacementForDragV4(
        content: FlowRect(xQ26_6: 0, yQ26_6: 0, widthQ26_6: 100, heightQ26_6: 200),
        objectWidthQ26_6: 20,
        objectHeightQ26_6: 40,
        anchorReferenceBlockStartQ26_6: 100,
        lineHeightQ26_6: 10,
        proposedXQ26_6: 60,
        proposedYQ26_6: 80,
        writingMode: .verticalRl
    )
    #expect(placement == NormalizedFlowPlacement(inlineU16: 32_768, blockOffset1024: 2_048))
}
