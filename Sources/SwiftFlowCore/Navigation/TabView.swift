import CSwiftFlow

public enum TabRole: Sendable {

    case leading
    case trailing
}

public enum TabReselect {

    case popToRoot

    case scrollToTop

    case run(() -> Void)
}

public protocol TabIcon {
    func resolveIcon(size: Float, color: Color) -> AnyView
}

extension Image: TabIcon {
    public func resolveIcon(size: Float, color: Color) -> AnyView {
        AnyView(
            resizable()
                .aspectRatio(contentMode: .fit)
                .foregroundColor(color)
                .frame(width: size, height: size)
        )
    }
}

extension Icon: TabIcon {

    public func resolveIcon(size defaultSize: Float, color defaultColor: Color) -> AnyView {

        AnyView(
            size(explicitSize ?? defaultSize)
                .foregroundColor(explicitTint ?? defaultColor)
        )
    }
}

/// One tab in a `TabView`.
public struct Tab<Content: View>: TabContent {
    let title: String
    let icon: (any TabIcon)?
    let role: TabRole?
    let reselect: TabReselect?
    let badgeCount: Int?
    let content: () -> Content

    public init(
        _ title: String,
        _ icon: (any TabIcon)? = nil,
        role: TabRole? = nil,
        onReselect: TabReselect? = nil,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.title = title
        self.icon = icon
        self.role = role
        self.reselect = onReselect
        self.badgeCount = nil
        self.content = content
    }

    public init(
        _ title: String,
        _ icon: Icon,
        role: TabRole? = nil,
        onReselect: TabReselect? = nil,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.init(
            title, icon as any TabIcon,
            role: role, onReselect: onReselect, content: content
        )
    }

    private init(
        title: String, icon: (any TabIcon)?, role: TabRole?,
        reselect: TabReselect?, badgeCount: Int?, content: @escaping () -> Content
    ) {
        self.title = title
        self.icon = icon
        self.role = role
        self.reselect = reselect
        self.badgeCount = badgeCount
        self.content = content
    }

    public func badge(_ count: Int) -> Tab {
        Tab(
            title: title, icon: icon, role: role,
            reselect: reselect, badgeCount: count > 0 ? count : nil, content: content
        )
    }

    public func resolveTabs() -> [ResolvedTab] {

        [
            ResolvedTab(
                title: title,
                icon: icon,
                role: role,
                reselect: reselect,
                badge: badgeCount,
                build: { self.content() }
            )
        ]
    }
}

public struct ResolvedTab {
    let title: String
    let icon: (any TabIcon)?

    var index: Int = 0
    let role: TabRole?
    let reselect: TabReselect?
    let badge: Int?

    let build: () -> any View
}

public protocol TabContent {
    func resolveTabs() -> [ResolvedTab]
}

@resultBuilder
public struct TabBuilder {
    public static func buildBlock(_ parts: any TabContent...) -> [ResolvedTab] {
        parts.flatMap { $0.resolveTabs() }
    }

    public static func buildOptional(_ part: [ResolvedTab]?) -> [ResolvedTab] {
        part ?? []
    }

    public static func buildEither(first: [ResolvedTab]) -> [ResolvedTab] { first }

    public static func buildEither(second: [ResolvedTab]) -> [ResolvedTab] { second }
}

enum TabBarMetrics {

    static let barHeight: Float = 64
    static let iconSize: Float = 24
    static let horizontalPadding: Float = 20

    static var contentInset: Float { barHeight + SafeArea.bottom }
}

/// A view that switches between tabs.
///
/// Each tab gets its own `NavigationStack` — don't add one.
public struct TabView: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let tabs: [ResolvedTab]
    let background: Material?
    let navigationBackground: Material?
    let transition: NavTransition
    let screenBackground: Color
    let interactiveBack: Bool

    public init(
        background: Material? = Material(blurRadius: 50, tint: .background, isProgressive: false),
        navigationBackground: Material? = Material(
            blurRadius: 50, tint: .background, isProgressive: true),
        transition: NavTransition = .standard,
        screenBackground: Color = .background,
        interactiveBack: Bool = true,
        @TabBuilder content: () -> [ResolvedTab]
    ) {
        var resolved = content()

        for index in resolved.indices { resolved[index].index = index }
        self.tabs = resolved
        self.background = background
        self.navigationBackground = navigationBackground
        self.transition = transition
        self.screenBackground = screenBackground
        self.interactiveBack = interactiveBack
    }
}

