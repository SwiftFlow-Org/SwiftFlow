/// The entry point of an app.
public protocol SwiftFlowApp {
    associatedtype Body: Scene
    @SceneBuilder var body: Body { get }

    init()
}

@resultBuilder
public struct SceneBuilder {
    public static func buildBlock<S: Scene>(_ scene: S) -> S {
        scene
    }
}

public protocol LifecycleProvider {
    var lifecycle: SceneLifecycle { get }
    var rootView: any View { get }
}

extension WindowGroup: LifecycleProvider {
    public var rootView: any View { content }
}
