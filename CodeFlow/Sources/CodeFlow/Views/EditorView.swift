import SwiftFlow

struct EditorView: View {
    let document: Document

    private static let overscan = 4

    private static let firstFrameLines = 60

    var body: some View {
        let theme = Theme.current
        let lines = Highlighter.cached(for: document)
        let gutterWidth = Metrics.gutterWidth(digits: document.buffer.lineNumberDigits)
        let lineHeight = Metrics.lineHeight

        ScrollView(.vertical) { scroll in
            let visibleCount =
                scroll.viewportLength > 0
                ? Int(scroll.viewportLength / lineHeight) + Self.overscan * 2
                : Self.firstFrameLines

            let first = min(
                max(0, Int(scroll.offset / lineHeight) - Self.overscan),
                max(0, lines.count - visibleCount)
            )
            let last = min(lines.count, first + visibleCount)

            VStack(alignment: .leading, spacing: 0) {
                ForEach(lines[first..<last]) { line in
                    LineRowView(
                        line: line,
                        gutterWidth: gutterWidth,
                        isCurrent: line.id == document.cursor.line,
                        caretColumn: line.id == document.cursor.line
                            ? document.cursor.column : nil
                    )
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            .padding(
                EdgeInsets(
                    top: Float(first) * lineHeight,
                    bottom: Float(max(0, lines.count - last)) * lineHeight
                        + lineHeight * 3,
                    leading: 0,
                    trailing: 0
                )
            )

            .gesture(
                DragGesture(minimumDistance: 0).onEnded { value in

                    let travel = abs(value.translation.x) + abs(value.translation.y)
                    guard travel < 4 else { return }
                    place(caretFrom: value.startLocation, gutterWidth: gutterWidth)
                }
            )
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.editorBackground)
        )

        .textInput(
            isFocused: true,
            onInsert: { document.insert($0) },
            onKey: { key, modifiers in document.handle(key, modifiers) }
        )
    }

    private func place(caretFrom point: Point, gutterWidth: Float) {
        let textX = point.x - gutterWidth - Metrics.gutterPadding
        let line = Metrics.line(atY: point.y)
        document.place(
            at: Position(line: line, column: Metrics.column(atX: max(0, textX)))
        )
    }
}

struct EmptyEditorView: View {
    var body: some View {
        let theme = Theme.current

        VStack(spacing: 12) {
            Icon.bracketsCurly
                .size(40)
                .foregroundColor(theme.border)
            Text("No file open")
                .font(.system(size: 15, weight: .medium))
                .foregroundColor(theme.secondaryText)
            Text("Pick something from the sidebar")
                .font(.system(size: 13))
                .foregroundColor(theme.gutterText)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.editorBackground)
        )
    }
}
