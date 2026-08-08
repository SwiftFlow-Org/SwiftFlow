import CSwiftFlow
import Foundation
@_exported import SwiftFlowCore

public final class AndroidHost {
    nonisolated(unsafe) public static let shared = AndroidHost()

    var rootView: (any View)?
    var lifecycle = SceneLifecycle()

    var cachedNode: SFNode?
    private var surface = SFAndroidSurfaceInfo(
        width: 1, height: 1, scale: 1,
        safeTop: 0, safeBottom: 0, safeLeading: 0, safeTrailing: 0,
        cornerRadius: 0
    )

    func applySurface(_ info: SFAndroidSurfaceInfo) {
        surface = info
        DeviceScale.current = info.scale

        let scale = info.scale > 0 ? info.scale : 1
        DeviceMetrics.screenWidth = Float(info.width) / scale
        DeviceMetrics.screenHeight = Float(info.height) / scale

        DeviceMetrics.screenCornerRadius = info.cornerRadius
        SafeArea.top = info.safeTop
        SafeArea.bottom = info.safeBottom
        SafeArea.leading = info.safeLeading
        SafeArea.trailing = info.safeTrailing

        NodeRegistry.shared.needsRender = true
    }

    func applyAssetsPath(_ path: String) {
        let url = URL(fileURLWithPath: path, isDirectory: true)
        AssetCatalog.searchRoots.insert(url, at: 0)
    }

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

    func pointerDown(x: Float, y: Float, t: Double) {
        guard var node = cachedNode else { return }
        GestureRouter.shared.pointerDown(
            x: x, y: y, t: t,
            path: NodeBuilder.hitPath(&node, x: x, y: y),
            scrolls: NodeBuilder.scrollPath(&node, x: x, y: y)
        )
    }

    func pointerMoved(x: Float, y: Float, t: Double) {
        GestureRouter.shared.pointerMoved(x: x, y: y, t: t)
    }

    func pointerUp(x: Float, y: Float, t: Double) {
        GestureRouter.shared.pointerUp(x: x, y: y, t: t)
    }

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
    AndroidHost.shared.frame(dt: dt)
}

private func onPointerDown(_ x: Float, _ y: Float, _ t: Double) {
    AndroidHost.shared.pointerDown(x: x, y: y, t: t)
}

private func onPointerMoved(_ x: Float, _ y: Float, _ t: Double) {
    AndroidHost.shared.pointerMoved(x: x, y: y, t: t)
}

private func onPointerUp(_ x: Float, _ y: Float, _ t: Double) {
    AndroidHost.shared.pointerUp(x: x, y: y, t: t)
}

private func onResized(_ info: SFAndroidSurfaceInfo) {
    AndroidHost.shared.applySurface(info)
}

private func onLifecycle(_ event: UInt32) {
    AndroidHost.shared.handleLifecycle(event)
}

private func onAssetsPath(_ path: UnsafePointer<CChar>?) {
    guard let path else { return }
    let string = String(cString: path)
    AndroidHost.shared.applyAssetsPath(string)
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

extension AndroidHost {

    func run() {

        TextInput.shared.setIMEAllowed = { sf_android_set_ime_allowed($0 ? 1 : 0) }
        TextInput.shared.setIMECursorArea = { sf_android_set_ime_cursor_area($0, $1, $2, $3) }

        let callbacks = SFAndroidCallbacks(
            frame: onFrame,
            pointerDown: onPointerDown,
            pointerMoved: onPointerMoved,
            pointerUp: onPointerUp,
            resized: onResized,
            lifecycle: onLifecycle,
            assetsPath: onAssetsPath,
            key: onKey,
            imePreedit: onIMEPreedit,
            imeCommit: onIMECommit,
            imeEnabled: onIMEEnabled
        )
        sf_android_run(callbacks)
    }
}
