import CSwiftFlow

/// One glyph from the bundled icon face.
///
/// Sized to its em box rather than its ink, so a row of icons shares one
/// size and one baseline.
public struct Icon: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    public enum Weight: Float, Sendable {
        case thin = 100
        case light = 300
        case regular = 400
        case bold = 700

        case fill = 900
    }

    let scalar: Unicode.Scalar

    var explicitSize: Float?
    var explicitTint: Color?

    var faceWeight: Weight = .regular

    public var pointSize: Float { explicitSize ?? 24 }
    public var tint: Color { explicitTint ?? .primary }

    public init(scalar: Unicode.Scalar) {
        self.scalar = scalar
    }

    /// Sets the icon's size, in points.
    public func size(_ points: Float) -> Icon {
        var copy = self
        copy.explicitSize = points
        return copy
    }

    /// Picks the icon face, from thin through to filled.
    public func weight(_ weight: Weight) -> Icon {
        var copy = self
        copy.faceWeight = weight
        return copy
    }

    /// Sets the icon's colour.
    public func foregroundColor(_ color: Color) -> Icon {
        var copy = self
        copy.explicitTint = color
        return copy
    }

    public func foregroundStyle(_ color: Color) -> Icon {
        foregroundColor(color)
    }
}

extension Icon {
    public func toSFNode() -> SFNode {
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_ICON
        node.fontSize = pointSize
        node.fontWeight = faceWeight.rawValue

        node.fontFamily = SF_FONT_ICON
        node.color = SFColor(r: tint.r, g: tint.g, b: tint.b, a: tint.a)

        node.sizing = SF_SIZING_HUG

        FrameArena.shared.store(String(Character(scalar))) { ptr, len in
            node.text = ptr
            node.textLen = Int(len)
        }

        return node
    }
}
