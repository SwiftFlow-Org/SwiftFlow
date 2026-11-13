import CSwiftFlow

/// A material that refracts what is behind it as well as blurring it.
public struct Glass: Sendable {
    public let blurRadius: Float
    public let tint: Color

    public let refraction: Float

    public let clarity: Float

    public let isInteractive: Bool

    public init(
        blurRadius: Float,
        tint: Color,
        refraction: Float,
        clarity: Float,
        isInteractive: Bool = false
    ) {
        self.blurRadius = blurRadius
        self.tint = tint
        self.refraction = refraction
        self.clarity = clarity
        self.isInteractive = isInteractive
    }

    public static let regular = Glass(
        blurRadius: 0.4,
        tint: Color(r: 1, g: 1, b: 1, a: 0.10),
        refraction: 8,
        clarity: 1.0
    )

    public static let clear = Glass(
        blurRadius: 0.5,
        tint: Color(r: 1, g: 1, b: 1, a: 0.04),
        refraction: 8,
        clarity: 0.85
    )

    /// Tints the glass.
    public func tint(_ color: Color) -> Glass {
        Glass(
            blurRadius: blurRadius,
            tint: color,
            refraction: refraction,
            clarity: clarity,
            isInteractive: isInteractive
        )
    }

    /// Makes the glass respond while it is pressed.
    public func interactive(_ enabled: Bool = true) -> Glass {
        Glass(
            blurRadius: blurRadius,
            tint: tint,
            refraction: refraction,
            clarity: clarity,
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
        backdrop.glassInteractive = pressAmount(fg)
        backdrop.glassClarity = glass.clarity


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

    /// How far into its press the pane is, on a spring rather than a switch,
    /// so the bulge and the brighter rim swell in and settle back out.
    private func pressAmount(_ content: SFNode) -> Float {
        guard glass.isInteractive, content.node_id != 0 else { return 0 }
        let held = NodeRegistry.shared.pressedNodes.contains(content.node_id)
        return GlassPress.shared.amount(for: content.node_id, held: held)
    }
}

/// The press springs behind interactive glass, one per pane that has been
/// touched.
///
/// A press goes in on a quick spring with some give (0.4s, 0.7 damping) and
/// comes back out on a slower, steadier one (0.5s, 0.8), the pair
/// LiquidGlassKit lifts and drops its lens on. The value can run a little
/// past 1 on the way in; the shader takes that as a slightly deeper bend,
/// which is the overshoot that makes it read as liquid.
final class GlassPress {
    nonisolated(unsafe) static let shared = GlassPress()

    private struct Spring {
        var value: Float = 0
        var velocity: Float = 0
        var held = false

        var isSettled: Bool {
            velocity == 0 && value == (held ? 1 : 0)
        }
    }

    private var springs: [UInt32: Spring] = [:]

    var isAnimating: Bool {
        springs.values.contains { !$0.isSettled }
    }

    /// Reads the pane's press for this frame and aims its spring at whether
    /// it is held now.
    func amount(for id: UInt32, held: Bool) -> Float {
        var spring = springs[id] ?? Spring()
        if !held && spring.value == 0 && spring.velocity == 0 {
            springs[id] = nil
            return 0
        }
        spring.held = held
        springs[id] = spring
        return max(spring.value, 0)
    }

    func step(dt: Float) {
        for (id, var spring) in springs where !spring.isSettled {
            let target: Float = spring.held ? 1 : 0
            let (response, damping) = spring.held
                ? (GlassMotion.pressResponse, GlassMotion.pressDamping)
                : (GlassMotion.releaseResponse, GlassMotion.releaseDamping)
            let (x, v) = scalarSpringStep(
                x0: spring.value - target, v0: spring.velocity,
                omega: (2 * Float.pi) / max(response, 0.01),
                zeta: damping, dt: dt
            )
            spring.value = target + x
            spring.velocity = v
            if abs(x) < 0.001 && abs(v) < 0.01 {
                spring.value = target
                spring.velocity = 0
            }
            springs[id] = spring
        }
    }
}

/// The springs glass moves on.
enum GlassMotion {
    static let pressResponse: Float = 0.4
    static let pressDamping: Float = 0.7
    static let releaseResponse: Float = 0.5
    static let releaseDamping: Float = 0.8

    /// The lift a held bar takes, as an `Animation`.
    static let lift = Animation.spring(
        response: Double(pressResponse), dampingFraction: Double(pressDamping)
    )
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
