import SwiftFlow

enum Metrics {

    static let codeFontSize: Float = 13

    static let lineHeight: Float = 20

    static let sidebarWidth: Float = 220
    static let rowHeight: Float = 26
    static let tabHeight: Float = 36
    static let statusBarHeight: Float = 24

    static let gutterPadding: Float = 12

    static let characterWidth: Float = codeFontSize * 0.6

    static func gutterWidth(digits: Int) -> Float {
        Float(max(2, digits)) * characterWidth + gutterPadding * 2
    }

    static func x(ofColumn column: Int) -> Float {
        Float(column) * characterWidth
    }

    static func column(atX x: Float) -> Int {
        max(0, Int((x / characterWidth).rounded()))
    }

    static func line(atY y: Float) -> Int {
        max(0, Int(y / lineHeight))
    }
}
