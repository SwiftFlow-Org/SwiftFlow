import CSwiftFlow
import Foundation

public final class ImageRegistry {
    nonisolated(unsafe) public static let shared = ImageRegistry()

    public struct Entry {
        public let id: UInt32
        public let pixelWidth: Float
        public let pixelHeight: Float

        public let assetScale: Float

        public var width: Float { pixelWidth / assetScale }
        public var height: Float { pixelHeight / assetScale }
    }

    private var nextID: UInt32 = 1
    private var entries: [String: Entry] = [:]

    private var failures: Set<String> = []

    public func asset(_ name: String) -> Entry? {
        if let existing = entries[name] { return existing }
        if failures.contains(name) { return nil }

        guard let asset = AssetCatalog.load(name) else {

            Log.print("[SwiftFlow] Image(\"\(name)\") not found. Searched:")
            for path in AssetCatalog.searchedPaths(for: name) {
                Log.print("[SwiftFlow]   \(path)")
            }
            failures.insert(name)
            return nil
        }
        guard let entry = register(asset.data, assetScale: asset.scale) else {
            Log.print(
                "[SwiftFlow] Image(\"\(name)\") found but not decodable — PNG and JPEG only"
            )
            failures.insert(name)
            return nil
        }
        entries[name] = entry
        return entry
    }

    public func register(_ data: Data, assetScale: Float) -> Entry? {
        let id = nextID
        let size = data.withUnsafeBytes { buffer -> SFImageSize in
            guard let base = buffer.bindMemory(to: UInt8.self).baseAddress else {
                return SFImageSize(width: 0, height: 0)
            }
            return sf_register_image(id, base, buffer.count)
        }
        guard size.width > 0, size.height > 0 else { return nil }

        nextID += 1
        return Entry(
            id: id,
            pixelWidth: size.width,
            pixelHeight: size.height,
            assetScale: assetScale
        )
    }

    /// Reserves an image id for a live stream (see `updateRaw`).
    public func reserveStreamID() -> UInt32 {
        let id = nextID
        nextID += 1
        return id
    }

    /// Uploads raw BGRA pixels under `id`, replacing any prior texture, and
    /// returns a fresh `Entry` for the current dimensions. For live sources
    /// (the preview pane) where decoding a PNG per frame would be far too slow.
    /// Must run on the render thread — it touches the GPU backend.
    public func updateRaw(
        id: UInt32,
        width: Int,
        height: Int,
        assetScale: Float,
        bgra: UnsafeRawPointer,
        byteCount: Int
    ) -> Entry {
        sf_register_image_raw(
            id,
            bgra.assumingMemoryBound(to: UInt8.self),
            byteCount,
            UInt32(width),
            UInt32(height)
        )
        return Entry(
            id: id,
            pixelWidth: Float(width),
            pixelHeight: Float(height),
            assetScale: assetScale
        )
    }

    public func release(_ entry: Entry) {
        sf_unregister_image(entry.id)
        entries = entries.filter { $0.value.id != entry.id }
    }
}
