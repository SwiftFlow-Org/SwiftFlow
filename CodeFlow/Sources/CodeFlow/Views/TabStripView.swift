import SwiftFlow

struct TabStripView: View {
    var body: some View {
        let theme = Theme.current
        let workspace = Workspace.shared

        HStack(alignment: .center, spacing: 0) {
            ForEach(workspace.documents) { document in
                DocumentTabView(document: document)
            }
            Spacer()
        }
        .frame(height: Metrics.tabHeight, maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.chromeBackground)
        )
    }
}

struct DocumentTabView: View {
    let document: Document

    var body: some View {
        let theme = Theme.current
        let workspace = Workspace.shared
        let isActive = workspace.activeDocumentID == document.id

        HStack(alignment: .center, spacing: 8) {
            Button(action: { workspace.focus(document) }) {
                HStack(alignment: .center, spacing: 6) {
                    Text(document.name)
                        .font(.system(size: 13, weight: isActive ? .medium : .regular))
                        .foregroundColor(isActive ? theme.plain : theme.secondaryText)
                        .lineLimit(1)

                    if document.isModified {
                        Circle()
                            .fill(theme.accent)
                            .frame(width: 6, height: 6)
                    }
                }
            }
            .buttonStyle(PlainButtonStyle())

            Button(action: { workspace.close(document) }) {
                Icon.x
                    .size(10)
                    .foregroundColor(theme.gutterText)
            }
            .buttonStyle(PlainButtonStyle())
        }
        .padding(.horizontal, 14)
        .frame(height: Metrics.tabHeight, alignment: .center)
        .background(
            RoundedRectangle(cornerRadius: 0)
                .fill(isActive ? theme.editorBackground : .clear)
        )
    }
}
