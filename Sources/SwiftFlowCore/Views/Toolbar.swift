import CSwiftFlow

public enum ToolbarItemPlacement: Sendable {

    case topBarLeading

    case topBarTrailing

    case principal

    case automatic
}

extension ToolbarItemPlacement {

    var resolved: ToolbarItemPlacement {
        self == .automatic ? .topBarTrailing : self
    }
}

public struct ResolvedToolbarItem {
    public let placement: ToolbarItemPlacement
    public let node: SFNode

    public let transitions: Bool

    public init(
        placement: ToolbarItemPlacement,
        node: SFNode,
        transitions: Bool = true
    ) {
        self.placement = placement
        self.node = node
        self.transitions = transitions
    }
}

public protocol ToolbarContent {
    func resolveItems() -> [ResolvedToolbarItem]
}

public struct ToolbarItem<Content: View>: ToolbarContent {
    let placement: ToolbarItemPlacement
    let content: Content

    public init(
        placement: ToolbarItemPlacement = .automatic,
        @ViewBuilder content: () -> Content
    ) {
        self.placement = placement
        self.content = content()
    }

    public func resolveItems() -> [ResolvedToolbarItem] {

        [ResolvedToolbarItem(placement: placement.resolved, node: NodeBuilder.buildAny(content))]
    }
}

public struct ToolbarItemGroup<Content: View>: ToolbarContent {
    let placement: ToolbarItemPlacement
    let content: Content

    public init(
        placement: ToolbarItemPlacement = .automatic,
        @ViewBuilder content: () -> Content
    ) {
        self.placement = placement
        self.content = content()
    }

    public func resolveItems() -> [ResolvedToolbarItem] {
        let resolved = placement.resolved
        return buildChildren(content).map {
            ResolvedToolbarItem(placement: resolved, node: $0)
        }
    }
}

@resultBuilder
public struct ToolbarContentBuilder {
    public static func buildExpression<C: ToolbarContent>(_ expression: C) -> [any ToolbarContent] {
        [expression]
    }

    public static func buildBlock(_ components: [any ToolbarContent]...) -> [any ToolbarContent] {
        components.flatMap { $0 }
    }

    public static func buildOptional(_ component: [any ToolbarContent]?) -> [any ToolbarContent] {
        component ?? []
    }

    public static func buildEither(first component: [any ToolbarContent]) -> [any ToolbarContent] {
        component
    }

    public static func buildEither(second component: [any ToolbarContent]) -> [any ToolbarContent] {
        component
    }

    public static func buildArray(_ components: [[any ToolbarContent]]) -> [any ToolbarContent] {
        components.flatMap { $0 }
    }
}

public struct ToolbarModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let items: [any ToolbarContent]
}

extension ToolbarModifier {
    public func toSFNode() -> SFNode {

        for item in items {
            NavigationConfigStore.shared.addItems(item.resolveItems())
        }
        return content.toSFNode()
    }
}

extension View {
    /// Populates the navigation bar above this view.
    public func toolbar(
        @ToolbarContentBuilder content: () -> [any ToolbarContent]
    ) -> ToolbarModifier<Self> {
        ToolbarModifier(content: self, items: content())
    }
}
