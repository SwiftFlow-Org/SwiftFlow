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
        let press = pressState(fg)
        backdrop.glassInteractive = press.amount
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

        // The drag stretch goes on the stack, so the content inside the pane
        // stretches with the glass rather than sliding about on top of it.
        let scale = DeviceScale.current
        node.offsetX += press.offset.x * scale
        node.offsetY += press.offset.y * scale
        node.scale.x *= press.scale.x
        node.scale.y *= press.scale.y

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }

    /// How far into its press the pane is, and how far a drag has pulled
    /// it, all on springs rather than switches, so the bulge swells in, the
    /// pane follows the finger, and both settle back out.
    private func pressState(_ content: SFNode) -> GlassPressState {
        guard glass.isInteractive, content.node_id != 0 else { return .rest }
        let id = content.node_id
        let held = NodeRegistry.shared.pressedNodes.contains(id)
        return GlassPress.shared.state(
            for: id,
            held: held,
            translation: held ? GestureRouter.shared.pressTranslation(for: id) : nil
        )
    }
}

/// An interactive pane's press for one frame.
struct GlassPressState {
    /// 0 at rest, 1 held; a little past 1 on the spring's way in.
    var amount: Float
    /// How far the pane leans toward the finger, in points.
    var offset: Point
    /// How far it is stretched along each axis.
    var scale: Point

    static let rest = GlassPressState(amount: 0, offset: .zero, scale: Point(x: 1, y: 1))
}

/// The press springs behind interactive glass, one per pane that has been
/// touched.
///
/// A press goes in on a quick spring with some give (0.4s, 0.7 damping) and
/// comes back out on a slower, steadier one (0.5s, 0.8), the pair
/// LiquidGlassKit lifts and drops its lens on. The value can run a little
/// past 1 on the way in; the shader takes that as a slightly deeper bend,
/// which is the overshoot that makes it read as liquid.
///
/// Dragging a held pane pulls it after the finger. The pull is a rubber
/// band: it grows with the square root of the drag, as LiquidGlassKit's
/// switch and slider do past their ends, so it gives readily at first and
/// then resists. The pane leans three quarters of the pull toward the finger
/// and grows by the whole pull along the drag, losing two thirds of it
/// across: LiquidGlassKit's slider-track ratios, doubled, because at theirs
/// a pane the size of a toolbar barely changes shape. Let go and the pull
/// springs back on an underdamped spring, so the pane overshoots its rest
/// once and wobbles in.
final class GlassPress {
    nonisolated(unsafe) static let shared = GlassPress()

    private struct Spring {
        var value: Float = 0
        var velocity: Float = 0
        var held = false

        /// The finger's travel as the pane has caught up with it, in points.
        var drag = Point.zero
        var dragVelocity = Point.zero
        var dragTarget = Point.zero

        /// The pane's size when the press began. Read once, because the
        /// stretch would otherwise measure itself.
        var size: Point?

        var isSettled: Bool {
            velocity == 0 && value == (held ? 1 : 0)
                && dragVelocity == .zero && drag == dragTarget
        }
    }

    private var springs: [UInt32: Spring] = [:]

    var isAnimating: Bool {
        springs.values.contains { !$0.isSettled }
    }

    /// Whether `id` has a spring, so a finger moving on it needs frames.
    func tracks(_ id: UInt32) -> Bool {
        springs[id] != nil
    }

    /// Reads the pane's press for this frame and aims its springs at whether
    /// it is held now and where the finger has got to.
    func state(for id: UInt32, held: Bool, translation: Point?) -> GlassPressState {
        var spring = springs[id] ?? Spring()
        if !held && spring.value == 0 && spring.velocity == 0
            && spring.drag == .zero && spring.dragVelocity == .zero {
            springs[id] = nil
            return .rest
        }
        if held && spring.size == nil,
           let frame = NodeFrames.shared.frame(for: id)?.inPoints(),
           frame.width > 0, frame.height > 0 {
            spring.size = Point(x: frame.width, y: frame.height)
        }
        spring.held = held
        spring.dragTarget = held ? (translation ?? .zero) : .zero
        springs[id] = spring
        return Self.geometry(of: spring)
    }

