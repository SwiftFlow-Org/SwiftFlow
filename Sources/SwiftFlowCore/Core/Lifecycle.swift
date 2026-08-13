//
//  Lifecycle.swift
//  SwiftFlowCore
//
//  Created by Cel on 12/08/2026.
//

import CSwiftFlow

final class LifecycleRegistry {
    nonisolated(unsafe) static let shared = LifecycleRegistry()

    private var alive: Set<UInt32> = []
    private var seenThisFrame: Set<UInt32> = []
    private var appeared: [() -> Void] = []
    private var disappearing: [UInt32: () -> Void] = [:]

    func observe(_ id: UInt32, appear: (() -> Void)?, disappear: (() -> Void)?) {
        seenThisFrame.insert(id)
        if let disappear { disappearing[id] = disappear }
        if !alive.contains(id), let appear { appeared.append(appear) }
    }

    func beginBuild() {
        seenThisFrame.removeAll(keepingCapacity: true)
    }

    // Callbacks fire after the walk, never during it. A handler that touches
    // state would otherwise mutate a registry the build is still reading.
    func endBuild() {
        for id in alive.subtracting(seenThisFrame) {
            disappearing.removeValue(forKey: id)?()
        }
        alive = seenThisFrame
        seenThisFrame.removeAll(keepingCapacity: true)

        let firing = appeared
        appeared.removeAll(keepingCapacity: true)
        for action in firing { action() }
    }

    func rekey(from old: UInt32, to new: UInt32) {
        if alive.remove(old) != nil { alive.insert(new) }
        if seenThisFrame.remove(old) != nil { seenThisFrame.insert(new) }
        if let d = disappearing.removeValue(forKey: old) { disappearing[new] = d }
    }
}

public struct LifecycleModifier<Content: View>: View {
    public typealias Body = Never
    public var body: Never { fatalError() }

    let content: Content
    let appear: (() -> Void)?
    let disappear: (() -> Void)?
}

extension LifecycleModifier {
    public func toSFNode() -> SFNode {
        var node = content.toSFNode()
        let id = node.node_id != 0 ? node.node_id : BuildContext.shared.currentID(for: self)
        node.node_id = id
        LifecycleRegistry.shared.observe(id, appear: appear, disappear: disappear)
        return node
    }
}

extension View {

    /// Adds an action to run when this view appears.
    public func onAppear(perform action: @escaping () -> Void) -> LifecycleModifier<Self> {
        LifecycleModifier(content: self, appear: action, disappear: nil)
    }

    /// Adds an action to run when this view disappears.
    public func onDisappear(perform action: @escaping () -> Void) -> LifecycleModifier<Self> {
        LifecycleModifier(content: self, appear: nil, disappear: action)
    }
}
