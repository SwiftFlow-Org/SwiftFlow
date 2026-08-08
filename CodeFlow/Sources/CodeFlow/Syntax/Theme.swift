import SwiftFlow

struct Theme {

    let editorBackground: Color
    let sidebarBackground: Color
    let chromeBackground: Color
    let border: Color
    let gutterText: Color
    let gutterActiveText: Color
    let currentLine: Color
    let secondaryText: Color
    let accent: Color

    let plain: Color
    let keyword: Color
    let type: Color
    let string: Color
    let number: Color
    let comment: Color
    let punctuation: Color
    let attribute: Color

    func color(for kind: TokenKind) -> Color {
        switch kind {
        case .plain: return plain
        case .keyword: return keyword
        case .type: return type
        case .string: return string
        case .number: return number
        case .comment: return comment
        case .punctuation: return punctuation
        case .attribute: return attribute
        }
    }

    static let dusk = Theme(
        editorBackground: Color(hex: 0x16140F),
        sidebarBackground: Color(hex: 0x121009),
        chromeBackground: Color(hex: 0x1C1913),
        border: Color(hex: 0x2A251C),
        gutterText: Color(hex: 0x4A4438),
        gutterActiveText: Color(hex: 0x9A9184),
        currentLine: Color(hex: 0x211D16),
        secondaryText: Color(hex: 0x6E675C),
        accent: Color(hex: 0xC15B3A),

        plain: Color(hex: 0xE8E1D6),
        keyword: Color(hex: 0xC15B3A),
        type: Color(hex: 0xE0A45C),
        string: Color(hex: 0x9BB06B),
        number: Color(hex: 0xD08F6E),
        comment: Color(hex: 0x5D5648),
        punctuation: Color(hex: 0x9A9184),
        attribute: Color(hex: 0xB08CC4)
    )

    nonisolated(unsafe) static var current: Theme = .dusk {
        didSet { ObservationRegistrar.invalidate() }
    }
}
