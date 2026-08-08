import CSwiftFlow

/// A font, by size, weight and design.
public struct Font: Sendable {
    public let size: Float
    public var weight: Weight
    public let design: Design

    /// How heavy a font is drawn.
    public enum Weight: Sendable {
        case ultraLight, thin, light, regular, medium
        case semibold, bold, heavy, black

        var numericValue: Float {
            switch self {
            case .ultraLight: return 100
            case .thin: return 200
            case .light: return 300
            case .regular: return 400
            case .medium: return 500
            case .semibold: return 600
            case .bold: return 700
            case .heavy: return 800
            case .black: return 900
            }
        }
    }

    /// Which bundled face a font draws from.
    ///
    /// `.serif` and `.rounded` have no face of their own yet and render as
    /// `.default`.
    public enum Design: Sendable {
        case `default`, monospaced, rounded, serif

        // TODO: no serif or rounded face is bundled — both render as the default.
        var family: SFFontFamily {
            switch self {
            case .default: return SF_FONT_SANS
            case .monospaced: return SF_FONT_MONOSPACED
            case .serif: return SF_FONT_SERIF
            case .rounded: return SF_FONT_ROUNDED
            }
        }
    }

    public static let largeTitle = Font(size: 34, weight: .bold, design: .default)
    public static let title = Font(size: 28, weight: .bold, design: .default)
    public static let title2 = Font(size: 22, weight: .bold, design: .default)
    public static let title3 = Font(size: 20, weight: .semibold, design: .default)
    public static let headline = Font(size: 17, weight: .semibold, design: .default)
    public static let body = Font(size: 17, weight: .regular, design: .default)
    public static let callout = Font(size: 16, weight: .regular, design: .default)
    public static let subheadline = Font(size: 15, weight: .regular, design: .default)
    public static let footnote = Font(size: 13, weight: .regular, design: .default)
    public static let caption = Font(size: 12, weight: .regular, design: .default)
    public static let caption2 = Font(size: 11, weight: .regular, design: .default)

    /// A font of the given size, weight and design.
    public static func system(
        size: Float,
        weight: Weight = .regular,
        design: Design = .default
    ) -> Font {
        Font(size: size, weight: weight, design: design)
    }

    public init(size: Float, weight: Weight = .regular, design: Design = .default) {
        self.size = size
        self.weight = weight
        self.design = design
    }
}

/// A line or paragraph of read-only text.
public struct Text: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: String
    var font: Font = .body
    var color: Color = .primary
    var lineLimit: Int? = nil
    var alignment: TextAlignment = .leading

    public init(_ content: String) {
        self.content = content
    }

    /// Sets the font for this text.
    public func font(_ font: Font) -> Text {
        var copy = self
        copy.font = font
        return copy
    }

    /// Sets the weight of this text.
    public func fontWeight(_ weight: Font.Weight) -> Text {
        var copy = self
        copy.font.weight = weight
        return copy
    }

    /// Sets the colour of this text.
    public func foregroundColor(_ color: Color) -> Text {
        var copy = self
        copy.color = color
        return copy
    }

    public func foregroundStyle(_ color: Color) -> Text {
        foregroundColor(color)
    }

    /// The maximum number of lines this text may wrap to. `nil` is unlimited.
    public func lineLimit(_ limit: Int?) -> Text {
        var copy = self
        copy.lineLimit = limit
        return copy
    }

    /// How wrapped lines sit within this text's box.
    ///
    /// Visible only when the box is wider than a line — a paragraph that
    /// wrapped, or a text given a `.frame(width:)`. Each line is aligned in
    /// turn, so a centred paragraph centres every line.
    public func multilineTextAlignment(_ alignment: TextAlignment) -> Text {
        var copy = self
        copy.alignment = alignment
        return copy
    }
}

/// Where wrapped lines sit inside a text's own box.
///
/// Not the same as `HorizontalAlignment`, which places a view inside its
/// parent. A centred paragraph in a leading-aligned column needs both.
public enum TextAlignment: Sendable {
    case leading, center, trailing

    var sfValue: SFAlignment {
        switch self {
        case .leading: return SF_ALIGNMENT_LEADING
        case .center: return SF_ALIGNMENT_CENTER
        case .trailing: return SF_ALIGNMENT_TRAILING
        }
    }
}

extension Text {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_TEXT
        node.fontSize = font.size
        node.fontWeight = font.weight.numericValue
        node.fontFamily = font.design.family
        node.color = SFColor(r: color.r, g: color.g, b: color.b, a: color.a)
        node.sizing = SF_SIZING_HUG

        node.lineLimit = UInt32(max(0, lineLimit ?? 0))
        node.textAlign = alignment.sfValue

        FrameArena.shared.store(content) { ptr, len in
            node.text = ptr
            node.textLen = Int(len)
        }

        return node
    }
}

public struct FontModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let font: Font
}

extension FontModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.fontSize = font.size
        node.fontWeight = font.weight.numericValue
        node.fontFamily = font.design.family
        return node
    }
}

extension View {
    public func font(_ font: Font) -> FontModifier<Self> {
        FontModifier(content: self, font: font)
    }
}

public struct MultilineTextAlignmentModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let alignment: TextAlignment
}

extension MultilineTextAlignmentModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.textAlign = alignment.sfValue
        return node
    }
}

extension View {
    /// How wrapped lines sit within this view's text box.
    ///
    /// Applies to the view it is attached to and does not reach into a stack's
    /// children — put it on the `Text` itself.
    // TODO: doesn't reach into a stack's children the way SwiftUI's does.
    public func multilineTextAlignment(
        _ alignment: TextAlignment
    ) -> MultilineTextAlignmentModifier<Self> {
        MultilineTextAlignmentModifier(content: self, alignment: alignment)
    }
}

public struct FontWeightModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let weight: Font.Weight
}

extension FontWeightModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.fontWeight = weight.numericValue
        return node
    }
}

extension View {

    public func fontWeight(_ weight: Font.Weight) -> FontWeightModifier<Self> {
        FontWeightModifier(content: self, weight: weight)
    }
}

public struct FontDesignModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let design: Font.Design
}

extension FontDesignModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        node.fontFamily = design.family
        return node
    }
}

extension View {

    public func fontDesign(_ design: Font.Design) -> FontDesignModifier<Self> {
        FontDesignModifier(content: self, design: design)
    }
}
