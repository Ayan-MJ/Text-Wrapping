import CoreGraphics
import Foundation

public let lastDraftLayoutUnitsPerPoint: Int32 = 64

public func flowUnit(_ points: CGFloat) -> Int32 {
    let scaled = (Double(points) * Double(lastDraftLayoutUnitsPerPoint)).rounded()
    return Int32(clamping: Int64(scaled))
}

public func flowPoint(_ unit: Int32) -> CGFloat {
    CGFloat(unit) / CGFloat(lastDraftLayoutUnitsPerPoint)
}

public enum ParagraphBaseDirection: String, Codable, Sendable {
    case auto
    case leftToRight
    case rightToLeft
}

public enum FlowWritingMode: String, Codable, Sendable {
    case horizontalTb
    case verticalRl
    case verticalLr
}

public enum CanonicalTextOrientation: String, Codable, Sendable {
    case mixed
    case upright
    case sideways
}

public enum CanonicalGlyphOrientation: String, Codable, Sendable {
    case upright
    case sideways
}

public struct FlowRect: Codable, Equatable, Sendable {
    public var xQ26_6: Int32
    public var yQ26_6: Int32
    public var widthQ26_6: Int32
    public var heightQ26_6: Int32

    public init(xQ26_6: Int32, yQ26_6: Int32, widthQ26_6: Int32, heightQ26_6: Int32) {
        self.xQ26_6 = xQ26_6
        self.yQ26_6 = yQ26_6
        self.widthQ26_6 = widthQ26_6
        self.heightQ26_6 = heightQ26_6
    }

    public var cgRect: CGRect {
        CGRect(
            x: flowPoint(xQ26_6),
            y: flowPoint(yQ26_6),
            width: flowPoint(widthQ26_6),
            height: flowPoint(heightQ26_6)
        )
    }
}

public struct FlowCluster: Codable, Equatable, Sendable {
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var advanceQ26_6: Int32
    public var canBreakAfter: Bool
    public var isWhitespace: Bool

    public init(
        utf16Start: UInt32,
        utf16End: UInt32,
        advanceQ26_6: Int32,
        canBreakAfter: Bool,
        isWhitespace: Bool = false
    ) {
        self.utf16Start = utf16Start
        self.utf16End = utf16End
        self.advanceQ26_6 = advanceQ26_6
        self.canBreakAfter = canBreakAfter
        self.isWhitespace = isWhitespace
    }
}

public struct FlowParagraphStyle: Codable, Equatable, Sendable {
    public var lineHeightQ26_6: Int32
    public var ascentQ26_6: Int32
    public var spaceAfterQ26_6: Int32

    public init(lineHeightQ26_6: Int32, ascentQ26_6: Int32, spaceAfterQ26_6: Int32 = 0) {
        self.lineHeightQ26_6 = lineHeightQ26_6
        self.ascentQ26_6 = ascentQ26_6
        self.spaceAfterQ26_6 = spaceAfterQ26_6
    }
}

public struct FlowParagraph: Codable, Equatable, Sendable {
    /// LastDraft's existing ProseMirror `attrs.blockId`.
    public var id: String
    public var utf16Length: UInt32
    public var style: FlowParagraphStyle
    public var clusters: [FlowCluster]

    public init(id: String, utf16Length: UInt32, style: FlowParagraphStyle, clusters: [FlowCluster]) {
        self.id = id
        self.utf16Length = utf16Length
        self.style = style
        self.clusters = clusters
    }
}

public enum FlowAnchorAffinity: String, Codable, Sendable {
    case upstream
    case downstream
}

public struct FlowAnchor: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var utf16Offset: UInt32
    public var affinity: FlowAnchorAffinity

    public init(paragraphId: String, utf16Offset: UInt32, affinity: FlowAnchorAffinity = .downstream) {
        self.paragraphId = paragraphId
        self.utf16Offset = utf16Offset
        self.affinity = affinity
    }
}

public struct NormalizedFlowPlacement: Codable, Equatable, Sendable {
    public var inlineU16: UInt16
    public var blockOffset1024: Int32

    public init(inlineU16: UInt16, blockOffset1024: Int32) {
        self.inlineU16 = inlineU16
        self.blockOffset1024 = blockOffset1024
    }
}

public struct FlowObjectSize: Codable, Equatable, Sendable {
    public var idealWidthQ26_6: Int32
    public var idealHeightQ26_6: Int32
    public var maxInlineU16: UInt16

    public init(idealWidthQ26_6: Int32, idealHeightQ26_6: Int32, maxInlineU16: UInt16 = .max) {
        self.idealWidthQ26_6 = idealWidthQ26_6
        self.idealHeightQ26_6 = idealHeightQ26_6
        self.maxInlineU16 = maxInlineU16
    }
}

public struct FlowExclusion: Codable, Equatable, Sendable {
    public let shape: String
    public var marginStartQ26_6: Int32
    public var marginTopQ26_6: Int32
    public var marginEndQ26_6: Int32
    public var marginBottomQ26_6: Int32
    public var minimumFragmentWidthQ26_6: Int32

