import CLastDraftFlow
import Foundation

public struct EditorRichParagraph: Codable, Equatable, Sendable {
    public var paragraph: CanonicalRichParagraph
    public var style: FlowParagraphStyle

    public init(paragraph: CanonicalRichParagraph, style: FlowParagraphStyle) {
        self.paragraph = paragraph
        self.style = style
    }
}

public struct EditorSnapshotRequestV1: Codable, Equatable, Sendable {
    public let version: Int
    public var content: FlowRect
    public var paragraphs: [EditorRichParagraph]
    public var objects: [AnchoredFlowObjectV1]
    public var caretWidthQ26_6: Int32

    public init(
        content: FlowRect,
        paragraphs: [EditorRichParagraph],
        objects: [AnchoredFlowObjectV1],
        caretWidthQ26_6: Int32 = 64
    ) {
        version = 1
        self.content = content
        self.paragraphs = paragraphs
        self.objects = objects
        self.caretWidthQ26_6 = caretWidthQ26_6
    }
}

public struct EditorSnapshotRequestV2: Codable, Equatable, Sendable {
    public let version: Int
    public var content: FlowRect
    public var paragraphs: [EditorRichParagraph]
    public var objects: [AnchoredFlowObjectV1]
    public var caretWidthQ26_6: Int32

    public init(
        content: FlowRect,
        paragraphs: [EditorRichParagraph],
        objects: [AnchoredFlowObjectV1],
        caretWidthQ26_6: Int32 = 64
    ) {
        version = 2
        self.content = content
        self.paragraphs = paragraphs
        self.objects = objects
        self.caretWidthQ26_6 = caretWidthQ26_6
    }
}

public struct CanonicalPositionedLine: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var lineIndex: Int
    public var documentLineIndex: Int
    public var topQ26_6: Int32
    public var baselineQ26_6: Int32
    public var writingMode: FlowWritingMode
    public var blockStartQ26_6: Int32
    public var inlineStartQ26_6: Int32
    public var fragmentCount: Int
}

public struct CanonicalPositionedCaretStop: Codable, Equatable, Sendable {
    public var position: CanonicalTextPosition
    public var rect: FlowRect
    public var lineIndex: Int
    public var documentLineIndex: Int
    public var fragmentIndex: Int
    public var visualIndex: Int
    public var writingMode: FlowWritingMode
}

public struct CanonicalPositionedCluster: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var direction: CanonicalTextDirection
    public var bidiLevel: UInt8
    public var orientation: CanonicalGlyphOrientation
    public var writingMode: FlowWritingMode
    public var rect: FlowRect
    public var lineIndex: Int
    public var documentLineIndex: Int
    public var fragmentIndex: Int
    public var visualIndex: Int
    public var caretStops: [CanonicalPositionedCaretStop]
}

public struct CanonicalPositionedGlyph: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var runIndex: Int
    public var glyphIndex: Int
    public var glyphId: UInt32
    public var clusterUtf16: UInt32
    public var pageXQ26_6: Int32
    public var pageYQ26_6: Int32
    public var xAdvanceQ26_6: Int32
    public var yAdvanceQ26_6: Int32
    public var xOffsetQ26_6: Int32
    public var yOffsetQ26_6: Int32
    public var orientation: CanonicalGlyphOrientation
    public var writingMode: FlowWritingMode
    public var lineIndex: Int
    public var fragmentIndex: Int
}

public struct EditorGeometrySnapshotData: Codable, Equatable, Sendable {
    public var layout: FlowLayout
    public var lines: [CanonicalPositionedLine]
    public var clusters: [CanonicalPositionedCluster]
    public var glyphs: [CanonicalPositionedGlyph]
    public var caretStops: [CanonicalPositionedCaretStop]
    public var caretWidthQ26_6: Int32
}

public struct EditorCaretGeometry: Codable, Equatable, Sendable {
    public var position: CanonicalTextPosition
    public var rect: FlowRect
    public var lineIndex: Int
    public var fragmentIndex: Int
}

public struct EditorCaretMovement: Codable, Equatable, Sendable {
    public var position: CanonicalTextPosition
    public var rect: FlowRect
    public var preferredXQ26_6: Int32
    public var preferredInlineQ26_6: Int32
}

public enum EditorCaretMovementDirection: String, Codable, Sendable {
    case left
    case right
    case up
    case down
}

struct EditorErrorPayload: Codable {
    var code: String
    var message: String
}

