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
        node.sizing = SF_SIZING_HUG
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
    return [NodeBuilder.buildAny(view)]
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
