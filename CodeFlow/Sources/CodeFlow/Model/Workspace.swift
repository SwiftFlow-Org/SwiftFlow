import Foundation
import SwiftFlow

struct FileEntry: Identifiable {
    let id: Int
    let name: String
    let path: String
    let isDirectory: Bool

    let depth: Int
}

final class Workspace: Observable {
    nonisolated(unsafe) static let shared = Workspace()

    @Observed private(set) var entries: [FileEntry] = []
    @Observed private(set) var documents: [Document] = []
    @Observed private(set) var activeDocumentID: Int?

    @Observed private var collapsed: Set<String> = []

    private var nextDocumentID = 0

    private init() {
        loadSampleProject()
    }

    var activeDocument: Document? {
        guard let id = activeDocumentID else { return nil }
        return documents.first { $0.id == id }
    }

    var visibleEntries: [FileEntry] {
        var result: [FileEntry] = []
        var hiddenPrefix: String?
        for entry in entries {
            if let prefix = hiddenPrefix {
                if entry.path.hasPrefix(prefix) { continue }
                hiddenPrefix = nil
            }
            result.append(entry)
            if entry.isDirectory && collapsed.contains(entry.path) {

                hiddenPrefix = entry.path + "/"
            }
        }
        return result
    }

    func isCollapsed(_ entry: FileEntry) -> Bool {
        collapsed.contains(entry.path)
    }

    func toggle(_ entry: FileEntry) {
        guard entry.isDirectory else { return }
        if collapsed.contains(entry.path) {
            collapsed.remove(entry.path)
        } else {
            collapsed.insert(entry.path)
        }
    }

    func open(_ entry: FileEntry) {
        guard !entry.isDirectory else {
            toggle(entry)
            return
        }
        if let existing = documents.first(where: { $0.path == entry.path }) {
            activeDocumentID = existing.id
            return
        }
        let document = Document(
            id: nextDocumentID,
            path: entry.path,
            contents: SampleProject.contents(of: entry.path)
        )
        nextDocumentID += 1
        documents.append(document)
        activeDocumentID = document.id
    }

    func focus(_ document: Document) {
        activeDocumentID = document.id
    }

    func close(_ document: Document) {
        documents.removeAll { $0.id == document.id }
        guard activeDocumentID == document.id else { return }

        activeDocumentID = documents.last?.id
    }

    private func loadSampleProject() {
        entries = SampleProject.entries
        if let first = entries.first(where: { !$0.isDirectory }) {
            open(first)
        }
    }

    func open(directoryAt root: String, fileManager: FileManager = .default) {
        var loaded: [FileEntry] = []
        var id = 0

        func walk(_ directory: String, depth: Int) {
            let children = (try? fileManager.contentsOfDirectory(atPath: directory)) ?? []

            let sorted = children.sorted { lhs, rhs in
                let lhsDir = isDirectory("\(directory)/\(lhs)", fileManager)
                let rhsDir = isDirectory("\(directory)/\(rhs)", fileManager)
                if lhsDir != rhsDir { return lhsDir }
                return lhs.localizedCaseInsensitiveCompare(rhs) == .orderedAscending
            }
            for child in sorted where !child.hasPrefix(".") {
                let path = "\(directory)/\(child)"
                let directoryFlag = isDirectory(path, fileManager)
                loaded.append(
                    FileEntry(
                        id: id, name: child, path: path,
                        isDirectory: directoryFlag, depth: depth
                    )
                )
                id += 1
                if directoryFlag { walk(path, depth: depth + 1) }
            }
        }

        walk(root, depth: 0)
        entries = loaded
        documents.removeAll()
        activeDocumentID = nil
    }

    private func isDirectory(_ path: String, _ fileManager: FileManager) -> Bool {
        var flag: ObjCBool = false
        let exists = fileManager.fileExists(atPath: path, isDirectory: &flag)
        return exists && flag.boolValue
    }
}
