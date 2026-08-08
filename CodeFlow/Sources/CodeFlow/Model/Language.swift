import Foundation

enum Language: String {
    case swift
    case rust
    case toml
    case markdown
    case plain

    static func forFile(named name: String) -> Language {
        switch (name as NSString).pathExtension.lowercased() {
        case "swift": return .swift
        case "rs": return .rust
        case "toml": return .toml
        case "md", "markdown": return .markdown
        default: return .plain
        }
    }

    var displayName: String {
        switch self {
        case .swift: return "Swift"
        case .rust: return "Rust"
        case .toml: return "TOML"
        case .markdown: return "Markdown"
        case .plain: return "Plain Text"
        }
    }

    var keywords: Set<String> {
        switch self {
        case .swift:
            return [
                "associatedtype", "async", "await", "break", "case", "catch", "class",
                "continue", "default", "defer", "deinit", "do", "else", "enum",
                "extension", "fallthrough", "false", "fileprivate", "for", "func",
                "guard", "if", "import", "in", "init", "inout", "internal", "is",
                "let", "mutating", "nil", "nonisolated", "open", "operator",
                "private", "protocol", "public", "repeat", "return", "self", "Self",
                "some", "static", "struct", "subscript", "super", "switch",
                "throw", "throws", "true", "try", "typealias", "var", "where",
                "while",
            ]
        case .rust:
            return [
                "as", "async", "await", "break", "const", "continue", "crate", "dyn",
                "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
                "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
                "self", "Self", "static", "struct", "super", "trait", "true", "type",
                "unsafe", "use", "where", "while",
            ]
        case .toml:
            return ["true", "false"]
        case .markdown, .plain:
            return []
        }
    }

    var lineCommentPrefix: String? {
        switch self {
        case .swift, .rust: return "//"
        case .toml: return "#"
        case .markdown, .plain: return nil
        }
    }
}
