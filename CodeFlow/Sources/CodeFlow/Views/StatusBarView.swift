import SwiftFlow

struct StatusBarView: View {
    let document: Document?

    var body: some View {
        let theme = Theme.current

        HStack(alignment: .center, spacing: 16) {
            HStack(alignment: .center, spacing: 6) {
                Icon.gitBranch
                    .size(12)
                    .foregroundColor(theme.secondaryText)
                Text("main")
                    .font(.system(size: 11))
                    .foregroundColor(theme.secondaryText)
            }

            Spacer()

            if let document {
                Text("Ln \(document.cursor.line + 1), Col \(document.cursor.column + 1)")
                    .font(.system(size: 11))
                    .foregroundColor(theme.secondaryText)

                Text("\(document.buffer.lineCount) lines")
                    .font(.system(size: 11))
                    .foregroundColor(theme.secondaryText)

                Text(document.language.displayName)
                    .font(.system(size: 11))
                    .foregroundColor(theme.secondaryText)
            } else {
                Text("No file open")
                    .font(.system(size: 11))
                    .foregroundColor(theme.gutterText)
            }
        }
        .padding(.horizontal, 14)
        .frame(height: Metrics.statusBarHeight, maxWidth: .infinity, alignment: .center)
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.chromeBackground)
        )
    }
}
