import SwiftFlow

struct LineRowView: View {
    let line: HighlightedLine
    let gutterWidth: Float
    let isCurrent: Bool

    let caretColumn: Int?

    var body: some View {
        let theme = Theme.current

        HStack(alignment: .center, spacing: 0) {
            Text("\(line.id + 1)")
                .font(.system(size: Metrics.codeFontSize, design: .monospaced))
                .foregroundColor(isCurrent ? theme.gutterActiveText : theme.gutterText)
                .frame(width: gutterWidth, alignment: .trailing)

            ZStack(alignment: .topLeading) {

                HStack(alignment: .center, spacing: 0) {
                    ForEach(line.tokens) { token in
                        Text(token.text)
                            .font(.system(size: Metrics.codeFontSize, design: .monospaced))
                            .foregroundColor(theme.color(for: token.kind))
                    }
                }
                .frame(height: Metrics.lineHeight, alignment: .leading)

                if let caretColumn {

                    RoundedRectangle(cornerRadius: 1)
                        .fill(theme.accent)
                        .frame(width: 2, height: Metrics.codeFontSize * 1.25)
                        .offset(x: Metrics.x(ofColumn: caretColumn), y: 3)
                }
            }
            .padding(.leading, Metrics.gutterPadding)

            Spacer()
        }
        .frame(height: Metrics.lineHeight, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 0)
                .fill(isCurrent ? theme.currentLine : .clear)
        )
    }
}
