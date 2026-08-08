import CSwiftFlow

public struct AnyView: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let node: SFNode

    public init<V: View>(_ view: V) {
        self.node = view.toSFNode()
    }

    public init(erasing view: any View) {
        self.node = NodeBuilder.buildAny(view)
    }
}

extension AnyView {
    public func toSFNode() -> SFNode {
        node
    }
}
