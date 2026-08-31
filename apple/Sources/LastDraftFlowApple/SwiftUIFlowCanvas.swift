#if canImport(SwiftUI) && canImport(UIKit)
import SwiftUI
import UIKit

@MainActor
public final class LastDraftFlowRenderer: LastDraftFlowViewDelegate {
    public typealias TextPainter = (
        FlowFragment,
        FlowParagraphLayout,
        FlowLine,
        CGContext
    ) -> Void
    public typealias ImageProvider = (ResolvedFlowObject) -> UIImage?

    private let textPainter: TextPainter
    private let imageProvider: ImageProvider
    private let placementChanged: (String, NormalizedFlowPlacement) -> Void

    public init(
        textPainter: @escaping TextPainter,
        imageProvider: @escaping ImageProvider,
        placementChanged: @escaping (String, NormalizedFlowPlacement) -> Void
    ) {
        self.textPainter = textPainter
        self.imageProvider = imageProvider
        self.placementChanged = placementChanged
    }

    public func flowView(
        _ view: LastDraftFlowView,
        draw fragment: FlowFragment,
        paragraph: FlowParagraphLayout,
        line: FlowLine,
        in context: CGContext
    ) {
        textPainter(fragment, paragraph, line, context)
    }

    public func flowView(_ view: LastDraftFlowView, imageFor object: ResolvedFlowObject) -> UIImage? {
        imageProvider(object)
    }

    public func flowView(
        _ view: LastDraftFlowView,
        didChange placement: NormalizedFlowPlacement,
        for objectID: String
    ) {
        placementChanged(objectID, placement)
    }
}

public struct LastDraftFlowCanvas: UIViewRepresentable {
    public var layout: FlowLayout
    public var contentRect: FlowRect
    public var paragraphLineHeightsQ26_6: [String: Int32]
    public var renderer: LastDraftFlowRenderer
    private let engine: LastDraftFlowEngine

    public init(
        engine: LastDraftFlowEngine,
        layout: FlowLayout,
        contentRect: FlowRect,
        paragraphLineHeightsQ26_6: [String: Int32],
        renderer: LastDraftFlowRenderer
    ) {
        self.engine = engine
        self.layout = layout
        self.contentRect = contentRect
        self.paragraphLineHeightsQ26_6 = paragraphLineHeightsQ26_6
        self.renderer = renderer
    }

    public func makeUIView(context: Context) -> LastDraftFlowView {
        let view = LastDraftFlowView(engine: engine)
        view.delegate = renderer
        return view
    }

    public func updateUIView(_ view: LastDraftFlowView, context: Context) {
        view.delegate = renderer
        view.contentRect = contentRect
        view.paragraphLineHeightsQ26_6 = paragraphLineHeightsQ26_6
        view.layout = layout
    }

    public func sizeThatFits(
        _ proposal: ProposedViewSize,
        uiView: LastDraftFlowView,
        context: Context
    ) -> CGSize? {
        guard let width = proposal.width else { return nil }
        let extent = max(Int64(0), Int64(contentRect.yQ26_6) + Int64(layout.contentHeightQ26_6))
        let height = flowPoint(Int32(clamping: extent))
        return CGSize(width: width, height: height)
    }
}
#endif
