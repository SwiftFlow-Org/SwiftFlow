import CSwiftFlow

public struct MatchedTransitionSource<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let identity: UInt32

    public func toSFNode() -> SFNode {
        NodeFrames.shared.register(identity)
        return content.toSFNode()
    }
}

extension MatchedTransitionSource: ExplicitlyIdentifiedView {
    var explicitIdentity: UInt32 { identity }
}

extension View {

    /// Marks this view as the thing a sheet grows out of.
    ///
    /// Pass the same id to `Navigator.present(_:as:morphingFrom:)`.
    public func matchedTransitionSource(id: String) -> MatchedTransitionSource<Self> {
        MatchedTransitionSource(
            content: self,
            identity: NodeFrames.id(for: id)
        )
    }
}
