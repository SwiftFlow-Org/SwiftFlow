import Foundation

struct TextBuffer {
    private(set) var lines: [String]

    private(set) var revision: Int = 0

    init(lines: [String]) {

        self.lines = lines.isEmpty ? [""] : lines
    }

    init(text: String) {

        self.init(
            lines: text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        )
    }

    var lineCount: Int { lines.count }
    var text: String { lines.joined(separator: "\n") }

    func line(_ index: Int) -> String {
        guard lines.indices.contains(index) else { return "" }
        return lines[index]
    }

    func lineLength(_ index: Int) -> Int {
        line(index).count
    }

    var lineNumberDigits: Int {
        String(lineCount).count
    }

    func clamp(_ position: Position) -> Position {
        let line = min(max(0, position.line), lines.count - 1)
        let column = min(max(0, position.column), lines[line].count)
        return Position(line: line, column: column)
    }

    @discardableResult
    mutating func insert(_ text: String, at position: Position) -> Position {
        guard !text.isEmpty else { return position }
        let at = clamp(position)
        let line = lines[at.line]
        let split = line.index(line.startIndex, offsetBy: at.column)
        let head = String(line[line.startIndex..<split])
        let tail = String(line[split...])

        let inserted = text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        revision += 1

        if inserted.count == 1 {
            lines[at.line] = head + inserted[0] + tail
            return Position(line: at.line, column: at.column + inserted[0].count)
        }

        var replacement = inserted
        replacement[0] = head + replacement[0]
        let lastIndex = replacement.count - 1
        let caretColumn = replacement[lastIndex].count
        replacement[lastIndex] += tail
        lines.replaceSubrange(at.line...at.line, with: replacement)
        return Position(line: at.line + lastIndex, column: caretColumn)
    }

    @discardableResult
    mutating func deleteBackward(at position: Position) -> Position {
        let at = clamp(position)
        if at.column > 0 {
            var line = lines[at.line]
            let target = line.index(line.startIndex, offsetBy: at.column - 1)
            line.remove(at: target)
            lines[at.line] = line
            revision += 1
            return Position(line: at.line, column: at.column - 1)
        }
        guard at.line > 0 else { return at }
        let previousLength = lines[at.line - 1].count
        lines[at.line - 1] += lines[at.line]
        lines.remove(at: at.line)
        revision += 1
        return Position(line: at.line - 1, column: previousLength)
    }

    @discardableResult
    mutating func deleteForward(at position: Position) -> Position {
        let at = clamp(position)
        var line = lines[at.line]
        if at.column < line.count {
            let target = line.index(line.startIndex, offsetBy: at.column)
            line.remove(at: target)
            lines[at.line] = line
            revision += 1
            return at
        }
        guard at.line + 1 < lines.count else { return at }
        lines[at.line] += lines[at.line + 1]
        lines.remove(at: at.line + 1)
        revision += 1
        return at
    }

    @discardableResult
    mutating func insertNewline(at position: Position, autoIndent: Bool = true) -> Position {
        let at = clamp(position)
        let line = lines[at.line]
        let indent = autoIndent ? String(line.prefix(while: { $0 == " " || $0 == "\t" })) : ""

        let carried = String(indent.prefix(at.column))
        return insert("\n" + carried, at: at)
    }
}

struct Position: Equatable {
    var line: Int
    var column: Int

    static let start = Position(line: 0, column: 0)
}
