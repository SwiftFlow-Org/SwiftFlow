import CSwiftFlow
import OSLog
@_exported import SwiftFlowCore
import UIKit
import os

extension SwiftFlowApp {
    public static func main() {
        setbuf(stdout, nil)

        let log = OSLog(subsystem: "com.swiftflow.test", category: "debug")
        NSSetUncaughtExceptionHandler { exception in
            os_log("[SWIFT] CRASH: %s", type: .fault, exception.description)
            os_log("[SWIFT] REASON: %s", type: .fault, exception.reason ?? "none")
        }

        UIApplicationMain(
            CommandLine.argc,
            CommandLine.unsafeArgv,
            nil,
            NSStringFromClass(SwiftFlowAppDelegate<Self>.self)
        )
    }
}

final class SwiftFlowAppDelegate<App: SwiftFlowApp>: UIResponder, UIApplicationDelegate {
    let log = OSLog(subsystem: "com.swiftflow.test", category: "debug")
    var window: UIWindow?
    var metalView: MetalView?

    private var app: App = App()
    private var lifecycle: SceneLifecycle = .init()

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        if let provider = app.body as? any LifecycleProvider {
            lifecycle = provider.lifecycle
            let root = provider.rootView

            let screen = UIScreen.main
            window = UIWindow(frame: screen.bounds)

            let vc = UIViewController()
            let mv = MetalView(frame: screen.bounds)
            mv.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            mv.rootView = root

            mv.installTextInputBridge()
            metalView = mv
            vc.view = mv

            window?.rootViewController = vc
            window?.makeKeyAndVisible()
        }
        return true
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        lifecycle.onBackground?()
    }

    func applicationWillEnterForeground(_ application: UIApplication) {
        lifecycle.onForeground?()
    }

    func applicationWillTerminate(_ application: UIApplication) {
        lifecycle.onTerminate?()
    }

    func application(
        _ app: UIApplication,
        open url: URL,
        options: [UIApplication.OpenURLOptionsKey: Any] = [:]
    ) -> Bool {
        lifecycle.onOpenURL?(url)
        return lifecycle.onOpenURL != nil
    }
}
