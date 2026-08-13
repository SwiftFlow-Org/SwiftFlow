import CSwiftFlow
import Foundation
@_exported import SwiftFlowCore

public final class DesktopHost {
    nonisolated(unsafe) public static let shared = DesktopHost()

    var rootView: (any View)?
    var lifecycle = SceneLifecycle()

    var cachedNode: SFNode?
    private var surface = SFDesktopSurfaceInfo(
        width: 1, height: 1, scale: 1,
        safeTop: 0, safeBottom: 0, safeLeading: 0, safeTrailing: 0
    )

    func applySurface(_ info: SFDesktopSurfaceInfo) {
        surface = info
        DeviceScale.current = info.scale

        let scale = info.scale > 0 ? info.scale : 1
        DeviceMetrics.screenWidth = Float(info.width) / scale
        DeviceMetrics.screenHeight = Float(info.height) / scale

        DeviceMetrics.screenCornerRadius = 0
        SafeArea.top = info.safeTop
        SafeArea.bottom = info.safeBottom
        SafeArea.leading = info.safeLeading
        SafeArea.trailing = info.safeTrailing

        NodeRegistry.shared.needsRender = true
    }

    @MainActor
    func frame(dt: Float) {
        guard let rootView else { return }

        ImageLoader.shared.drainPending()

        let anyScrollActive = NodeRegistry.shared.scrollStates.values.contains {
            $0.isDragging || $0.isSettling
        }

        let anyAnimationActive = NodeRegistry.shared.hasActiveAnimations

        let anyGestureActive = GestureRouter.shared.needsFrames
        guard NodeRegistry.shared.needsRender || anyScrollActive || anyAnimationActive
            || anyGestureActive else {
            return
        }

        for (_, state) in NodeRegistry.shared.scrollStates { state.step(dt: dt) }
        NodeRegistry.shared.stepAnimations(dt: dt)
        GestureRouter.shared.step(dt: dt)

        Screen.width = Float(surface.width) / surface.scale
        Screen.height = Float(surface.height) / surface.scale

        FrameArena.shared.reset()
        NodeFrames.shared.beginFrame()
        var node = NodeBuilder.build(rootView)
        sf_render_tree(&node, Float(surface.width), Float(surface.height), surface.scale)

        for (id, state) in NodeRegistry.shared.scrollStates {
            state.adopt(sf_get_scroll_metrics(&node, id))
        }

        for id in NodeFrames.shared.tracked {
            let frame = sf_get_node_frame(&node, id)
            NodeFrames.shared.record(id, frame: frame)

            ListRowHeights.shared.record(id, physicalHeight: frame.height)
        }

        cachedNode = node
        NodeRegistry.shared.clearDirty()
    }

    @MainActor
    func pointerDown(x: Float, y: Float, t: Double) {
        guard var node = cachedNode else { return }
        GestureRouter.shared.pointerDown(
            x: x, y: y, t: t,
            path: NodeBuilder.hitPath(&node, x: x, y: y),
            scrolls: NodeBuilder.scrollPath(&node, x: x, y: y)
        )
    }

    @MainActor
    func pointerMoved(x: Float, y: Float, t: Double) {
        GestureRouter.shared.pointerMoved(x: x, y: y, t: t)
    }

    @MainActor
    func pointerUp(x: Float, y: Float, t: Double) {
        GestureRouter.shared.pointerUp(x: x, y: y, t: t)
    }

    @MainActor
    func scroll(x: Float, y: Float, dx: Float, dy: Float, phase: UInt32) {
        _ = phase
        guard var node = cachedNode else { return }
        GestureRouter.shared.wheel(
            x: x, y: y, dx: dx, dy: dy,
            scrolls: NodeBuilder.scrollPath(&node, x: x, y: y)
        )
    }

