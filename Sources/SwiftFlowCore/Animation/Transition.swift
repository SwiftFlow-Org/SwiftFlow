import CSwiftFlow

/// How a view appears and disappears.
///
/// A snapshot of the far side, interpolated toward the view's own values.
public struct Transition: Sendable {

    public struct Phase: Sendable {

        var scale: Float = 1

        var scaleX: Float = 1
        var scaleY: Float = 1

        var cornerRadius: Float = 0

        var opacity: Float = 1

        var offsetX: Float = 0
        var offsetY: Float = 0

        var blur: Float = 0

        static let identity = Phase()

        func merged(with other: Phase) -> Phase {
            Phase(
                scale: scale * other.scale,
                scaleX: scaleX * other.scaleX,
                scaleY: scaleY * other.scaleY,
                cornerRadius: cornerRadius + other.cornerRadius,
                opacity: opacity * other.opacity,
                offsetX: offsetX + other.offsetX,
                offsetY: offsetY + other.offsetY,
                blur: blur + other.blur
            )
        }

        func applied(to base: AnimatableSnapshot) -> AnimatableSnapshot {
            let deviceScale = DeviceScale.current
            var s = base
            s.scaleX *= scale * scaleX
            s.scaleY *= scale * scaleY
            s.cornerRadius += cornerRadius * deviceScale
            s.fillA *= opacity
            s.colorA *= opacity
            s.borderA *= opacity
            s.offsetX += offsetX * deviceScale
            s.offsetY += offsetY * deviceScale
            s.contentBlur += blur * deviceScale
            return s
        }
    }

    public var insertion: Phase
    public var removal: Phase

    init(insertion: Phase, removal: Phase) {
        self.insertion = insertion
        self.removal = removal
    }

    init(_ phase: Phase) {
        self.init(insertion: phase, removal: phase)
    }

    public static let identity = Transition(.identity)

    public static let opacity = Transition(Phase(opacity: 0))

    public static func scale(_ factor: Float = 0.92) -> Transition {
        Transition(Phase(scale: factor, opacity: 0))
    }

    public static func offset(x: Float = 0, y: Float = 0) -> Transition {
        Transition(Phase(offsetX: x, offsetY: y))
    }

    public static let slide = Transition(Phase(opacity: 0, offsetY: 8))

    public static let blurReplace = Transition(Phase(opacity: 0, blur: 10))

    public static func blurReplace(radius: Float) -> Transition {
        Transition(Phase(opacity: 0, blur: radius))
    }

    public func combined(with other: Transition) -> Transition {
        Transition(
            insertion: insertion.merged(with: other.insertion),
            removal: removal.merged(with: other.removal)
        )
    }

    public static func asymmetric(insertion: Transition, removal: Transition) -> Transition {
        Transition(insertion: insertion.insertion, removal: removal.removal)
    }
}

public struct TransitionModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let transition: Transition
}

extension TransitionModifier {
    public func toSFNode() -> SFNode {
        let node = content.toSFNode()
        TransitionRegistry.shared.pending = transition
        return node
    }
}

extension View {

    /// How this view appears and disappears when it is inserted or removed.
    ///
    /// Only takes effect inside a `ForEach`, which is where an insertion can be
    /// told apart from a rebuild.
    public func transition(_ transition: Transition) -> TransitionModifier<Self> {
        TransitionModifier(content: self, transition: transition)
    }
}
