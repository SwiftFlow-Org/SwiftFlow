import SwiftFlow

struct FileTreeView: View {
    var body: some View {
        let theme = Theme.current
        let workspace = Workspace.shared

        VStack(alignment: .leading, spacing: 0) {
            Text("EXPLORER")
                .font(.system(size: 11, weight: .semibold))
                .foregroundColor(theme.secondaryText)
                .padding(.horizontal, 14)
                .padding(.vertical, 12)

            ScrollView(.vertical) {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(workspace.visibleEntries) { entry in
                        FileRowView(entry: entry)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            Spacer()
        }
        .frame(width: Metrics.sidebarWidth, maxHeight: .infinity, alignment: .topLeading)
        .background(
            RoundedRectangle(cornerRadius: 0).fill(theme.sidebarBackground)
        )
    }
}

struct FileRowView: View {
    let entry: FileEntry

    var body: some View {
        let theme = Theme.current
        let workspace = Workspace.shared
        let isActive = workspace.activeDocument?.path == entry.path

        Button(action: { workspace.open(entry) }) {
            HStack(alignment: .center, spacing: 6) {

                if entry.isDirectory {
                    (workspace.isCollapsed(entry) ? Icon.caretRight : Icon.caretDown)
                        .size(10)
                        .foregroundColor(theme.secondaryText)
                        .frame(width: 12, alignment: .center)
                } else {

                    Color.clear.frame(width: 12, height: 1)
                }

                icon(for: entry)
                    .size(14)
                    .foregroundColor(entry.isDirectory ? theme.accent : theme.secondaryText)

                Text(entry.name)
                    .font(.system(size: 13, weight: isActive ? .medium : .regular))
                    .foregroundColor(isActive ? theme.plain : theme.secondaryText)
                    .lineLimit(1)

                Spacer()
            }
            .padding(.leading, 10 + Float(entry.depth) * 14)
            .padding(.trailing, 10)
            .frame(height: Metrics.rowHeight, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 5)
                    .fill(isActive ? theme.currentLine : .clear)
            )
        }
        .buttonStyle(PlainButtonStyle())
    }

    private func icon(for entry: FileEntry) -> Icon {
        guard !entry.isDirectory else {
            return Workspace.shared.isCollapsed(entry) ? .folder : .folderOpen
        }
        switch Language.forFile(named: entry.name) {
        case .swift, .rust: return .fileCode
        case .toml: return .gearSix
        case .markdown: return .textAa
        case .plain: return .file
        }
    }
}
