import Testing
@testable import LastDraftFlowApple

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

