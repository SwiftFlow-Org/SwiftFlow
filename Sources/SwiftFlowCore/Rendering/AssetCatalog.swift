import Foundation

public enum AssetCatalog {

    public struct Asset {
        public let data: Data
        public let scale: Float
    }

    nonisolated(unsafe) public static var searchRoots: [URL] = defaultSearchRoots()

    nonisolated(unsafe) public static var subdirectories: [String] = ["Assets", ""]

    nonisolated(unsafe) public static var fileExtensions: [String] = ["png", "jpg", "jpeg"]

    public static func load(_ name: String) -> Asset? {
        for candidate in candidates(for: name) {
            if let data = try? Data(contentsOf: candidate.url), !data.isEmpty {
                return Asset(data: data, scale: candidate.scale)
            }
        }
        return nil
    }

    public static func searchedPaths(for name: String) -> [String] {

        candidates(for: name).map { $0.url.path }
    }

    private static func candidates(for name: String) -> [(url: URL, scale: Float)] {
        let (base, explicitExtension) = splitExtension(name)
        let extensions = explicitExtension.map { [$0] } ?? fileExtensions

        var out: [(url: URL, scale: Float)] = []
        for candidate in variants() {
            for root in searchRoots {
                for subdirectory in subdirectories {
                    let directory = subdirectory.isEmpty
                        ? root
                        : root.appendingPathComponent(subdirectory)
                    for ext in extensions {
                        out.append((
                            directory.appendingPathComponent(
                                "\(base)\(candidate.suffix).\(ext)"
                            ),
                            candidate.scale
                        ))
                    }
                }
            }
        }
        return out
    }

    private static func variants() -> [(suffix: String, scale: Float)] {
        var out: [(suffix: String, scale: Float)] = []
        var scale = max(1, Int(DeviceScale.current.rounded()))
        while scale > 1 {
            out.append(("@\(scale)x", Float(scale)))
            scale -= 1
        }
        out.append(("@1x", 1))
        out.append(("", 1))
        return out
    }

    private static func splitExtension(_ name: String) -> (base: String, ext: String?) {
        guard let dot = name.lastIndex(of: "."), dot != name.startIndex else {
            return (name, nil)
        }
        let ext = String(name[name.index(after: dot)...]).lowercased()
        guard fileExtensions.contains(ext) else { return (name, nil) }
        return (String(name[..<dot]), ext)
    }

    private static func defaultSearchRoots() -> [URL] {

        var roots: [URL] = []
        for url in [Bundle.main.resourceURL, Bundle.main.bundleURL].compactMap({ $0 }) {
            if !roots.contains(where: { $0.path == url.path }) {
                roots.append(url)
            }
        }

        let topLevel = roots
        for root in topLevel {
            let nested = (try? FileManager.default.contentsOfDirectory(
                at: root,
                includingPropertiesForKeys: nil
            )) ?? []
            roots.append(contentsOf: nested.filter { $0.pathExtension == "bundle" })
        }
        return roots
    }
}
