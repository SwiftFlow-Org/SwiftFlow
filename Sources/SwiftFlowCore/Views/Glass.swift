import CSwiftFlow

/// A material that refracts what is behind it as well as blurring it.
public struct Glass: Sendable {
    public let blurRadius: Float
    public let tint: Color

    public let refraction: Float

    public let isInteractive: Bool

    public init(
        blurRadius: Float,
        tint: Color,
        refraction: Float,
        isInteractive: Bool = false
    ) {
        self.blurRadius = blurRadius
        self.tint = tint
        self.refraction = refraction
        self.isInteractive = isInteractive
    }

    public static let regular = Glass(
        blurRadius: 24,
        tint: Color(r: 1, g: 1, b: 1, a: 0.10),
        refraction: 18
    )

    public static let clear = Glass(
        blurRadius: 0.5,
        tint: Color(r: 1, g: 1, b: 1, a: 0.04),
        refraction: 18
    )

    /// Tints the glass.
    public func tint(_ color: Color) -> Glass {
        Glass(
            blurRadius: blurRadius,
            tint: color,
            refraction: refraction,
            isInteractive: isInteractive
        )
    }

    /// Makes the glass respond while it is pressed.
    public func interactive(_ enabled: Bool = true) -> Glass {
        Glass(
            blurRadius: blurRadius,
            tint: tint,
            refraction: refraction,
            isInteractive: enabled
        )
    }
}

public struct GlassEffectModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let glass: Glass

    let cornerRadius: Float?
}

extension GlassEffectModifier {
    public func toSFNode() -> SFNode {

        let fg = content.toSFNode()

        var backdrop = SFNode.makeDefault()
        backdrop.kind = SF_NODE_RECT
        backdrop.fill = glass.tint.toSFColor()
        backdrop.blurRadius = glass.blurRadius * DeviceScale.current

        backdrop.glassRefraction = glass.refraction * DeviceScale.current
        backdrop.glassInteractive = isPressed(fg) ? 1 : 0

        backdrop.specular = 1
        backdrop.cornerRadius = cornerRadius ?? 0
        backdrop.sizing = SF_SIZING_FILL

        var children = [backdrop, fg]
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.sizingX = fg.sizingX
        node.sizingY = fg.sizingY

        node.fixedWidth = fg.fixedWidth
        node.fixedHeight = fg.fixedHeight

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }

    private func isPressed(_ content: SFNode) -> Bool {
        guard glass.isInteractive, content.node_id != 0 else { return false }
        return NodeRegistry.shared.pressedNodes.contains(content.node_id)
    }
}

extension View {

    /// Puts this view on a pane of Liquid Glass.
    public func glassEffect(_ glass: Glass = .regular) -> GlassEffectModifier<Self> {
        GlassEffectModifier(content: self, glass: glass, cornerRadius: nil)
    }

    /// Puts this view on a pane of Liquid Glass with the given shape.
    public func glassEffect<S: View>(
        _ glass: Glass = .regular,
        in shape: S
    ) -> GlassEffectModifier<Self> {
        GlassEffectModifier(
            content: self,
            glass: glass,
            cornerRadius: shapeCornerRadius(shape)
        )
    }
}
