import Foundation
import SwiftFlow

final class Document: Identifiable, Observable {
    let id: Int
    let name: String
    let path: String
    let language: Language

    @Observed private(set) var buffer: TextBuffer
    @Observed private(set) var cursor: Position = .start
    @Observed private(set) var isModified = false

    private var desiredColumn: Int = 0

    init(id: Int, path: String, contents: String) {
        self.id = id
        self.path = path
        self.name = (path as NSString).lastPathComponent
        self.language = Language.forFile(named: path)
        self.buffer = TextBuffer(text: contents)
    }

    func markSaved() {
        isModified = false
    }

    func insert(_ text: String) {
        cursor = buffer.insert(text, at: cursor)
        desiredColumn = cursor.column
        isModified = true
    }

    func handle(_ key: Key, _ modifiers: KeyModifiers) -> Bool {
        switch key {
        case .left:
            moveLeft(word: modifiers.contains(.option))
        case .right:
            moveRight(word: modifiers.contains(.option))
        case .up:
            moveVertically(by: -1)
        case .down:
            moveVertically(by: 1)
        case .home:
            moveToLineStart()
        case .end:
            moveToLineEnd()
        case .pageUp:
            moveVertically(by: -Self.pageLines)
        case .pageDown:
            moveVertically(by: Self.pageLines)
        case .backspace:
            cursor = buffer.deleteBackward(at: cursor)
            desiredColumn = cursor.column
            isModified = true
        case .delete:
            cursor = buffer.deleteForward(at: cursor)
            desiredColumn = cursor.column
            isModified = true
        case .enter:
            cursor = buffer.insertNewline(at: cursor)
            desiredColumn = cursor.column
            isModified = true
        case .tab:

            insert(String(repeating: " ", count: Self.indentWidth))
        case .escape, .other:
            return false
        }
        return true
    }

    static let indentWidth = 4

    static let pageLines = 20

    func place(at position: Position) {
        cursor = buffer.clamp(position)
        desiredColumn = cursor.column
    }

    private func moveLeft(word: Bool) {
        if word {
            cursor = wordBoundary(from: cursor, forward: false)
        } else if cursor.column > 0 {
            cursor.column -= 1
        } else if cursor.line > 0 {

            cursor.line -= 1
            cursor.column = buffer.lineLength(cursor.line)
        }
        desiredColumn = cursor.column
    }

    private func moveRight(word: Bool) {
        if word {
            cursor = wordBoundary(from: cursor, forward: true)
        } else if cursor.column < buffer.lineLength(cursor.line) {
            cursor.column += 1
        } else if cursor.line + 1 < buffer.lineCount {
            cursor.line += 1
            cursor.column = 0
        }
        desiredColumn = cursor.column
    }

    private func moveVertically(by delta: Int) {
        let line = min(max(0, cursor.line + delta), buffer.lineCount - 1)
        cursor = Position(
            line: line,

            column: min(desiredColumn, buffer.lineLength(line))
        )
    }

    private func moveToLineStart() {

        let line = buffer.line(cursor.line)
        let indent = line.prefix(while: { $0 == " " || $0 == "\t" }).count
        cursor.column = cursor.column == indent ? 0 : indent
        desiredColumn = cursor.column
    }

    private func moveToLineEnd() {
        cursor.column = buffer.lineLength(cursor.line)
        desiredColumn = cursor.column
    }

    private func wordBoundary(from position: Position, forward: Bool) -> Position {
        let characters = Array(buffer.line(position.line))
        var column = position.column

        if forward {
            guard column < characters.count else {
                return position.line + 1 < buffer.lineCount
                    ? Position(line: position.line + 1, column: 0) : position
            }
            while column < characters.count, !isWordCharacter(characters[column]) { column += 1 }
            while column < characters.count, isWordCharacter(characters[column]) { column += 1 }
        } else {
            guard column > 0 else {
                return position.line > 0
                    ? Position(line: position.line - 1, column: buffer.lineLength(position.line - 1))
                    : position
            }
            while column > 0, !isWordCharacter(characters[column - 1]) { column -= 1 }
            while column > 0, isWordCharacter(characters[column - 1]) { column -= 1 }
        }
        return Position(line: position.line, column: column)
    }

    private func isWordCharacter(_ c: Character) -> Bool {
        c.isLetter || c.isNumber || c == "_"
    }
}
