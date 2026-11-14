import CSwiftFlow

public enum HorizontalAlignment: Sendable { case leading, center, trailing }
public enum VerticalAlignment: Sendable { case top, center, bottom }

/// A position in both axes at once.
public struct Alignment: Sendable {
    public let horizontal: HorizontalAlignment
    public let vertical: VerticalAlignment
    public static let center = Alignment(horizontal: .center, vertical: .center)
    public static let top = Alignment(horizontal: .center, vertical: .top)
    public static let bottom = Alignment(horizontal: .center, vertical: .bottom)
    public static let leading = Alignment(horizontal: .leading, vertical: .center)
    public static let trailing = Alignment(horizontal: .trailing, vertical: .center)
    public static let topLeading = Alignment(horizontal: .leading, vertical: .top)
    public static let topTrailing = Alignment(horizontal: .trailing, vertical: .top)
    public static let bottomLeading = Alignment(horizontal: .leading, vertical: .bottom)
    public static let bottomTrailing = Alignment(horizontal: .trailing, vertical: .bottom)
}

/// A view that arranges its children in a column.
public struct VStack<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let alignment: HorizontalAlignment
    let spacing: Float
    let content: Content

    public init(
        alignment: HorizontalAlignment = .center,
        spacing: Float = 8,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.spacing = spacing
        self.content = content()
    }
}

extension VStack {
    public func toSFNode() -> SFNode {
        let scale = DeviceScale.current
        var children = buildChildren(content)
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_VERTICAL
        node.spacing = spacing * scale
        node.sizing = SF_SIZING_HUG
        node.alignment = alignment.toSFAlignment()

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

/// A view that arranges its children in a row.
public struct HStack<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let alignment: VerticalAlignment
    let spacing: Float
    let content: Content

    public init(
        alignment: VerticalAlignment = .center,
        spacing: Float = 8,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.spacing = spacing
        self.content = content()
    }
}

extension HStack {
    public func toSFNode() -> SFNode {
        let scale = DeviceScale.current
        var children = buildChildren(content)

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_HORIZONTAL
        node.spacing = spacing * scale
        node.sizing = SF_SIZING_HUG
        node.alignment = alignment.toSFAlignment()

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

/// A view that overlays its children, aligned in both axes.
public struct ZStack<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let alignment: Alignment
    let content: Content

    public init(
        alignment: Alignment = .center,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.content = content()
    }
}

extension ZStack {
    public func toSFNode() -> SFNode {
        var children = buildChildren(content)

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.spacing = 0
        // As in SwiftUI: flexible on an axis when something in it is (a
        // shape, a colour, a Spacer), so a shape in a framed ZStack fills the
        // frame; hugging its content otherwise.
        node.sizingX = children.contains { $0.sizingX == SF_SIZING_FILL } ? SF_SIZING_FILL : SF_SIZING_HUG
        node.sizingY = children.contains { $0.sizingY == SF_SIZING_FILL } ? SF_SIZING_FILL : SF_SIZING_HUG
        node.alignment = alignment.horizontal.toSFAlignment()
        node.verticalAlignment = alignment.vertical.toSFAlignment()

        let count = children.count
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

func buildChildren(_ view: any View) -> [SFNode] {
    if let multi = view as? any MultiNodeView {
        return multi.buildNodes()
    }
    if let nodes = flattenedBody(view) {
        return nodes
    }
    return [NodeBuilder.buildAny(view)]
}

/// A view whose body is several views (or an `if`): the body's views take
/// their places in the stack directly, as SwiftUI does, rather than the
/// view becoming one node. Nil for any other view.
private func flattenedBody(_ view: any View) -> [SFNode]? {
    func open<V: View>(_ v: V) -> [SFNode]? {
        guard V.Body.self != Never.self else { return nil }
        let body = v.body
        guard body is any ExpandableView else { return nil }
        return NodeBuilder.inOnePlace { buildChildren(body) }
    }
    return open(view)
}

extension HorizontalAlignment {
    func toSFAlignment() -> SFAlignment {
        switch self {
        case .leading: return SF_ALIGNMENT_LEADING
        case .center: return SF_ALIGNMENT_CENTER
        case .trailing: return SF_ALIGNMENT_TRAILING
        }
    }
}

extension VerticalAlignment {
    func toSFAlignment() -> SFAlignment {
        switch self {
        case .top: return SF_ALIGNMENT_LEADING
        case .center: return SF_ALIGNMENT_CENTER
        case .bottom: return SF_ALIGNMENT_TRAILING
        }
    }
}