extension TabView {
    public func toSFNode() -> SFNode {

        Navigator.tabTitles = tabs.map(\.title)

        let selected = tabs.indices.contains(Navigator.tabSelection) ? Navigator.tabSelection : 0

        var content: SFNode
        if tabs.indices.contains(selected) {
            let tab = tabs[selected]
            TabBuildContext.shared.begin(tab: selected)

            content = NodeBuilder.buildAny(
                NavStack(
                    stack: Navigator.stack(tab.index),
                    background: navigationBackground,
                    transition: transition,
                    screenBackground: screenBackground,
                    interactiveBack: interactiveBack,
                    content: { AnyView(erasing: tab.build()) }
                )
            )
            TabBuildContext.shared.end()
        } else {
            content = SFNode.makeDefault()
            content.kind = SF_NODE_EMPTY
        }

        let bar = NodeBuilder.buildAny(
            TabBar(tabs: tabs, selected: selected, background: background))

        var children = [content, bar] + PresentationStore.shared.drain()
        let count = children.count

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_DEPTH
        node.sizing = SF_SIZING_FILL
        node.alignment = SF_ALIGNMENT_CENTER

        node.verticalAlignment = SF_ALIGNMENT_TRAILING
        FrameArena.shared.storeNodes(&children) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

struct TabBar: View {
    let tabs: [ResolvedTab]
    let selected: Int
    let background: Material?

    var body: some View {

        let filtered = tabs.filter { $0.role != .leading && $0.role != .trailing }

        HStack(spacing: 12) {

            HStack(spacing: 0) {
                ForEach(filtered.indices, id: \.self) { position in
                    TabBarItem(
                        tab: filtered[position],
                        isSelected: filtered[position].index == selected,
                        showLabel: true,
                        tap: { self.activate(filtered[position]) }
                    )
                    .weight(1)
                    .background(
                        filtered[position].index == selected ?
                            Capsule()
                                .fill(.overlay)
                        : nil
                    )
                }
            }
            .weight(1)
            .padding(.vertical, 4)
            .padding(.horizontal, 4)
            .background(
                Capsule()
                    .fill(.surface)
                    .specular()
            )

            if let searchTab = tabs.first(where: { $0.role == .trailing }) {
                TabBarItem(
                    tab: searchTab,
                    isSelected: searchTab.index == selected,
                    showLabel: false,
                    tap: { self.activate(searchTab) }
                )
                .frame(width: 62, height: 62)
                .background(
                    Circle()
                        .fill(.surface)
                        .specular()
                )
            }
        }
        .padding(.horizontal, TabBarMetrics.horizontalPadding)
        .padding(.bottom, SafeArea.bottom)
        .frame(height: TabBarMetrics.contentInset, maxWidth: .infinity)
    }

    private func activate(_ tab: ResolvedTab) {
        if tab.index == selected {
            reselect(tab)
        } else {
            Navigator.tabSelection = tab.index
        }
    }

    private func reselect(_ tab: ResolvedTab) {
        switch tab.reselect {
        case .none:
            break
        case .popToRoot:
            Navigator.stack(tab.index).popToRoot()
        case .scrollToTop:
            TabBuildContext.shared.scrollToTop(in: tab.index)
        case .run(let action):
            action()
        }
    }
}

struct TabBarItem: View {
    let tab: ResolvedTab
    let isSelected: Bool
    let showLabel: Bool
    let tap: () -> Void

    var body: some View {
        Button(action: tap) {
            VStack(spacing: 2) {
                icon
                if showLabel {
                    Text(tab.title)
                        .font(.system(size: 10, weight: isSelected ? .semibold : .regular))
                        .foregroundColor(isSelected ? .accent : .primary)
                }
            }
            .padding(.vertical, 9.5)
            .padding(.horizontal, 17)
            .offset(y: showLabel ? 0 : 1)
        }
        .buttonStyle(PlainButtonStyle())
    }

    @ViewBuilder
    private var icon: some View {
        if let icon = tab.icon {
            ZStack(alignment: .topTrailing) {
                icon.resolveIcon(
                    size: TabBarMetrics.iconSize,
                    color: isSelected ? .accent : .primary
                )
                .fontWeight(isSelected ? .black : .semibold)
                badge
            }
        }
    }

    @ViewBuilder
    private var badge: some View {
        if let count = tab.badge {
            Text(count > 99 ? "99+" : "\(count)")
                .font(.system(size: 10, weight: .bold))
                .foregroundColor(.primary)
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(Capsule().fill(.accent))
                .offset(x: 10, y: -4)
        }
    }
}

final class TabBuildContext {
    nonisolated(unsafe) static let shared = TabBuildContext()

    nonisolated(unsafe) private var building: Int?
    nonisolated(unsafe) private var scrolls: [Int: UInt32] = [:]

    var isBuildingTab: Bool { building != nil }

    func begin(tab: Int) {
        building = tab
        scrolls[tab] = nil
    }

    func end() {
        building = nil
    }

    func register(_ scrollID: UInt32, axis: Axis) {
        guard let building, axis == .vertical, scrolls[building] == nil else { return }
        scrolls[building] = scrollID
    }

    func scrollToTop(in tab: Int) {
        guard let id = scrolls[tab] else { return }
        NodeRegistry.shared.scrollState(for: id).scrollToTop()
    }
}

extension Image {

    public static func system(_ name: String) -> Image { Image(name) }

    public static var house: Image { .system("house") }
    public static var magnifyingGlass: Image { .system("magnifyingglass") }
    public static var person: Image { .system("person") }
    public static var bell: Image { .system("bell") }
    public static var gear: Image { .system("gear") }
}
