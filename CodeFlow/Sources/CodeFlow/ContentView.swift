import SwiftFlow

struct ContentView: View {
    var body: some View {
        let theme = Theme.current
        let workspace = Workspace.shared

        VStack(spacing: 0) {
            HStack(spacing: 0) {
                FileTreeView()

                Color.clear
                    .frame(width: 1, maxHeight: .infinity)
                    .background(
                        RoundedRectangle(cornerRadius: 0).fill(theme.border)
                    )

                VStack(spacing: 0) {
                    TabStripView()

                    if let document = workspace.activeDocument {
                        EditorView(document: document).weight(1)
                    } else {
                        EmptyEditorView().weight(1)
                    }
                }
                .expands()

                .weight(1)
            }
            .expands()
            .weight(1)

            StatusBarView(document: workspace.activeDocument)
        }
        .expands()
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.editorBackground)
        )
    }
}
