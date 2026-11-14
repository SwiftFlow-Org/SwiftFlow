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

    /// The views in the tuple, one by one, with any tuple, `if` or
    /// optional among them opened up in turn.
    func childViews() -> [any View] {
        var views: [any View] = []
        repeat views.append(contentsOf: expandedViews(each value))
        return views
    }

    /// Several views where one node is wanted (a modifier's content, a
    /// body built on its own): stacked over each other, as a ZStack would.
    public func toSFNode() -> SFNode {
        NodeListView(nodes: buildNodes()).toSFNode()
    }
}

/// `view` as the views it stands for: a tuple, `if` or optional opened up
/// into its views (none for an empty one), anything else as itself.
func expandedViews(_ view: any View) -> [any View] {
    switch view {
    case let tuple as any ExpandableView: return tuple.childViews()
    default: return [view]
    }
}

/// A view that stands for some number of other views rather than being
/// one: a tuple of views, or an `if` or optional around one.
protocol ExpandableView {
    func childViews() -> [any View]
}

extension TupleView: ExpandableView {}

extension ConditionalView: ExpandableView {
    func childViews() -> [any View] {
        switch self {
        case .trueContent(let v): return expandedViews(v)
        case .falseContent(let v): return expandedViews(v)
        }
    }
}

extension Optional: ExpandableView where Wrapped: View {
    func childViews() -> [any View] {
        switch self {
        case .some(let v): return expandedViews(v)
        case .none: return []
        }
    }
}

/// An `if` or optional among a stack's children: its views take their
/// places in the stack directly, so a branch of several views shows them
/// all and an empty one takes no room (nor spacing). It keeps one place in
/// the build's numbering whatever it holds, so the views after it keep
/// their identity when it changes.
extension ConditionalView: MultiNodeView {
    func buildNodes() -> [SFNode] {
        switch self {
        case .trueContent(let v): return NodeBuilder.inOnePlace { buildChildren(v) }
        case .falseContent(let v): return NodeBuilder.inOnePlace { buildChildren(v) }
        }
    }
}

extension Optional: MultiNodeView where Wrapped: View {
    func buildNodes() -> [SFNode] {
        switch self {
        case .some(let v): return NodeBuilder.inOnePlace { buildChildren(v) }
        case .none:
            NodeBuilder.inOnePlace {}
            return []
        }
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
        // As a ZStack: filling on an axis only when something in it fills.
        node.sizingX = nodes.contains { $0.sizingX == SF_SIZING_FILL } ? SF_SIZING_FILL : SF_SIZING_HUG
        node.sizingY = nodes.contains { $0.sizingY == SF_SIZING_FILL } ? SF_SIZING_FILL : SF_SIZING_HUG

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
