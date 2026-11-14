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

        MainQueue.drain()
        ImageLoader.shared.drainPending()
        FolderPicker.shared.drainPending()
        FilePicker.shared.drainPending()

        let anyScrollActive = NodeRegistry.shared.scrollStates.values.contains {
            $0.isDragging || $0.isSettling
        }

        let anyAnimationActive = NodeRegistry.shared.hasActiveAnimations
            || Canvas.takeFrameRequest()

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
        // Cleared before the build, not after: whatever the build or the
        // frames recorded below mark dirty gets the next frame.
        NodeRegistry.shared.clearDirty()
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
    }

    /// Hands the host's latest tablet reading to `Stylus`, so the pointer
    /// event about to be routed carries it.
    private func readPen(t: Double) {
        var sample = SFPenSample()
        guard sf_desktop_pen_sample(&sample) != 0 else {
            Stylus.shared.current = nil
            return
        }
        Stylus.shared.current = StylusSample(
            tool: sample.tool == SF_PEN_ERASER.rawValue ? .eraser : .pen,
            pressure: sample.pressure,
            tiltX: sample.tiltX,
            tiltY: sample.tiltY,
            rotation: sample.rotation,
            timestamp: t
        )
    }

    @MainActor
    func pointerDown(x: Float, y: Float, t: Double) {
        readPen(t: t)
        guard var node = cachedNode else { return }
        GestureRouter.shared.pointerDown(
            x: x, y: y, t: t,
            path: NodeBuilder.hitPath(&node, x: x, y: y),
            scrolls: NodeBuilder.scrollPath(&node, x: x, y: y)
        )
    }

    @MainActor
    func secondaryClick(x: Float, y: Float, t: Double) {
        _ = t
        guard var node = cachedNode else { return }
        // Hit-testing stays in physical pixels — `cachedNode`'s frames were
        // baked out in that space by `sf_render_tree`. The anchor handed to
        // `MenuPresenter` is different: `MenuOverlay` positions and clamps
        // the panel against `Screen.width`/`height`, which are logical
        // points, so the anchor has to be too, or the panel lands at the
        // click position times the display's scale factor.
        let scale = surface.scale > 0 ? surface.scale : 1
        MenuPresenter.shared.handleSecondaryClick(
            path: NodeBuilder.hitPath(&node, x: x, y: y),
            at: Point(x: x / scale, y: y / scale)
        )
    }

    @MainActor
    func pointerMoved(x: Float, y: Float, t: Double) {
        readPen(t: t)
        GestureRouter.shared.pointerMoved(x: x, y: y, t: t)
    }

    @MainActor
    func pointerUp(x: Float, y: Float, t: Double) {
        readPen(t: t)
        GestureRouter.shared.pointerUp(x: x, y: y, t: t)
        // Hover was held while pressed; catch it up with where the press
        // ended.
        if var node = cachedNode {
            GestureRouter.shared.updateHover(path: NodeBuilder.hitPath(&node, x: x, y: y))
        }
    }

    /// How much ⌘ or Ctrl with the scroll wheel zooms, per point scrolled,
    /// as a power of e.
    static let zoomPerPoint = 0.0035

    /// The view a navigation gesture in progress belongs to.
    private var navigating: UInt32?

    @MainActor
    func scroll(x: Float, y: Float, dx: Float, dy: Float, phase: UInt32) {
        _ = phase
        guard var node = cachedNode else { return }

        // A view that navigates takes the wheel: ⌘ or Ctrl with it zooms,
        // anything else pans. The content follows the fingers, so it moves
        // against the scroll offset's direction.
        let router = GestureRouter.shared
        if let target = router.navigator(on: NodeBuilder.hitPath(&node, x: x, y: y)) {
            let scale = surface.scale > 0 ? surface.scale : 1
            let mods = KeyModifiers(rawValue: sf_desktop_modifiers())
            if mods.contains(.command) || mods.contains(.control) {
                // A wheel notch is several dozen points; this makes one
                // about 15%, and a trackpad's small steps smooth.
                let factor = Float(exp(Double(-dy / scale) * Self.zoomPerPoint))
                router.navigate(.zoom(factor: factor, anchor: router.localPoint(x: x, y: y, in: target)), on: target)
            } else {
                router.navigate(.pan(dx: -dx / scale, dy: -dy / scale), on: target)
            }
            return
        }

        GestureRouter.shared.wheel(
            x: x, y: y, dx: dx, dy: dy,
            scrolls: NodeBuilder.scrollPath(&node, x: x, y: y)
        )
    }

    @MainActor
    func navigate(kind: UInt32, x: Float, y: Float, a: Float, b: Float) {
        let router = GestureRouter.shared
        if kind == SF_NAVIGATE_END.rawValue {
            if let target = navigating { router.navigate(.end, on: target) }
            navigating = nil
            return
        }
        if navigating == nil, var node = cachedNode {
            navigating = router.navigator(on: NodeBuilder.hitPath(&node, x: x, y: y))
        }
        guard let target = navigating else { return }
        let scale = surface.scale > 0 ? surface.scale : 1
        let anchor = router.localPoint(x: x, y: y, in: target)
        switch kind {
        case SF_NAVIGATE_MAGNIFY.rawValue:
            router.navigate(.zoom(factor: max(0.05, 1 + a), anchor: anchor), on: target)
        case SF_NAVIGATE_ROTATE.rawValue:
            router.navigate(.rotate(radians: a, anchor: anchor), on: target)
        case SF_NAVIGATE_PAN.rawValue:
            router.navigate(.pan(dx: a / scale, dy: b / scale), on: target)
        default:
            break
        }
    }

    @MainActor
    func hover(x: Float, y: Float, t: Double) {
        guard var node = cachedNode else { return }
        GestureRouter.shared.pointerHover(x: x, y: y, t: t, path: NodeBuilder.hitPath(&node, x: x, y: y))
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

private func onSecondaryClick(_ x: Float, _ y: Float, _ t: Double) {
    MainActor.assumeIsolated { DesktopHost.shared.secondaryClick(x: x, y: y, t: t) }
}

private func onResized(_ info: SFDesktopSurfaceInfo) {
    MainActor.assumeIsolated { DesktopHost.shared.applySurface(info) }
}

private func onLifecycle(_ event: UInt32) {
    MainActor.assumeIsolated { DesktopHost.shared.handleLifecycle(event) }
}

private func onKey(_ key: UInt32, _ modifiers: UInt32, _ pressed: UInt32, _ isRepeat: UInt32) {
    let taken = TextInput.shared.key(key, modifiers: modifiers, pressed: pressed != 0, isRepeat: isRepeat != 0)
    if !taken {
        KeyCommands.shared.dispatch(code: key, modifiers: modifiers, pressed: pressed != 0, isRepeat: isRepeat != 0)
    }
}

private func onNavigate(_ kind: UInt32, _ x: Float, _ y: Float, _ a: Float, _ b: Float) {
    MainActor.assumeIsolated { DesktopHost.shared.navigate(kind: kind, x: x, y: y, a: a, b: b) }
}

private func onHover(_ x: Float, _ y: Float, _ t: Double) {
    MainActor.assumeIsolated { DesktopHost.shared.hover(x: x, y: y, t: t) }
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

/// An input event forwarded into an offscreen preview (from an editor pane).
/// Coordinates are in the preview's physical pixels — the same space the
/// windowed host's pointer callbacks use.
public enum PreviewInput {
    case pointerDown(x: Float, y: Float)
    case pointerMoved(x: Float, y: Float)
    case pointerUp(x: Float, y: Float)
    case scroll(x: Float, y: Float, dx: Float, dy: Float)
}

extension DesktopHost {

    /// Headless render loop for the preview pane: renders into an offscreen
    /// texture and hands each frame's BGRA pixels to `tick`, which publishes
    /// them (e.g. to shared memory). No window, no winit — driven here.
    ///
    /// v1 forces a render every tick rather than gating on `needsRender`, so a
    /// static preview still streams frames; that's simple and correct, if a
    /// little wasteful when nothing changes.
    @MainActor
    public func runOffscreen(
        root: any View,
        widthPoints: Int,
        heightPoints: Int,
        scale: Float,
        input: () -> [PreviewInput] = { [] },
        tick: (UnsafePointer<UInt8>, Int, Int) -> Void
    ) {
        rootView = root
        let pxW = UInt32(max(1, Int(Float(widthPoints) * scale)))
        let pxH = UInt32(max(1, Int(Float(heightPoints) * scale)))

        swiftflow_init_offscreen(pxW, pxH)
        applySurface(SFDesktopSurfaceInfo(
            width: pxW, height: pxH, scale: scale,
            safeTop: 0, safeBottom: 0, safeLeading: 0, safeTrailing: 0
        ))

        var buffer = [UInt8](repeating: 0, count: Int(pxW * pxH) * 4)
        let parent = getppid()
        while true {
            // Exit if our launcher (the editor) has gone away and we were
            // reparented to init — otherwise a closed pane leaks a renderer.
            if getppid() != parent || getppid() == 1 { break }

            // Feed forwarded input before rendering: pointer routing reads the
            // previous frame's node tree (cachedNode), and taps that fire mutate
            // state this frame then draws.
            for event in input() {
                let t = ProcessInfo.processInfo.systemUptime
                switch event {
                case let .pointerDown(x, y): pointerDown(x: x, y: y, t: t)
                case let .pointerMoved(x, y): pointerMoved(x: x, y: y, t: t)
                case let .pointerUp(x, y): pointerUp(x: x, y: y, t: t)
                case let .scroll(x, y, dx, dy): scroll(x: x, y: y, dx: dx, dy: dy, phase: 0)
                }
            }

            NodeRegistry.shared.needsRender = true
            frame(dt: 1.0 / 60.0)

            let n = buffer.withUnsafeMutableBufferPointer {
                sf_read_frame($0.baseAddress, $0.count)
            }
            if n == buffer.count {
                buffer.withUnsafeBufferPointer { tick($0.baseAddress!, Int(pxW), Int(pxH)) }
            }
            Thread.sleep(forTimeInterval: 1.0 / 60.0)
        }
    }

    @MainActor
    func run(title: String, width: Double, height: Double, min_width: Double, min_height: Double) {

        FolderPicker.shared.requestOpen = { sf_desktop_open_folder_dialog() }
        FolderPicker.shared.takePicked = {
            guard let raw = sf_desktop_take_picked_folder() else { return nil }
            return String(cString: raw)
        }

        FilePicker.shared.requestOpen = { extensions in
            extensions.joined(separator: ",").withCString { sf_desktop_open_file_dialog($0) }
        }
        FilePicker.shared.takePicked = {
            guard let raw = sf_desktop_take_picked_file() else { return nil }
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
            imeEnabled: onIMEEnabled,
            secondaryClick: onSecondaryClick,
            navigate: onNavigate,
            hover: onHover
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