    public init(
        marginStartQ26_6: Int32 = 0,
        marginTopQ26_6: Int32 = 0,
        marginEndQ26_6: Int32 = 0,
        marginBottomQ26_6: Int32 = 0,
        minimumFragmentWidthQ26_6: Int32
    ) {
        shape = "bounds"
        self.marginStartQ26_6 = marginStartQ26_6
        self.marginTopQ26_6 = marginTopQ26_6
        self.marginEndQ26_6 = marginEndQ26_6
        self.marginBottomQ26_6 = marginBottomQ26_6
        self.minimumFragmentWidthQ26_6 = minimumFragmentWidthQ26_6
    }
}

public struct AnchoredFlowObjectV1: Codable, Equatable, Sendable {
    public let responsivePolicy: String
    public var id: String
    public var anchor: FlowAnchor
    public var placement: NormalizedFlowPlacement
    public var size: FlowObjectSize
    public var exclusion: FlowExclusion

    public init(
        id: String,
        anchor: FlowAnchor,
        placement: NormalizedFlowPlacement,
        size: FlowObjectSize,
        exclusion: FlowExclusion
    ) {
        responsivePolicy = "scale-then-centered-block-v1"
        self.id = id
        self.anchor = anchor
        self.placement = placement
        self.size = size
        self.exclusion = exclusion
    }
}

public struct FlowLayoutRequestV1: Codable, Equatable, Sendable {
    public let version: Int
    public var content: FlowRect
    public var paragraphs: [FlowParagraph]
    public var objects: [AnchoredFlowObjectV1]

    public init(content: FlowRect, paragraphs: [FlowParagraph], objects: [AnchoredFlowObjectV1]) {
        version = 1
        self.content = content
        self.paragraphs = paragraphs
        self.objects = objects
    }
}

public struct FlowFragment: Codable, Equatable, Sendable {
    public var rect: FlowRect
    public var utf16Start: UInt32
    public var utf16End: UInt32
}

public struct FlowLine: Codable, Equatable, Sendable {
    public var topQ26_6: Int32
    public var baselineQ26_6: Int32
    /// Present in editor contract v2; nil for legacy layout-v1 responses.
    public var writingMode: FlowWritingMode?
    public var blockStartQ26_6: Int32?
    public var inlineStartQ26_6: Int32?
    public var fragments: [FlowFragment]
}

public struct FlowParagraphLayout: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var topQ26_6: Int32
    public var bottomQ26_6: Int32
    public var bounds: FlowRect?
    public var baseDirection: CanonicalTextDirection?
    public var writingMode: FlowWritingMode?
    public var lines: [FlowLine]
}

public enum FlowObjectLayoutMode: String, Codable, Sendable {
    case userPositioned
    case blockFallback
}

public struct ResolvedFlowObject: Codable, Equatable, Sendable {
    public var id: String
    public var anchor: FlowAnchor
    public var anchorReferenceTopQ26_6: Int32
    public var anchorReferenceBlockStartQ26_6: Int32?
    public var frame: FlowRect
    public var exclusionFrame: FlowRect
    public var mode: FlowObjectLayoutMode
}

public struct FlowLayout: Codable, Equatable, Sendable {
    public var contentHeightQ26_6: Int32
    public var contentWidthQ26_6: Int32?
    public var paragraphs: [FlowParagraphLayout]
    public var objects: [ResolvedFlowObject]
}

struct FlowErrorPayload: Codable {
    var code: String
    var message: String
}

struct FlowEnvelope: Codable {
    var version: Int
    var layout: FlowLayout?
    var error: FlowErrorPayload?
}

public enum CanonicalTextDirection: String, Codable, Sendable {
    case leftToRight
    case rightToLeft
}

public struct CanonicalFontFeature: Codable, Equatable, Sendable {
    public var tag: String
    public var value: UInt32

    public init(tag: String, value: UInt32) {
        self.tag = tag
        self.value = value
    }
}

public struct CanonicalFontVariation: Codable, Equatable, Sendable {
    public var tag: String
    public var value16_16: Int32

    public init(tag: String, value16_16: Int32) {
        self.tag = tag
        self.value16_16 = value16_16
    }
}

public struct CanonicalRichTextStyle: Codable, Equatable, Sendable {
    public var fontId: String
    public var fontSizeQ26_6: Int32
    /// Deprecated in ABI v4. Rust derives shaping direction from UBA levels.
    public var direction: CanonicalTextDirection?
    public var language: String?
    public var features: [CanonicalFontFeature]
    public var variations: [CanonicalFontVariation]
    public var fillRGBA: UInt32
    public var underline: Bool
    public var strikethrough: Bool