struct EditorSnapshotEnvelope: Codable {
    var version: Int
    var snapshot: EditorGeometrySnapshotData?
    var error: EditorErrorPayload?
}

private struct EditorQueryEnvelope: Decodable {
    var version: Int
    var result: EditorQueryResult?
    var error: EditorErrorPayload?
}

private struct EditorQueryResult: Decodable {
    var kind: String
    var caret: EditorCaretGeometry?
    var position: CanonicalTextPosition?
    var selection: CanonicalSelectionGeometry?
    var movement: EditorCaretMovement?
}

private struct CaretQuery: Encodable {
    let version = 2
    let kind = "caret"
    var position: CanonicalTextPosition
}

private struct HitTestQuery: Encodable {
    let version = 2
    let kind = "hitTest"
    var xQ26_6: Int32
    var yQ26_6: Int32
}

private struct SelectionQuery: Encodable {
    let version = 2
    let kind = "selection"
    var paragraphId: String
    var utf16Start: UInt32
    var utf16End: UInt32
}

private struct MoveQuery: Encodable {
    let version = 2
    let kind = "move"
    var position: CanonicalTextPosition
    var direction: EditorCaretMovementDirection
    var preferredInlineQ26_6: Int32?
}

public final class LastDraftEditorSnapshot {
    public let geometry: EditorGeometrySnapshotData
    private var handle: OpaquePointer?

    init(handle: OpaquePointer, geometry: EditorGeometrySnapshotData) {
        self.handle = handle
        self.geometry = geometry
    }

    deinit { dispose() }

    public func caret(at position: CanonicalTextPosition) throws -> EditorCaretGeometry {
        let result = try query(CaretQuery(position: position))
        guard result.kind == "caret", let caret = result.caret else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return caret
    }

    public func hitTest(xQ26_6: Int32, yQ26_6: Int32) throws -> CanonicalTextPosition? {
        let result = try query(HitTestQuery(xQ26_6: xQ26_6, yQ26_6: yQ26_6))
        guard result.kind == "hitTest" else { throw LastDraftFlowEngineError.emptyResponse }
        return result.position
    }

    public func selection(
        paragraphId: String,
        utf16Start: UInt32,
        utf16End: UInt32
    ) throws -> CanonicalSelectionGeometry {
        let result = try query(SelectionQuery(
            paragraphId: paragraphId,
            utf16Start: utf16Start,
            utf16End: utf16End
        ))
        guard result.kind == "selection", let selection = result.selection else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return selection
    }

    public func moveCaret(
        from position: CanonicalTextPosition,
        direction: EditorCaretMovementDirection,
        preferredInlineQ26_6: Int32? = nil
    ) throws -> EditorCaretMovement {
        let result = try query(MoveQuery(
            position: position,
            direction: direction,
            preferredInlineQ26_6: preferredInlineQ26_6
        ))
        guard result.kind == "move", let movement = result.movement else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return movement
    }

    public func dispose() {
        if let handle {
            ld_flow_editor_snapshot_destroy(handle)
            self.handle = nil
        }
    }

    private func query<Query: Encodable>(_ query: Query) throws -> EditorQueryResult {
        guard let handle else {
            throw LastDraftFlowEngineError.engine(code: "snapshot_disposed", message: "Editor geometry snapshot is disposed")
        }
        let input: Data
        do { input = try JSONEncoder().encode(query) }
        catch { throw LastDraftFlowEngineError.encoding(error.localizedDescription) }
        var output = LDFlowBuffer(data: nil, len: 0)
        let status = input.withUnsafeBytes { bytes in
            ld_flow_editor_query_json(
                handle,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count,
                &output
            )
        }
        guard status == LD_FLOW_STATUS_OK else { throw LastDraftFlowEngineError.abiCall(status) }
        let envelope: EditorQueryEnvelope = try decodeEditorOutput(output)
        if let error = envelope.error {
            throw LastDraftFlowEngineError.engine(code: error.code, message: error.message)
        }
        guard let result = envelope.result else { throw LastDraftFlowEngineError.emptyResponse }
        return result
    }
}

func decodeEditorOutput<Value: Decodable>(_ output: LDFlowBuffer) throws -> Value {
    guard let bytes = output.data, output.len > 0 else {
        throw LastDraftFlowEngineError.emptyResponse
    }
    defer { ld_flow_bytes_free(bytes, output.len) }
    do {
        return try JSONDecoder().decode(Value.self, from: Data(bytes: bytes, count: output.len))
    } catch {
        throw LastDraftFlowEngineError.decoding(error.localizedDescription)
    }
}
