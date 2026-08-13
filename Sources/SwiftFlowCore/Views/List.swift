import CSwiftFlow

/// How a list draws its separators.
public struct ListStyle: Sendable {

    let insetsSeparator: Bool

    public static let plain = ListStyle(insetsSeparator: false)

    public static let inset = ListStyle(insetsSeparator: true)
}

struct ListMetrics {

    var insets = EdgeInsets(top: 11, bottom: 11, leading: 16, trailing: 16)
    var style: ListStyle = .inset
    var showsSeparators = true
    var separatorInset: Float { style.insetsSeparator ? insets.leading : 0 }
}

extension ListMetrics {
    /// What a row is assumed to be before it has been measured: one line of
    /// text plus this list's own chrome.
    var estimatedRowHeight: Float {
        insets.top + insets.bottom + (showsSeparators ? 1 : 0) + 22
    }
}

struct ListRow<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let metrics: ListMetrics

    let identity: UInt32

    func toSFNode() -> SFNode {

        if identity != 0 {
            NodeFrames.shared.register(identity)
        }

        let row = VStack(alignment: .leading, spacing: 0) {
            content
                .padding(metrics.insets)

            if metrics.showsSeparators {
                Divider()
                    .padding(
                        EdgeInsets(
                            top: 0, bottom: 0,
                            leading: metrics.separatorInset, trailing: 0
                        )
                    )

                    .frame(height: 1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)

        var node = row.toSFNode()
        if identity != 0 {
            node.node_id = identity
        }
        return node
    }
}

public struct ListRows<Data: RandomAccessCollection, ID: Hashable, Row: View>: View {
    let data: Data
    let idKeyPath: KeyPath<Data.Element, ID>
    let row: (Data.Element) -> Row
    let metrics: ListMetrics
    let geometry: ScrollGeometry

    let fileID: String
    let line: Int
    let column: Int
    let listID: UInt32

    private func identity(of element: Data.Element) -> UInt32 {
        ListRowHeights.id(list: listID, element: element[keyPath: idKeyPath].hashValue)
    }

    private func rowIdentity(at index: Int) -> UInt32 {
        identity(of: data[data.index(data.startIndex, offsetBy: index)])
    }

    public var body: some View {
        let heights = ListRowHeights.shared
        let window = ListWindow(
            rowCount: data.count,
            offset: geometry.offset,
            viewportLength: geometry.viewportLength
        ) { index in
            heights.height(for: rowIdentity(at: index), fallback: metrics.estimatedRowHeight)
        }
        let start = data.index(data.startIndex, offsetBy: window.first)
        let end = data.index(start, offsetBy: window.count)

        VStack(alignment: .leading, spacing: 0) {
            ForEach(
                data[start..<end],
                id: idKeyPath,
                fileID: fileID, line: line, column: column
            ) { element in
                ListRow(
                    content: row(element),
                    metrics: metrics,
                    identity: identity(of: element)
                )
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(
            EdgeInsets(
                top: window.leadingPad, bottom: window.trailingPad,
                leading: 0, trailing: 0
            )
        )
    }
}

public struct StaticListRows<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: () -> Content
    let metrics: ListMetrics

    public func toSFNode() -> SFNode {
        var rows = buildChildren(content()).map { child in
            ListRow(
                content: NodeListView(nodes: [child]),
                metrics: metrics,

                identity: 0
            )
            .toSFNode()
        }

        var node = SFNode.makeDefault()
        node.kind = SF_NODE_STACK
        node.axis = SF_AXIS_VERTICAL
        node.spacing = 0
        node.sizing = SF_SIZING_HUG
        node.alignment = SF_ALIGNMENT_LEADING
        node.verticalAlignment = SF_ALIGNMENT_CENTER

        let count = rows.count
        FrameArena.shared.storeNodes(&rows) { pointer in
            node.children = pointer
            node.childrenLen = count
        }
        return node
    }
}

/// A scrolling column of rows that only builds the ones you can see.
///
/// Rows measure themselves — nothing has to be told a row height.
public struct List<Rows: View>: View {
    let rows: (ScrollGeometry, ListMetrics) -> Rows
    var metrics: ListMetrics
    let fileID: String
    let line: Int
    let column: Int

    init(
        metrics: ListMetrics,
        fileID: String,
        line: Int,
        column: Int,
        rows: @escaping (ScrollGeometry, ListMetrics) -> Rows
    ) {
        self.rows = rows
        self.metrics = metrics
        self.fileID = fileID
        self.line = line
        self.column = column
    }

    public var body: some View {
        ScrollView(.vertical, fileID: fileID, line: line, column: column) { geometry in
            rows(geometry, metrics)
        }
    }
}

extension List {

    public init<Content: View>(
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder content: @escaping () -> Content
    ) where Rows == StaticListRows<Content> {
        self.init(
            metrics: ListMetrics(), fileID: fileID, line: line, column: column
        ) { _, metrics in
            StaticListRows(content: content, metrics: metrics)
        }
    }

    public init<Data: RandomAccessCollection, ID: Hashable, Row: View>(
        _ data: Data,
        id: KeyPath<Data.Element, ID>,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder row: @escaping (Data.Element) -> Row
    ) where Rows == ListRows<Data, ID, Row> {

        let listID = fnv1a("\(fileID):\(line):\(column)")
        self.init(
            metrics: ListMetrics(), fileID: fileID, line: line, column: column
        ) { geometry, metrics in
            ListRows(
                data: data,
                idKeyPath: id,
                row: row,
                metrics: metrics,
                geometry: geometry,
                fileID: fileID, line: line, column: column,
                listID: listID
            )
        }
    }

    public init<Data: RandomAccessCollection, Row: View>(
        _ data: Data,
        fileID: String = #fileID, line: Int = #line, column: Int = #column,
        @ViewBuilder row: @escaping (Data.Element) -> Row
    ) where Data.Element: Identifiable, Rows == ListRows<Data, Data.Element.ID, Row> {
        self.init(data, id: \.id, fileID: fileID, line: line, column: column, row: row)
    }
}

extension List {
    /// Sets the style of this list.
    public func listStyle(_ style: ListStyle) -> List {
        var copy = self
        copy.metrics.style = style
        return copy
    }

    /// Whether rows are separated by a hairline.
    public func listRowSeparator(_ visible: Bool) -> List {
        var copy = self
        copy.metrics.showsSeparators = visible
        return copy
    }

    /// The space around each row's content.
    public func listRowInsets(_ insets: EdgeInsets) -> List {
        var copy = self
        copy.metrics.insets = insets
        return copy
    }
}
