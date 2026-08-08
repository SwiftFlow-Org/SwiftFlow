import CSwiftFlow

/// Builds one view per element, keyed by identity.
///
/// The only place an insertion or removal can be told apart from an
/// ordinary rebuild, so it is what makes `.transition(_:)` animate.
public struct ForEach<Data: RandomAccessCollection, ID: Hashable, Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let data: Data
    let idKeyPath: KeyPath<Data.Element, ID>

    let content: (Data.Element) -> Content

    let forEachID: UInt32

    public init(
        _ data: Data,
        id: KeyPath<Data.Element, ID>,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder content: @escaping (Data.Element) -> Content
    ) {
        self.data = data
        self.idKeyPath = id
        self.content = content
        self.forEachID = fnv1a("\(fileID):\(line):\(column)")
    }

    public func toSFNode() -> SFNode {
        var kids = buildNodes()
        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_VERTICAL
        node.sizing = SF_SIZING_HUG
        node.alignment = SF_ALIGNMENT_CENTER
        node.verticalAlignment = SF_ALIGNMENT_CENTER
        guard !kids.isEmpty else { return node }
        let count = kids.count
        FrameArena.shared.storeNodes(&kids) { ptr in
            node.children = ptr
            node.childrenLen = count
        }
        return node
    }
}

extension ForEach where Data.Element: Identifiable, ID == Data.Element.ID {

    public init(
        _ data: Data,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder content: @escaping (Data.Element) -> Content
    ) {
        self.init(data, id: \.id, fileID: fileID, line: line, column: column, content: content)
    }
}

extension ForEach: MultiNodeView {
    func buildNodes() -> [SFNode] {
        var keys: [UInt32] = []
        var nodes: [SFNode] = []
        for element in data {
            let key = elementKey(for: element)
            keys.append(key)

            nodes.append(
                NodeBuilder.buildAny(
                    IdentifiedContent(content: content(element), identity: key)
                )
            )
        }

        let registry = TransitionRegistry.shared
        registry.reconcile(owner: forEachID, keys: keys)

        let departing = registry.departingNodes(owner: forEachID)
        guard !departing.isEmpty else { return nodes }

        for ghost in departing {
            nodes.insert(ghost.node, at: min(ghost.index, nodes.count))
        }
        return nodes
    }

    private func elementKey(for element: Data.Element) -> UInt32 {

        fnv1a("\(forEachID)#\(element[keyPath: idKeyPath].hashValue)")
    }
}

protocol ExplicitlyIdentifiedView {
    var explicitIdentity: UInt32 { get }
}

public struct IdentifiedContent<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let identity: UInt32

    init(content: Content, identity: UInt32) {
        self.content = content
        self.identity = identity
    }

    public func toSFNode() -> SFNode {
        content.toSFNode()
    }
}

extension IdentifiedContent: ExplicitlyIdentifiedView {
    var explicitIdentity: UInt32 { identity }
}

extension View {

    /// Binds this view's identity to the given value.
    public func id<ID: Hashable>(_ id: ID) -> IdentifiedContent<Self> {
        IdentifiedContent(content: self, identity: fnv1a("id#\(id.hashValue)"))
    }
}
