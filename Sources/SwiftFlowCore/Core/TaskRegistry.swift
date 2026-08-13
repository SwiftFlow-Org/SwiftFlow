//
//  TaskRegistry.swift
//  SwiftFlowCore
//
//  Created by Cel on 12/08/2026.
//

import CSwiftFlow

final class TaskRegistry {
    nonisolated(unsafe) static let shared = TaskRegistry()
    nonisolated(unsafe) private var running: [UInt32: Task<Void, Never>] = [:]

    func start(
        _ id: UInt32,
        priority: TaskPriority,
        operation: @escaping @Sendable () async -> Void
    ) {
        running[id]?.cancel()
        running[id] = Task(priority: priority) { await operation() }
    }

    func cancel(_ id: UInt32) {
        running.removeValue(forKey: id)?.cancel()
    }
}

public struct TaskModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let priority: TaskPriority
    let operation: @Sendable () async -> Void
}

extension TaskModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id

        let priority = self.priority
        let operation = self.operation
        LifecycleRegistry.shared.observe(
            id,
            appear: { TaskRegistry.shared.start(id, priority: priority, operation: operation) },
            disappear: { TaskRegistry.shared.cancel(id) }
        )
        return node
    }
}

extension View {

    /// Runs an async task while this view is on screen, cancelling it when the
    /// view goes away.
    ///
    /// The body runs off the main thread. Nothing in it may touch `@State`,
    /// `@Observed` or the framework's registries directly; hand results back
    /// through an inbox drained on the main thread, the way `AsyncImage` does.
    public func task(
        priority: TaskPriority = .userInitiated,
        _ operation: @escaping @Sendable () async -> Void
    ) -> TaskModifier<Self> {
        TaskModifier(content: self, priority: priority, operation: operation)
    }
}
