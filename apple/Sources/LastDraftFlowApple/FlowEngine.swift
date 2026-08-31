import CLastDraftFlow
import Foundation

public enum LastDraftFlowEngineError: Error, Equatable, LocalizedError {
    case abiVersion(expected: UInt32, actual: UInt32)
    case encoding(String)
    case abiCall(Int32)
    case emptyResponse
    case engine(code: String, message: String)
    case decoding(String)

    public var errorDescription: String? {
        switch self {
        case let .abiVersion(expected, actual):
            "LastDraft Flow ABI mismatch: expected \(expected), received \(actual)"
        case let .encoding(message), let .decoding(message):
            message
        case let .abiCall(status):
            "LastDraft Flow ABI returned status \(status)"
        case .emptyResponse:
            "LastDraft Flow returned an empty response"
        case let .engine(_, message):
            message
        }
    }
}

/// Thin Swift owner for the shared Rust engine. It contains no layout policy.
public struct LastDraftFlowEngine: Sendable {
    public static let abiVersion: UInt32 = 1

    public init() throws {
        let actual = ld_flow_abi_version()
        guard actual == Self.abiVersion else {
            throw LastDraftFlowEngineError.abiVersion(expected: Self.abiVersion, actual: actual)
        }
    }

    public func layout(_ request: FlowLayoutRequestV1) throws -> FlowLayout {
        let input: Data
        do {
            input = try JSONEncoder().encode(request)
        } catch {
            throw LastDraftFlowEngineError.encoding(error.localizedDescription)
        }

        var output = LDFlowBuffer(data: nil, len: 0)
        let status = input.withUnsafeBytes { bytes in
            ld_flow_layout_json(
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count,
                &output
            )
        }
        guard status == LD_FLOW_STATUS_OK else {
            throw LastDraftFlowEngineError.abiCall(status)
        }
        guard let outputData = output.data, output.len > 0 else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        defer { ld_flow_bytes_free(outputData, output.len) }

        let response = Data(bytes: outputData, count: output.len)
        let envelope: FlowEnvelope
        do {
            envelope = try JSONDecoder().decode(FlowEnvelope.self, from: response)
        } catch {
            throw LastDraftFlowEngineError.decoding(error.localizedDescription)
        }
        if let error = envelope.error {
            throw LastDraftFlowEngineError.engine(code: error.code, message: error.message)
        }
        guard let layout = envelope.layout else {
            throw LastDraftFlowEngineError.emptyResponse
        }
        return layout
    }

    public func normalizedPlacementForDrag(
        contentXQ26_6: Int32,
        contentWidthQ26_6: Int32,
        objectWidthQ26_6: Int32,
        anchorReferenceTopQ26_6: Int32,
        lineHeightQ26_6: Int32,
        proposedXQ26_6: Int32,
        proposedYQ26_6: Int32
    ) throws -> NormalizedFlowPlacement {
        var inline: UInt16 = 0
        var block: Int32 = 0
        let status = ld_flow_normalized_placement_for_drag(
            contentXQ26_6,
            contentWidthQ26_6,
            objectWidthQ26_6,
            anchorReferenceTopQ26_6,
            lineHeightQ26_6,
            proposedXQ26_6,
            proposedYQ26_6,
            &inline,
            &block
        )
        guard status == LD_FLOW_STATUS_OK else {
            throw LastDraftFlowEngineError.abiCall(status)
        }
        return NormalizedFlowPlacement(inlineU16: inline, blockOffset1024: block)
    }
}

