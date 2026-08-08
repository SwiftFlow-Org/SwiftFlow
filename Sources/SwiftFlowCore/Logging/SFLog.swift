#if canImport(os)
import os
#endif

enum SFLog {
    static func debug(_ message: @autoclosure () -> String) {
        #if canImport(os)
        os_log("%{public}s", type: .debug, message())
        #endif
    }
}
