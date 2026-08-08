import CSwiftFlow

struct AnyEquatableBox {
    private let isEqualTo: (AnyEquatableBox) -> Bool
    private let value: Any

    init<V: Equatable>(_ value: V) {
        self.value = value
        self.isEqualTo = { other in
            guard let otherValue = other.value as? V else { return false }
            return otherValue == value
        }
    }

    static func == (lhs: AnyEquatableBox, rhs: AnyEquatableBox) -> Bool {
        lhs.isEqualTo(rhs)
    }
}

public struct AnimationModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let curve: Animation
    let animationID: UInt32

    let observing: AnyEquatableBox?

    public init(
        content: Content, curve: Animation,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) {
        self.content = content
        self.curve = curve
        self.observing = nil
        self.animationID = fnv1a("\(fileID):\(line):\(column)")
    }

    init(
        content: Content, curve: Animation, observing: AnyEquatableBox,
        fileID: String, line: Int, column: Int
    ) {
        self.content = content
        self.curve = curve
        self.observing = observing
        self.animationID = fnv1a("\(fileID):\(line):\(column)")
    }

    init(content: Content, curve: Animation, explicitID: UInt32) {
        self.content = content
        self.curve = curve
        self.observing = nil
        self.animationID = explicitID
    }
}

extension AnimationModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let targetSnapshot = AnimatableSnapshot.extract(from: node)
        let state = NodeRegistry.shared.animationState(for: animationID)

        if let observing {

            state.retargetIfObservedChanged(
                to: targetSnapshot, curve: curve, observed: observing
            )
        } else {
            state.retarget(to: targetSnapshot, curve: curve)
        }
        state.current.apply(to: &node)

        AnimationTransaction.markExplicitlyAnimated()
        return node
    }
}

extension View {

    /// Animates every change to this view.
    ///
    /// Keyed by the call site, so a modifier that moves between builds starts a
    /// new animation. Use the `id:` form to keep one across a move.
    public func animation(
        _ animation: Animation = .default,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) -> AnimationModifier<Self> {
        AnimationModifier(content: self, curve: animation, fileID: fileID, line: line, column: column)
    }

    public func animation(_ animation: Animation = .default, id explicitID: UInt32) -> AnimationModifier<Self> {
        AnimationModifier(content: self, curve: animation, explicitID: explicitID)
    }

    /// Animates this view only when `value` changes.
    public func animation<V: Equatable>(
        _ animation: Animation = .default,
        value: V,
        fileID: String = #fileID, line: Int = #line, column: Int = #column
    ) -> AnimationModifier<Self> {
        AnimationModifier(
            content: self,
            curve: animation,
            observing: AnyEquatableBox(value),
            fileID: fileID, line: line, column: column
        )
    }
}