    public init(
        fontId: String,
        fontSizeQ26_6: Int32,
        direction: CanonicalTextDirection? = nil,
        language: String? = nil,
        features: [CanonicalFontFeature] = [],
        variations: [CanonicalFontVariation] = [],
        fillRGBA: UInt32 = 0x000000ff,
        underline: Bool = false,
        strikethrough: Bool = false
    ) {
        self.fontId = fontId
        self.fontSizeQ26_6 = fontSizeQ26_6
        self.direction = direction
        self.language = language
        self.features = features
        self.variations = variations
        self.fillRGBA = fillRGBA
        self.underline = underline
        self.strikethrough = strikethrough
    }
}

public struct CanonicalRichTextRun: Codable, Equatable, Sendable {
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var style: CanonicalRichTextStyle

    public init(utf16Start: UInt32, utf16End: UInt32, style: CanonicalRichTextStyle) {
        self.utf16Start = utf16Start
        self.utf16End = utf16End
        self.style = style
    }
}

public struct CanonicalRichParagraph: Codable, Equatable, Sendable {
    public var id: String
    public var text: String
    public var baseDirection: ParagraphBaseDirection
    public var writingMode: FlowWritingMode
    public var textOrientation: CanonicalTextOrientation
    public var runs: [CanonicalRichTextRun]

    public init(
        id: String,
        text: String,
        baseDirection: ParagraphBaseDirection = .auto,
        writingMode: FlowWritingMode = .horizontalTb,
        textOrientation: CanonicalTextOrientation = .mixed,
        runs: [CanonicalRichTextRun]
    ) {
        self.id = id
        self.text = text
        self.baseDirection = baseDirection
        self.writingMode = writingMode
        self.textOrientation = textOrientation
        self.runs = runs
    }

    private enum CodingKeys: String, CodingKey {
        case id, text, baseDirection, writingMode, textOrientation, runs
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        text = try container.decode(String.self, forKey: .text)
        baseDirection = try container.decodeIfPresent(
            ParagraphBaseDirection.self,
            forKey: .baseDirection
        ) ?? .auto
        writingMode = try container.decodeIfPresent(
            FlowWritingMode.self,
            forKey: .writingMode
        ) ?? .horizontalTb
        textOrientation = try container.decodeIfPresent(
            CanonicalTextOrientation.self,
            forKey: .textOrientation
        ) ?? .mixed
        runs = try container.decode([CanonicalRichTextRun].self, forKey: .runs)
    }
}

public struct CanonicalFontDescriptor: Codable, Equatable, Sendable {
    public var id: String
    public var sha256: String
    public var faceIndex: UInt32
    public var unitsPerEm: UInt16
}

public struct CanonicalShapedGlyph: Codable, Equatable, Sendable {
    public var glyphId: UInt32
    public var clusterUtf16: UInt32
    public var xAdvanceQ26_6: Int32
    public var yAdvanceQ26_6: Int32
    public var xOffsetQ26_6: Int32
    public var yOffsetQ26_6: Int32
}

public struct CanonicalClusterCaretStop: Codable, Equatable, Sendable {
    public var utf16Offset: UInt32
    public var inlineOffsetQ26_6: Int32
}

public struct CanonicalShapedCluster: Codable, Equatable, Sendable {
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var advanceQ26_6: Int32
    public var canBreakAfter: Bool
    public var isWhitespace: Bool
    public var bidiLevel: UInt8
    public var direction: CanonicalTextDirection
    public var orientation: CanonicalGlyphOrientation
    public var caretStops: [CanonicalClusterCaretStop]
}

public struct CanonicalShapedRun: Codable, Equatable, Sendable {
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var font: CanonicalFontDescriptor
    public var fontSizeQ26_6: Int32
    public var bidiLevel: UInt8
    public var direction: CanonicalTextDirection
    public var orientation: CanonicalGlyphOrientation
    public var glyphs: [CanonicalShapedGlyph]
    public var advanceQ26_6: Int32
}

public struct CanonicalShapedParagraph: Codable, Equatable, Sendable {
    public var id: String
    public var utf16Length: UInt32
    public var requestedBaseDirection: ParagraphBaseDirection
    public var baseDirection: CanonicalTextDirection
    public var writingMode: FlowWritingMode
    public var textOrientation: CanonicalTextOrientation
    public var runs: [CanonicalShapedRun]
    public var clusters: [CanonicalShapedCluster]
}

public struct CanonicalTextPosition: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var utf16Offset: UInt32
    public var affinity: FlowAnchorAffinity

    public init(paragraphId: String, utf16Offset: UInt32, affinity: FlowAnchorAffinity) {
        self.paragraphId = paragraphId
        self.utf16Offset = utf16Offset
        self.affinity = affinity
    }
}

public struct CanonicalCaretGeometry: Codable, Equatable, Sendable {
    public var position: CanonicalTextPosition
    public var rect: FlowRect
}

public struct CanonicalSelectionGeometry: Codable, Equatable, Sendable {
    public var paragraphId: String
    public var utf16Start: UInt32
    public var utf16End: UInt32
    public var rects: [FlowRect]
}
