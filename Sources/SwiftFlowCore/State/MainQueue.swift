//
//  MainQueue.swift
//  SwiftFlowCore
//
//  Created by Cel on 13/08/2026.
//

import Foundation

/// Work to run on the render thread, posted from anywhere.
///
/// Background threads may set `needsRender` freely, but they may not touch
/// state a view reads: a pipe callback appending to an array races the build
/// that is reading it. Observation does not help — `@Observed`'s setter only
/// marks the tree dirty, it does not marshal anything. So the mutation itself
/// has to be handed to the render thread, and this is the one place that does
/// it rather than every async source growing its own lock and buffer.
///
/// There is no `DispatchQueue.main` to lean on here: the host is an event loop
/// calling `frame(dt:)`, and nothing services the dispatch main queue.
public enum MainQueue {
    private struct Item {
        let deadline: TimeInterval
        let work: () -> Void
    }

    private static let lock = NSLock()
    nonisolated(unsafe) private static var items: [Item] = []

    /// `systemUptime` rather than a wall clock: deadlines should survive the
    /// user changing their clock or a DST jump.
    private static var now: TimeInterval { ProcessInfo.processInfo.systemUptime }

    /// Runs on the next frame.
    public static func post(_ work: @escaping () -> Void) {
        post(after: 0, work)
    }

    /// Runs on the first frame after `delay`.
    ///
    /// The reason this is a deadline queue and not a list of closures: it is
    /// also the only clock available for debouncing, since the frame loop is
    /// the only thing that ticks.
    public static func post(after delay: TimeInterval, _ work: @escaping () -> Void) {
        lock.lock()
        items.append(Item(deadline: now + max(0, delay), work: work))
        lock.unlock()
        // Deliberately not setting needsRender: `drain` runs every frame
        // regardless, and flagging here would wake a rebuild for work that is
        // not due yet.
    }

    /// Called once a frame by the host, before it decides whether to rebuild —
    /// what runs here is usually what makes a rebuild necessary.
    public static func drain() {
        let current = now

        lock.lock()
        if items.isEmpty {
            lock.unlock()
            return
        }
        var due: [Item] = []
        var pending: [Item] = []
        for item in items {
            if item.deadline <= current {
                due.append(item)
            } else {
                pending.append(item)
            }
        }
        items = pending
        lock.unlock()

        guard !due.isEmpty else { return }

        // Outside the lock: work is allowed to post more work, and anything
        // it posts lands on the next frame rather than extending this drain
        // into a loop that never ends.
        for item in due { item.work() }
        NodeRegistry.shared.needsRender = true
    }
}
