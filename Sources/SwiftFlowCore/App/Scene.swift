import Foundation

public protocol Scene {
    associatedtype Body
    var body: Body { get }
    var lifecycle: SceneLifecycle { get }
    var windowConfig: WindowConfig { get set }
}

public extension Scene {
    var windowConfig: WindowConfig { .init() }
}

public struct AnyScene {
    let lifecycle: SceneLifecycle
    let windowConfig: WindowConfig
}

public struct SceneLifecycle {
    public var onForeground: (() -> Void)?
    public var onBackground: (() -> Void)?
    public var onTerminate: (() -> Void)?
    public var onOpenURL: ((URL) -> Void)?

    public init() {}
}

/// The app's window and its root view.
public struct WindowGroup<Content: View>: Scene {
    public let content: Content
    public var lifecycle: SceneLifecycle = .init()
    public var windowConfig: WindowConfig = .init()

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    public var body: Never { fatalError() }

    public func minWindowSize(width: Double, height: Double) -> Self {
        var copy = self
        copy.windowConfig.minWidth = width
        copy.windowConfig.minHeight = height
        return copy
    }

    public func onForeground(_ action: @escaping () -> Void) -> Self {
        var copy = self
        copy.lifecycle.onForeground = action
        return copy
    }

    public func onBackground(_ action: @escaping () -> Void) -> Self {
        var copy = self
        copy.lifecycle.onBackground = action
        return copy
    }

    public func onTerminate(_ action: @escaping () -> Void) -> Self {
        var copy = self
        copy.lifecycle.onTerminate = action
        return copy
    }

    public func onOpenURL(_ action: @escaping (URL) -> Void) -> Self {
        var copy = self
        copy.lifecycle.onOpenURL = action
        return copy
    }
}
