import Foundation

enum Highlighter {
    static func highlight(_ buffer: TextBuffer, language: Language) -> [HighlightedLine] {
        var state = LexerState.normal
        var result: [HighlightedLine] = []
        result.reserveCapacity(buffer.lineCount)
        for index in 0..<buffer.lineCount {
            let (tokens, next) = Lexer.tokenize(
                line: buffer.line(index), language: language, state: state
            )
            result.append(HighlightedLine(id: index, tokens: tokens))
            state = next
        }
        return result
    }

    private struct Key: Equatable {
        let documentID: Int
        let lineCount: Int

        let firstLine: String
    }

    nonisolated(unsafe) private static var cacheKey: Key?
    nonisolated(unsafe) private static var cacheValue: [HighlightedLine] = []

    static func cached(for document: Document) -> [HighlightedLine] {
        let key = Key(
            documentID: document.id,
            lineCount: document.buffer.lineCount,
            firstLine: document.buffer.line(0)
        )
        if cacheKey == key { return cacheValue }
        cacheValue = highlight(document.buffer, language: document.language)
        cacheKey = key
        return cacheValue
    }
}
