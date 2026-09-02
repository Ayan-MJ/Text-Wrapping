import CLastDraftFlow
import Foundation

private struct CanonicalErrorPayload: Codable {
    var code: String
    var message: String
}

private struct CanonicalFontEnvelope: Codable {
    var version: Int
    var font: CanonicalFontDescriptor?
    var error: CanonicalErrorPayload?
}

private struct CanonicalShapingEnvelope: Codable {
    var version: Int
    var paragraph: CanonicalShapedParagraph?
    var error: CanonicalErrorPayload?
}

/// Owns one shared-engine font registry. Calls must be serialized by the host.
public final class LastDraftCanonicalShaper {
    private let handle: OpaquePointer

    public init() throws {
        guard let handle = ld_flow_shaper_create() else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        self.handle = handle
    }

    deinit {
        ld_flow_shaper_destroy(handle)
    }

    public func registerFont(
        id: String,
        data: Data,
        faceIndex: UInt32 = 0
    ) throws -> CanonicalFontDescriptor {
        let idData = Data(id.utf8)
        var output = LDFlowBuffer(data: nil, len: 0)
        let status = idData.withUnsafeBytes { idBytes in
            data.withUnsafeBytes { fontBytes in
                ld_flow_shaper_register_font(
                    handle,
                    idBytes.bindMemory(to: UInt8.self).baseAddress,
                    idBytes.count,
                    fontBytes.bindMemory(to: UInt8.self).baseAddress,
                    fontBytes.count,
                    faceIndex,
                    &output
                )
            }
        }
        guard status == LD_FLOW_STATUS_OK else {
            throw LastDraftFlowEngineError.abiCall(status)
        }
        let envelope: CanonicalFontEnvelope = try decode(output)
        if let error = envelope.error {
            throw LastDraftFlowEngineError.engine(code: error.code, message: error.message)
        }
        guard let font = envelope.font else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return font
    }

    public func shape(_ paragraph: CanonicalRichParagraph) throws -> CanonicalShapedParagraph {
        let input: Data
        do {
            input = try JSONEncoder().encode(paragraph)
        } catch {
            throw LastDraftFlowEngineError.encoding(error.localizedDescription)
        }
        var output = LDFlowBuffer(data: nil, len: 0)
        let status = input.withUnsafeBytes { bytes in
            ld_flow_shaper_shape_json(
                handle,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count,
                &output
            )
        }
        guard status == LD_FLOW_STATUS_OK else {
            throw LastDraftFlowEngineError.abiCall(status)
        }
        let envelope: CanonicalShapingEnvelope = try decode(output)
        if let error = envelope.error {
            throw LastDraftFlowEngineError.engine(code: error.code, message: error.message)
        }
        guard let shaped = envelope.paragraph else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return shaped
    }

    public func createEditorSnapshot(
        _ request: EditorSnapshotRequestV1
    ) throws -> LastDraftEditorSnapshot {
        try createEditorSnapshotEncoded(request)
    }

    public func createEditorSnapshot(
        _ request: EditorSnapshotRequestV2
    ) throws -> LastDraftEditorSnapshot {
        try createEditorSnapshotEncoded(request)
    }

    private func createEditorSnapshotEncoded<Request: Encodable>(
        _ request: Request
    ) throws -> LastDraftEditorSnapshot {
        let input: Data
        do {
            input = try JSONEncoder().encode(request)
        } catch {
            throw LastDraftFlowEngineError.encoding(error.localizedDescription)
        }
        var snapshot: OpaquePointer?
        var output = LDFlowBuffer(data: nil, len: 0)
        let status = input.withUnsafeBytes { bytes in
            ld_flow_editor_snapshot_create_json(
                handle,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count,
                &snapshot,
                &output
            )
        }
        guard status == LD_FLOW_STATUS_OK else {
            throw LastDraftFlowEngineError.abiCall(status)
        }
        do {
            let envelope: EditorSnapshotEnvelope = try decodeEditorOutput(output)
            if let error = envelope.error {
                throw LastDraftFlowEngineError.engine(code: error.code, message: error.message)
            }
            guard let handle = snapshot, let geometry = envelope.snapshot else {
                throw LastDraftFlowEngineError.emptyResponse
            }
            return LastDraftEditorSnapshot(handle: handle, geometry: geometry)
        } catch {
            if let snapshot { ld_flow_editor_snapshot_destroy(snapshot) }
            throw error
        }
    }

    private func decode<Value: Decodable>(_ output: LDFlowBuffer) throws -> Value {
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
}