    func step(dt: Float) {
        for (id, var spring) in springs where !spring.isSettled {
            let target: Float = spring.held ? 1 : 0
            let (response, damping) = spring.held
                ? (GlassMotion.pressResponse, GlassMotion.pressDamping)
                : (GlassMotion.releaseResponse, GlassMotion.releaseDamping)
            let (x, v) = scalarSpringStep(
                x0: spring.value - target, v0: spring.velocity,
                omega: omega(response), zeta: damping, dt: dt
            )
            spring.value = target + x
            spring.velocity = v
            if abs(x) < 0.001 && abs(v) < 0.01 {
                spring.value = target
                spring.velocity = 0
            }

            // Held, the pane keeps close behind the finger; let go, it
            // springs home loosely enough to overshoot.
            let (dragResponse, dragDamping) = spring.held
                ? (GlassMotion.followResponse, GlassMotion.followDamping)
                : (GlassMotion.snapBackResponse, GlassMotion.snapBackDamping)
            let (dx, vx) = scalarSpringStep(
                x0: spring.drag.x - spring.dragTarget.x, v0: spring.dragVelocity.x,
                omega: omega(dragResponse), zeta: dragDamping, dt: dt
            )
            let (dy, vy) = scalarSpringStep(
                x0: spring.drag.y - spring.dragTarget.y, v0: spring.dragVelocity.y,
                omega: omega(dragResponse), zeta: dragDamping, dt: dt
            )
            spring.drag = Point(x: spring.dragTarget.x + dx, y: spring.dragTarget.y + dy)
            spring.dragVelocity = Point(x: vx, y: vy)
            if abs(dx) < 0.05 && abs(dy) < 0.05 && abs(vx) < 0.5 && abs(vy) < 0.5 {
                spring.drag = spring.dragTarget
                spring.dragVelocity = .zero
            }
            springs[id] = spring
        }
    }

    private func omega(_ response: Float) -> Float {
        (2 * Float.pi) / max(response, 0.01)
    }

    /// The lean and stretch a pull of `drag` points gives a pane.
    private static func geometry(of spring: Spring) -> GlassPressState {
        let pull = Point(x: rubberBand(spring.drag.x), y: rubberBand(spring.drag.y))
        var scale = Point(x: 1, y: 1)
        if let size = spring.size {
            let along = Point(x: abs(pull.x), y: abs(pull.y))
            let width = size.x + along.x * GlassMotion.dragGrowth - along.y * GlassMotion.dragThinning
            let height = size.y + along.y * GlassMotion.dragGrowth - along.x * GlassMotion.dragThinning
            scale = Point(
                x: max(width / size.x, GlassMotion.minimumDragScale),
                y: max(height / size.y, GlassMotion.minimumDragScale)
            )
        }
        return GlassPressState(
            amount: max(spring.value, 0),
            offset: Point(x: pull.x * GlassMotion.dragLean, y: pull.y * GlassMotion.dragLean),
            scale: scale
        )
    }

    /// LiquidGlassKit's square-root rubber band, eased through zero so a
    /// pane springing back across its rest has no corner in its motion.
    private static func rubberBand(_ travel: Float) -> Float {
        let reach = GlassMotion.dragReach
        let magnitude = reach * ((abs(travel) + 1).squareRoot() - 1)
        return travel < 0 ? -magnitude : magnitude
    }
}

/// The springs glass moves on.
enum GlassMotion {
    static let pressResponse: Float = 0.4
    static let pressDamping: Float = 0.7
    static let releaseResponse: Float = 0.5
    static let releaseDamping: Float = 0.8

    /// How tightly a held pane follows the finger. Touch events arrive in
    /// steps; this smooths them without reading as lag.
    static let followResponse: Float = 0.12
    static let followDamping: Float = 0.9

    /// The spring a pane comes home on when let go: loose enough to swing
    /// past rest once.
    static let snapBackResponse: Float = 0.45
    static let snapBackDamping: Float = 0.45

    /// Points of pull per square-root point of drag: a 25pt drag pulls
    /// about 5pt, 100pt about 12pt.
    static let dragReach: Float = 1.25

    /// Of the pull, how much moves the pane and how much stretches it along
    /// the drag and thins it across.
    static let dragLean: Float = 0.75
    static let dragGrowth: Float = 1.0
    static let dragThinning: Float = 2.0 / 3.0

    /// The narrowest a pane gets across a long drag.
    static let minimumDragScale: Float = 0.7

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
