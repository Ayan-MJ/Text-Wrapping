#if canImport(UIKit)
import UIKit

@MainActor
public protocol LastDraftFlowViewDelegate: AnyObject {
    func flowView(
        _ view: LastDraftFlowView,
        draw fragment: FlowFragment,
        paragraph: FlowParagraphLayout,
        line: FlowLine,
        in context: CGContext
    )
    func flowView(_ view: LastDraftFlowView, imageFor object: ResolvedFlowObject) -> UIImage?
    func flowView(_ view: LastDraftFlowView, didChange placement: NormalizedFlowPlacement, for objectID: String)
}

@MainActor
public final class LastDraftFlowView: UIView {
    public weak var delegate: LastDraftFlowViewDelegate?
    public var layout: FlowLayout? {
        didSet {
            invalidateIntrinsicContentSize()
            setNeedsDisplay()
            updateAccessibilityObjects()
        }
    }
    public var contentRect: FlowRect = .init(xQ26_6: 0, yQ26_6: 0, widthQ26_6: 0, heightQ26_6: 0)
    public var paragraphLineHeightsQ26_6: [String: Int32] = [:]

    private let engine: LastDraftFlowEngine
    private var draggedObject: ResolvedFlowObject?
    private var initialFrame: CGRect = .zero

    public init(engine: LastDraftFlowEngine) {
        self.engine = engine
        super.init(frame: .zero)
        isOpaque = false
        isMultipleTouchEnabled = false
        addGestureRecognizer(UIPanGestureRecognizer(target: self, action: #selector(handlePan(_:))))
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("Use init(engine:)")
    }

    public override var intrinsicContentSize: CGSize {
        CGSize(
            width: UIView.noIntrinsicMetric,
            height: layout.map {
                let extent = max(Int64(0), Int64(contentRect.yQ26_6) + Int64($0.contentHeightQ26_6))
                return flowPoint(Int32(clamping: extent))
            } ?? 0
        )
    }

    public override func draw(_ rect: CGRect) {
        guard let layout, let context = UIGraphicsGetCurrentContext() else { return }
        for paragraph in layout.paragraphs {
            for line in paragraph.lines {
                for fragment in line.fragments {
                    delegate?.flowView(self, draw: fragment, paragraph: paragraph, line: line, in: context)
                }
            }
        }
        for object in layout.objects {
            delegate?.flowView(self, imageFor: object)?.draw(in: object.frame.cgRect)
        }
    }

    @objc private func handlePan(_ recognizer: UIPanGestureRecognizer) {
        guard let layout else { return }
        let point = recognizer.location(in: self)
        switch recognizer.state {
        case .began:
            draggedObject = layout.objects.reversed().first { $0.frame.cgRect.contains(point) }
            initialFrame = draggedObject?.frame.cgRect ?? .zero
        case .changed:
            guard let object = draggedObject,
                  let lineHeight = paragraphLineHeightsQ26_6[object.anchor.paragraphId]
            else { return }
            let translation = recognizer.translation(in: self)
            do {
                let placement = try engine.normalizedPlacementForDrag(
                    contentXQ26_6: contentRect.xQ26_6,
                    contentWidthQ26_6: contentRect.widthQ26_6,
                    objectWidthQ26_6: object.frame.widthQ26_6,
                    anchorReferenceTopQ26_6: object.anchorReferenceTopQ26_6,
                    lineHeightQ26_6: lineHeight,
                    proposedXQ26_6: flowUnit(initialFrame.minX + translation.x),
                    proposedYQ26_6: flowUnit(initialFrame.minY + translation.y)
                )
                delegate?.flowView(self, didChange: placement, for: object.id)
            } catch {
                assertionFailure("LastDraft Flow drag failed: \(error)")
            }
        case .ended, .cancelled, .failed:
            draggedObject = nil
        default:
            break
        }
    }

    private func updateAccessibilityObjects() {
        guard let layout else {
            accessibilityElements = []
            return
        }
        accessibilityElements = layout.objects.map { object in
            let element = UIAccessibilityElement(accessibilityContainer: self)
            element.accessibilityLabel = "Image"
            element.accessibilityTraits = .image
            element.accessibilityFrameInContainerSpace = object.frame.cgRect
            return element
        }
    }
}
#endif
