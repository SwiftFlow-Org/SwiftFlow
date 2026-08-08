import Foundation

enum LexerState: Equatable {
    case normal
    case blockComment
}

enum Lexer {

    static func tokenize(
        line: String,
        language: Language,
        state: LexerState = .normal
    ) -> (tokens: [Token], next: LexerState) {
        guard language != .markdown, language != .plain else {
            return ([Token(id: 0, text: line, kind: .plain)], .normal)
        }

        let characters = Array(line)
        var tokens: [Token] = []
        var pending = ""
        var index = 0
        var state = state

        func flushPending() {
            guard !pending.isEmpty else { return }
            tokens.append(Token(id: tokens.count, text: pending, kind: .plain))
            pending = ""
        }

        func emit(_ text: String, _ kind: TokenKind) {
            flushPending()
            tokens.append(Token(id: tokens.count, text: text, kind: kind))
        }

        func matches(_ prefix: String, at position: Int) -> Bool {
            let prefixCharacters = Array(prefix)
            guard position + prefixCharacters.count <= characters.count else { return false }
            for offset in prefixCharacters.indices
            where characters[position + offset] != prefixCharacters[offset] {
                return false
            }
            return true
        }

        while index < characters.count {

            if state == .blockComment {
                var comment = ""
                while index < characters.count {
                    if matches("*/", at: index) {
                        comment += "*/"
                        index += 2
                        state = .normal
                        break
                    }
                    comment.append(characters[index])
                    index += 1
                }
                emit(comment, .comment)
                continue
            }

            let character = characters[index]

            if let prefix = language.lineCommentPrefix, matches(prefix, at: index) {
                emit(String(characters[index...]), .comment)
                index = characters.count
                continue
            }
            if language.usesBlockComments, matches("/*", at: index) {

                var comment = "/*"
                index += 2
                state = .blockComment
                while index < characters.count {
                    if matches("*/", at: index) {
                        comment += "*/"
                        index += 2
                        state = .normal
                        break
                    }
                    comment.append(characters[index])
                    index += 1
                }
                emit(comment, .comment)
                continue
            }

            if character == "\"" {
                var literal = "\""
                index += 1
                while index < characters.count {
                    let current = characters[index]
                    literal.append(current)
                    index += 1

                    if current == "\\", index < characters.count {
                        literal.append(characters[index])
                        index += 1
                        continue
                    }
                    if current == "\"" { break }
                }
                emit(literal, .string)
                continue
            }

            if character.isNumber {
                var literal = ""
                while index < characters.count,
                    characters[index].isHexDigit || characters[index] == "."
                        || characters[index] == "_" || characters[index] == "x"
                        || characters[index] == "b" || characters[index] == "o"
                {
                    literal.append(characters[index])
                    index += 1
                }
                emit(literal, .number)
                continue
            }

            if character == "@" || character == "#" {
                var word = String(character)
                index += 1
                while index < characters.count, isIdentifier(characters[index]) {
                    word.append(characters[index])
                    index += 1
                }
                emit(word, .attribute)
                continue
            }
            if isIdentifierStart(character) {
                var word = ""
                while index < characters.count, isIdentifier(characters[index]) {
                    word.append(characters[index])
                    index += 1
                }
                if language.keywords.contains(word) {
                    emit(word, .keyword)
                } else if word.first?.isUppercase == true {
                    emit(word, .type)
                } else {
                    pending += word
                }
                continue
            }

            if punctuation.contains(character) {
                emit(String(character), .punctuation)
                index += 1
                continue
            }

            pending.append(character)
            index += 1
        }

        flushPending()

        if tokens.isEmpty { tokens = [Token(id: 0, text: "", kind: .plain)] }
        return (tokens, state)
    }

    private static let punctuation: Set<Character> = [
        "{", "}", "(", ")", "[", "]", ".", ",", ":", ";",
        "=", "+", "-", "*", "/", "<", ">", "?", "!", "&", "|", "%", "^", "~",
    ]

    private static func isIdentifierStart(_ character: Character) -> Bool {
        character.isLetter || character == "_"
    }

    private static func isIdentifier(_ character: Character) -> Bool {
        character.isLetter || character.isNumber || character == "_"
    }
}

extension Language {

    var usesBlockComments: Bool {
        switch self {
        case .swift, .rust: return true
        case .toml, .markdown, .plain: return false
        }
    }
}
