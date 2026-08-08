import CSwiftFlow
import QuartzCore
import SwiftFlowCore
import UIKit
import os

final class MetalView: UIView {
    let log = OSLog(subsystem: "com.swiftflow.test", category: "debug")

    override class var layerClass: AnyClass { CAMetalLayer.self }

    var metalLayer: CAMetalLayer { layer as! CAMetalLayer }

    private var displayLink: CADisplayLink?

    var rootView: (any View)?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        guard window != nil else { return }
        setupMetal()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateGeometry()

        guard displayLink != nil else { return }
        let scale = renderScale
        let size = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        if size != lastSurfaceSize {
            lastSurfaceSize = size

            swiftflow_resize(UInt32(size.width), UInt32(size.height))
        }

        NodeRegistry.shared.needsRender = true
    }

    private var lastSurfaceSize: CGSize = .zero

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        updateGeometry()

        NodeRegistry.shared.needsRender = true
    }

    private func updateGeometry() {
        let insets = safeAreaInsets
        SafeArea.top = Float(insets.top)
        SafeArea.bottom = Float(insets.bottom)
        SafeArea.leading = Float(insets.left)
        SafeArea.trailing = Float(insets.right)

        DeviceMetrics.screenWidth = Float(bounds.width)
        DeviceMetrics.screenHeight = Float(bounds.height)
    }

    private var renderScale: CGFloat { window?.screen.scale ?? UIScreen.main.scale }

    private func setupMetal() {
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.framebufferOnly = true

        let scale = renderScale

        metalLayer.contentsScale = scale
        contentScaleFactor = scale

        let width = UInt32(bounds.width * scale)
        let height = UInt32(bounds.height * scale)
        DeviceScale.current = Float(scale)
        updateGeometry()

        let layerPtr = Unmanaged.passUnretained(metalLayer).toOpaque()
        let surfaceDescriptor = SFSurfaceDescriptor(
            kind: SF_SURFACE_METAL_LAYER, handle: layerPtr, display_handle: nil)
        swiftflow_init(surfaceDescriptor, width, height)

        displayLink = CADisplayLink(target: self, selector: #selector(render))
        displayLink?.add(to: .main, forMode: .common)
        observeLifecycle()
    }

    private func observeLifecycle() {
        let center = NotificationCenter.default
        center.addObserver(
            self, selector: #selector(appWillResignActive),
            name: UIApplication.willResignActiveNotification, object: nil)
        center.addObserver(
            self, selector: #selector(appDidBecomeActive),
            name: UIApplication.didBecomeActiveNotification, object: nil)
    }

    @objc private func appWillResignActive() {

        displayLink?.isPaused = true
    }

    @objc private func appDidBecomeActive() {
        guard let displayLink else { return }
        swiftflow_surface_invalidated()
        displayLink.isPaused = false

        NodeRegistry.shared.needsRender = true
    }

    var cachedNode: SFNode?

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = touches.first, var node = cachedNode else { return }

        let point = touch.location(in: self)
        let scale = DeviceScale.current
        let px = Float(point.x) * scale
        let py = Float(point.y) * scale

        GestureRouter.shared.pointerDown(
            x: px, y: py, t: CACurrentMediaTime(),
            path: NodeBuilder.hitPath(&node, x: px, y: py),
            scrolls: NodeBuilder.scrollPath(&node, x: px, y: py)
        )
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = touches.first else { return }
        let point = touch.location(in: self)
        let scale = DeviceScale.current
        GestureRouter.shared.pointerMoved(
            x: Float(point.x) * scale,
            y: Float(point.y) * scale,
            t: CACurrentMediaTime()
        )
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = touches.first else { return }
        let point = touch.location(in: self)
        let scale = DeviceScale.current
        GestureRouter.shared.pointerUp(
            x: Float(point.x) * scale,
            y: Float(point.y) * scale,
            t: CACurrentMediaTime()
        )
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        GestureRouter.shared.pointerCancelled()
    }

    private var lastFrameTime: CFTimeInterval?

    @MainActor
    @objc private func render() {
        guard let rootView else { return }

        ImageLoader.shared.drainPending()

        let anyScrollActive = NodeRegistry.shared.scrollStates.values.contains {
            $0.isDragging || $0.isSettling
        }

        let anyAnimationActive = NodeRegistry.shared.hasActiveAnimations

        let anyGestureActive = GestureRouter.shared.needsFrames
        guard NodeRegistry.shared.needsRender || anyScrollActive || anyAnimationActive
            || anyGestureActive else { return }

        let now = CACurrentMediaTime()
        let dt = lastFrameTime.map { now - $0 } ?? (1.0 / 60.0)
        lastFrameTime = now
        stepScrollPhysics(dt: Float(dt))
        stepAnimations(dt: Float(dt))
        GestureRouter.shared.step(dt: Float(dt))

        let scale = Float(renderScale)
        DeviceScale.current = scale
        let width = Float(bounds.width) * scale
        let height = Float(bounds.height) * scale

        Screen.width = Float(bounds.width)
        Screen.height = Float(bounds.height)

        FrameArena.shared.reset()
        NodeFrames.shared.beginFrame()
        var node = NodeBuilder.build(rootView)
        sf_render_tree(&node, width, height, scale)

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

    private func stepScrollPhysics(dt: Float) {
        for (_, state) in NodeRegistry.shared.scrollStates {
            state.step(dt: dt)
        }
    }

    private func stepAnimations(dt: Float) {
        NodeRegistry.shared.stepAnimations(dt: dt)
    }
}

extension MetalView: UIKeyInput {
    var hasText: Bool { TextInput.shared.focused != nil }

    func insertText(_ text: String) {

        if text == "\n" {
            TextInput.shared.key(
                SF_KEY_ENTER.rawValue, modifiers: 0, pressed: true, isRepeat: false
            )
            return
        }
        TextInput.shared.commit(text)
    }

    func deleteBackward() {
        TextInput.shared.key(
            SF_KEY_BACKSPACE.rawValue, modifiers: 0, pressed: true, isRepeat: false
        )
    }
}

extension MetalView {
    override var canBecomeFirstResponder: Bool { true }

    func installTextInputBridge() {
        TextInput.shared.setIMEAllowed = { [weak self] allowed in
            guard let self else { return }
            if allowed {
                _ = self.becomeFirstResponder()
            } else {
                _ = self.resignFirstResponder()
            }
        }

        TextInput.shared.setIMECursorArea = nil
    }
}
