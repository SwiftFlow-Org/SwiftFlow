import CSwiftFlow
import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public enum AsyncImagePhase {
    case empty
    case success(Image)
    case failure(Error)

    public var image: Image? {
        if case .success(let image) = self { return image }
        return nil
    }
}

public final class ImageLoader {
    nonisolated(unsafe) public static let shared = ImageLoader()

    private var phases: [String: AsyncImagePhase] = [:]

    private var inFlight: Set<String> = []

    private let inbox = Inbox()

    private final class Inbox: @unchecked Sendable {
        private let lock = NSLock()
        private var items: [(key: String, data: Data?, failure: String?)] = []

        func deposit(key: String, data: Data?, failure: String?) {
            lock.lock()
            items.append((key, data, failure))
            lock.unlock()
        }

        func drain() -> [(key: String, data: Data?, failure: String?)] {
            lock.lock()
            defer { lock.unlock() }
            let out = items
            items.removeAll(keepingCapacity: true)
            return out
        }
    }

    public func drainPending() {
        for item in inbox.drain() {
            finish(key: item.key, data: item.data, failure: item.failure)
        }
    }

    public struct LoadError: Error, Sendable {
        public let message: String
    }

    public func phase(for url: URL) -> AsyncImagePhase {
        let key = url.absoluteString
        if let existing = phases[key] { return existing }
        start(url, key: key)
        return .empty
    }

    private func start(_ url: URL, key: String) {
        guard !inFlight.contains(key) else { return }
        inFlight.insert(key)

        let inbox = self.inbox
        URLSession.shared.dataTask(with: url) { data, _, error in

            inbox.deposit(key: key, data: data, failure: error?.localizedDescription)
        }
        .resume()
    }

    private func finish(key: String, data: Data?, failure: String?) {
        inFlight.remove(key)

        if let failure {
            phases[key] = .failure(LoadError(message: failure))
        } else if let data, !data.isEmpty {

            if let entry = ImageRegistry.shared.register(data, assetScale: 1) {
                phases[key] = .success(Image(entry: entry))
            } else {
                phases[key] = .failure(LoadError(message: "not a decodable PNG or JPEG"))
            }
        } else {
            phases[key] = .failure(LoadError(message: "empty response"))
        }

        NodeRegistry.shared.needsRender = true
    }
}

/// An image loaded from a URL, showing a placeholder until it arrives.
public struct AsyncImage<Content: View, Placeholder: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let url: URL?
    let content: (Image) -> Content
    let placeholder: () -> Placeholder

    public init(
        url: URL?,
        @ViewBuilder content: @escaping (Image) -> Content,
        @ViewBuilder placeholder: @escaping () -> Placeholder
    ) {
        self.url = url
        self.content = content
        self.placeholder = placeholder
    }
}

extension AsyncImage {
    public func toSFNode() -> SFNode {
        guard let url else { return placeholder().toSFNode() }

        switch ImageLoader.shared.phase(for: url) {
        case .success(let image):
            return content(image).toSFNode()
        case .empty, .failure:

            return placeholder().toSFNode()
        }
    }
}

extension AsyncImage where Content == Image, Placeholder == EmptyView {

    public init(url: URL?) {
        self.init(url: url, content: { $0 }, placeholder: { EmptyView() })
    }
}

public struct AsyncImagePhaseView<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let url: URL?
    let content: (AsyncImagePhase) -> Content

    public init(url: URL?, @ViewBuilder content: @escaping (AsyncImagePhase) -> Content) {
        self.url = url
        self.content = content
    }
}

extension AsyncImagePhaseView {
    public func toSFNode() -> SFNode {
        guard let url else { return content(.empty).toSFNode() }
        return content(ImageLoader.shared.phase(for: url)).toSFNode()
    }
}
