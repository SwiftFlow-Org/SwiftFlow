import CSwiftFlow

#if canImport(OSLog)
import OSLog
#endif

/// Prints to the platform's own log, which survives on a device where
/// `print` does not.
public enum Log {
    public static func print(_ items: Any...) {
        let message = items.map { "\($0)" }.joined(separator: " ")
        #if canImport(OSLog)
        Logger(subsystem: "app", category: "stdout").info("\(message)")
        #elseif os(Android)

        message.withCString { sf_log($0) }
        #else
        Swift.print(message)
        #endif
    }
}
