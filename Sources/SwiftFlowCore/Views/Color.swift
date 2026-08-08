import CSwiftFlow

/// A colour.
public struct Color: Sendable {
    public let r: Float
    public let g: Float
    public let b: Float
    public let a: Float

    public init(r: Float, g: Float, b: Float, a: Float = 1.0) {
        self.r = r
        self.g = g
        self.b = b
        self.a = a
    }

    public init(hex: UInt32, alpha: Float = 1.0) {
        self.r = Float((hex >> 16) & 0xFF) / 255.0
        self.g = Float((hex >> 8) & 0xFF) / 255.0
        self.b = Float((hex) & 0xFF) / 255.0
        self.a = alpha
    }

    /// Multiplies this colour's alpha.
    public func opacity(_ value: Float) -> Color {
        Color(r: r, g: g, b: b, a: a * value)
    }

    public static let clear = Color(r: 0, g: 0, b: 0, a: 0)
    public static let black = Color(r: 0, g: 0, b: 0)
    public static let white = Color(r: 1, g: 1, b: 1)
    public static let blue = Color(r: 0, g: 0, b: 1)
    public static let surface = Color(hex: 0x211D16)
    public static let overlay = Color(hex: 0x342E25)
    public static let border = Color(hex: 0x3b3b3b)
    public static let background = Color(hex: 0x16140F)
    public static let primary = Color(hex: 0xF6F1E9)
    public static let secondary = Color(hex: 0x6E675C)
    public static let accent = Color(hex: 0xC15B3A)
}

extension Color {

    public static let tertiary = Color(hex: 0x4F4941)

    public static let placeholder = Color(hex: 0x5A5349)

    public static let fill = Color(hex: 0x2B2620)

    public static let separator = Color(hex: 0x2E2A23)

    public static let scrim = Color(r: 0, g: 0, b: 0, a: 0.45)

    public static let destructive = Color.red
    public static let success = Color.green
    public static let warning = Color.yellow

    public static let red = Color(hex: 0xD1524A)
    public static let orange = Color(hex: 0xD9853F)

    public static let yellow = Color(hex: 0xD9B441)
    public static let green = Color(hex: 0x5E9E5A)
    public static let mint = Color(hex: 0x5CB08F)
    public static let teal = Color(hex: 0x489B9B)
    public static let cyan = Color(hex: 0x4E9BC4)
    public static let indigo = Color(hex: 0x6E72C4)
    public static let purple = Color(hex: 0x9A6BC0)
    public static let pink = Color(hex: 0xC96A94)
    public static let brown = Color(hex: 0x8A6B4A)

    public static let gray = Color(hex: 0x8A8478)
}

extension Color: PrimitiveView {
    public typealias Body = Swift.Never
}

extension Color {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_RECT
        node.fill = SFColor(r: r, g: g, b: b, a: a)
        node.sizing = SF_SIZING_FILL
        return node
    }
}
