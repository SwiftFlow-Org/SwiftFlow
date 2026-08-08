import CSwiftFlow

@resultBuilder
public struct ViewBuilder {
    public static func buildBlock() -> EmptyView {
        EmptyView()
    }

    public static func buildBlock<C: View>(_ content: C) -> C {
        content
    }

    public static func buildBlock<each C: View>(
        _ component: repeat each C
    ) -> TupleView<repeat each C> {
        TupleView((repeat each component))
    }

    public static func buildIf<C: View>(_ content: C?) -> C? {
        content
    }

    public static func buildEither<T: View, F: View>(
        first: T
    ) -> ConditionalView<T, F> {
        .trueContent(first)
    }

    public static func buildEither<T: View, F: View>(
        second: F
    ) -> ConditionalView<T, F> {
        .falseContent(second)
    }
}

/// A view that draws nothing.
public struct EmptyView: PrimitiveView {
    public typealias Body = Swift.Never

    public init() {}
}

public struct TupleView<each T: View>: PrimitiveView {
    public var value: (repeat each T)
    public typealias Body = Swift.Never
    public init(_ value: (repeat each T)) { self.value = value }
}

protocol MultiNodeView {
    func buildNodes() -> [SFNode]
}

extension TupleView: MultiNodeView {
    func buildNodes() -> [SFNode] {

        var groups: [[SFNode]] = []
        repeat groups.append(buildChildren(each value))
        return groups.flatMap { $0 }
    }
}

struct NodeListView: View {
    typealias Body = Swift.Never
    var body: Swift.Never { fatalError() }

    let nodes: [SFNode]

    func toSFNode() -> SFNode {

        if nodes.count == 1 { return nodes[0] }
        var kids = nodes
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.sizing = SF_SIZING_FILL

        let count = kids.count
        FrameArena.shared.storeNodes(&kids) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

extension NodeListView: MultiNodeView {
    func buildNodes() -> [SFNode] { nodes }
}

public enum ConditionalView<TrueContent: View, FalseContent: View>: PrimitiveView {
    case trueContent(TrueContent)
    case falseContent(FalseContent)
    public typealias Body = Swift.Never
}

extension ConditionalView {

    public func toSFNode() -> SFNode {
        switch self {
        case .trueContent(let view): return view.toSFNode()
        case .falseContent(let view): return view.toSFNode()
        }
    }
}

extension Optional: View where Wrapped: View {
    public typealias Body = Swift.Never
    public var body: Swift.Never { fatalError() }

    public func toSFNode() -> SFNode {
        switch self {
        case .some(let view):
            return view.toSFNode()
        case .none:
            var node = SFNode.makeDefault()
            node.kind = SF_NODE_EMPTY
            return node
        }
    }
}
