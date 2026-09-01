import CSwiftFlow

/// One row of a context menu.
///
/// A value, not a view: the menu is data the presenter renders, so a
/// `[MenuItem]` can be handed to `.contextMenu` and stored until a
/// secondary-click asks for it.
public struct MenuItem {

    /// A destructive item is tinted with `Color.destructive` — delete, discard,
    /// and the like.
    public enum Role: Sendable {
        case standard
        case destructive
    }

    public let title: String
    public let icon: Icon?
    public let role: Role
    public let action: () -> Void

    public init(
        _ title: String,
        icon: Icon? = nil,
        role: Role = .standard,
        action: @escaping () -> Void
    ) {
        self.title = title
        self.icon = icon
        self.role = role
        self.action = action
    }
}

/// Menus attached to nodes this frame, keyed by node id — the same
/// per-node-registry shape `GestureRegistry` uses for taps and drags.
///
/// Cleared and repopulated every build; a menu registered under an id that
/// `buildAny` later rekeys is moved with `rekey`, so an `.id()` above a
/// `.contextMenu` does not orphan it.
final class ContextMenuRegistry {
    nonisolated(unsafe) static let shared = ContextMenuRegistry()

    nonisolated(unsafe) var menus: [UInt32: [MenuItem]] = [:]

    func beginBuild() {
        menus.removeAll(keepingCapacity: true)
    }

    func menu(for id: UInt32) -> [MenuItem]? {
        menus[id]
    }

    func rekey(from old: UInt32, to new: UInt32) {
        if let menu = menus.removeValue(forKey: old) { menus[new] = menu }
    }
}

public struct ContextMenuModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }
    let content: Content
    let items: [MenuItem]
}

extension ContextMenuModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()

        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        ContextMenuRegistry.shared.menus[id] = items
        return node
    }
}

extension View {

    /// Presents `items` when this view is secondary-clicked.
    ///
    /// The registered region is this view's hit area; when several views under
    /// the pointer carry menus, the innermost wins — the framework's hit path
    /// is ordered innermost-first, and the presenter takes the first match.
    public func contextMenu(_ items: [MenuItem]) -> ContextMenuModifier<Self> {
        ContextMenuModifier(content: self, items: items)
    }
}

/// Holds the one menu currently on screen, if any.
///
/// A singleton read during the build like the navigation stores: the host
/// resolves a secondary-click to a `[MenuItem]` and hands it here, the overlay
/// woven in by `NodeBuilder.build` renders it, and an outside tap clears it.
public final class MenuPresenter {
    nonisolated(unsafe) public static let shared = MenuPresenter()

    struct Presentation {
        let items: [MenuItem]
        let anchor: Point
    }

    private(set) var presentation: Presentation?

    public var isPresenting: Bool { presentation != nil }

    private func present(_ items: [MenuItem], at anchor: Point) {
        guard !items.isEmpty else { return }
        presentation = Presentation(items: items, anchor: anchor)
        NodeRegistry.shared.needsRender = true
    }

    public func dismiss() {
        guard presentation != nil else { return }
        presentation = nil
        NodeRegistry.shared.needsRender = true
    }

    /// The host's secondary-click entry point: `path` is the hit path at the
    /// click, innermost-first. The first node carrying a menu wins; a click on
    /// bare space closes whatever was open.
    public func handleSecondaryClick(path: [UInt32], at point: Point) {
        for id in path {
            if let items = ContextMenuRegistry.shared.menu(for: id) {
                present(items, at: point)
                return
            }
        }
        dismiss()
    }
}

// MARK: - Rendering

/// Wraps the app's root so the menu floats above everything, including modals.
/// Only woven in by `NodeBuilder.build` while a menu is up; the base is pinned
/// to fill so being nested one level deeper doesn't let it collapse to its hug
/// size the way an unstretched child of a ZStack would.
struct MenuOverlayHost: View {
    let base: AnyView

    var body: some View {
        ZStack(alignment: .topLeading) {
            base
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            if let presentation = MenuPresenter.shared.presentation {
                MenuOverlay(presentation: presentation)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private enum MenuMetrics {
    static let width: Float = 240
    static let rowHeight: Float = 30
    static let verticalPadding: Float = 6
    static let screenInset: Float = 8
}

struct MenuOverlay: View {
    let presentation: MenuPresenter.Presentation

    private var origin: Point {
        let anchor = presentation.anchor

        // Keep the panel on screen: an estimated height is enough to decide
        // whether it would spill off the bottom, since rows are fixed-height.
        let estimatedHeight = Float(presentation.items.count) * MenuMetrics.rowHeight
            + MenuMetrics.verticalPadding * 2
        let maxX = max(0, Screen.width - MenuMetrics.width - MenuMetrics.screenInset)
        let maxY = max(0, Screen.height - estimatedHeight - MenuMetrics.screenInset)
        return Point(x: min(anchor.x, maxX), y: min(anchor.y, maxY))
    }

    var body: some View {
        ZStack(alignment: .topLeading) {
            // Full-screen catcher: any tap outside the panel closes the menu.
            // Transparent, but hit-testable because `onTap` gives it an id and
            // the hit test is geometric, not colour-aware.
            RoundedRectangle(cornerRadius: 0)
                .fill(.clear)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .onTap { _ in MenuPresenter.shared.dismiss() }

            panel
                .offset(x: origin.x, y: origin.y)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var panel: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(presentation.items.indices, id: \.self) { index in
                MenuRow(item: presentation.items[index])
            }
        }
        .padding(.init(top: MenuMetrics.verticalPadding, bottom: MenuMetrics.verticalPadding, leading: 0, trailing: 0))
        .frame(width: MenuMetrics.width)
        .background(
            RoundedRectangle(cornerRadius: 10)
                .fill(.overlay)
                .specular()
        )
        .cornerRadius(10)
        .shadow(radius: 20, opacity: 0.35)
    }
}

private struct MenuRow: View {
    let item: MenuItem

    private var tint: Color {
        item.role == .destructive ? .destructive : .primary
    }

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            if let icon = item.icon {
                icon.size(15).foregroundColor(tint)
            }
            Text(item.title)
                .foregroundColor(tint)
            Spacer()
        }
        .padding(.init(top: 6, bottom: 6, leading: 12, trailing: 12))
        .frame(width: MenuMetrics.width, alignment: .leading)
        .onTap { _ in
            // Close first, then act: the action may open its own UI (an inline
            // rename field, another menu) that should not be torn down by the
            // dismiss that follows it.
            MenuPresenter.shared.dismiss()
            item.action()
        }
    }
}
