import Foundation

enum TokenKind {
    case plain
    case keyword
    case type
    case string
    case number
    case comment
    case punctuation
    case attribute
}

struct Token: Identifiable {
    let id: Int
    let text: String
    let kind: TokenKind
}

struct HighlightedLine: Identifiable {

    let id: Int
    let tokens: [Token]
}