    @MainActor
    func handleLifecycle(_ event: UInt32) {
        switch event {
        case SF_LIFECYCLE_FOREGROUND.rawValue: lifecycle.onForeground?()
        case SF_LIFECYCLE_BACKGROUND.rawValue: lifecycle.onBackground?()
        case SF_LIFECYCLE_TERMINATE.rawValue: lifecycle.onTerminate?()
        default: break
        }
    }
}

private func onFrame(_ dt: Float) {
    MainActor.assumeIsolated { DesktopHost.shared.frame(dt: dt) }
}

private func onPointerDown(_ x: Float, _ y: Float, _ t: Double) {
    MainActor.assumeIsolated { DesktopHost.shared.pointerDown(x: x, y: y, t: t) }
}

private func onPointerMoved(_ x: Float, _ y: Float, _ t: Double) {
    MainActor.assumeIsolated { DesktopHost.shared.pointerMoved(x: x, y: y, t: t) }
}

private func onPointerUp(_ x: Float, _ y: Float, _ t: Double) {
    MainActor.assumeIsolated { DesktopHost.shared.pointerUp(x: x, y: y, t: t) }
}

private func onScroll(_ x: Float, _ y: Float, _ dx: Float, _ dy: Float, _ phase: UInt32) {
    MainActor.assumeIsolated {
        DesktopHost.shared.scroll(x: x, y: y, dx: dx, dy: dy, phase: phase)
    }
}

private func onResized(_ info: SFDesktopSurfaceInfo) {
    MainActor.assumeIsolated { DesktopHost.shared.applySurface(info) }
}

private func onLifecycle(_ event: UInt32) {
    MainActor.assumeIsolated { DesktopHost.shared.handleLifecycle(event) }
}

private func onKey(_ key: UInt32, _ modifiers: UInt32, _ pressed: UInt32, _ isRepeat: UInt32) {
    TextInput.shared.key(key, modifiers: modifiers, pressed: pressed != 0, isRepeat: isRepeat != 0)
}

private func onIMEPreedit(_ text: UnsafePointer<CChar>?, _ begin: Int32, _ end: Int32) {
    guard let text else { return }
    TextInput.shared.setPreedit(String(cString: text), cursorBegin: Int(begin), cursorEnd: Int(end))
}

private func onIMECommit(_ text: UnsafePointer<CChar>?) {
    guard let text else { return }
    TextInput.shared.commit(String(cString: text))
}

private func onIMEEnabled(_ enabled: UInt32) {
    TextInput.shared.imeEnabled(enabled != 0)
}

extension DesktopHost {

    @MainActor
    func run(title: String, width: Double, height: Double, min_width: Double, min_height: Double) {

        FolderPicker.shared.requestOpen = { sf_desktop_open_folder_dialog() }
        FolderPicker.shared.takePicked = {
            guard let raw = sf_desktop_take_picked_folder() else { return nil }
            return String(cString: raw)
        }

        TextInput.shared.setIMEAllowed = { sf_desktop_set_ime_allowed($0 ? 1 : 0) }
        TextInput.shared.setIMECursorArea = { sf_desktop_set_ime_cursor_area($0, $1, $2, $3) }

        let callbacks = SFDesktopCallbacks(
            frame: onFrame,
            pointerDown: onPointerDown,
            pointerMoved: onPointerMoved,
            pointerUp: onPointerUp,
            scroll: onScroll,
            resized: onResized,
            lifecycle: onLifecycle,
            key: onKey,
            imePreedit: onIMEPreedit,
            imeCommit: onIMECommit,
            imeEnabled: onIMEEnabled
        )

        title.withCString { titlePtr in
            var config = SFDesktopConfig(title: titlePtr, width: width, height: height, min_width: min_width, min_height: min_height)
            var cbs = callbacks
            withUnsafePointer(to: &config) { configPtr in
                withUnsafePointer(to: &cbs) { cbsPtr in
                    sf_desktop_run(configPtr, cbsPtr)
                }
            }
        }

        exit(0)
    }
}
