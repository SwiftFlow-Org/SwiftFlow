# SwiftFlow desktop

The platform layer for macOS, Linux and Windows. The iOS counterpart is
`../apple`; both re-export `SwiftFlowCore`, so an app's `import SwiftFlow`
and its `@main struct App: SwiftFlowApp` are identical on either.

## The one real difference: who owns the loop

On iOS, Swift drives everything — `CADisplayLink` calls into
`MetalView.render()`, which rebuilds the tree and submits a frame.

winit can't work that way. `EventLoop::run_app` takes the thread and
never returns, so on desktop the direction inverts:

```
Swift main()  →  sf_desktop_run()  →  winit event loop
                                          │
                                          ├── RedrawRequested → frame(dt)
                                          ├── MouseInput      → pointerDown/Up
                                          ├── MouseWheel      → scroll
                                          └── Resized         → resized(info)
                                                  ↓
                                            DesktopHost (Swift)
```

Swift still owns `main`, so nothing about the app-facing API changes. It
just hands over the thread instead of keeping it.

Everything above the pump is shared. `DesktopHost.frame(dt:)` is
deliberately the same code as `MetalView.render()` — same dirty check,
same physics order, same scroll-metrics readback — because a divergence
there would surface as "scrolling feels different on desktop", which is
a miserable class of bug to track down.

## Things that genuinely differ from iOS

- **Rust supplies the clock.** `MetalView` uses `CACurrentMediaTime()`
  for frame `dt` and touch-velocity timing; that's QuartzCore and does
  not exist off Apple. `dt` and event timestamps cross the FFI instead.
- **Wheel scrolling is not a drag.** A touch drag ends with a lift-off
  that hands momentum to the spring. A wheel has no such event, so
  `ScrollPhysicsState.applyWheel` clamps hard instead of rubber-banding
  — rubber-banding would strand content past its edge with nothing to
  pull it back.
- **Pointer moves only count while the button is down.** The pipeline
  this feeds has no concept of a hovering finger, so a bare mouse move
  must not look like a drag.
- **macOS runs edge to edge.** The window is created with a transparent,
  full-size content view, and `SafeArea.top` is set to the titlebar
  height, so a `NavigationStack`'s progressive blur sits under the
  traffic lights the way it sits under the iOS status bar.

## Building

```sh
.swiftflow/build-desktop.sh
```

Builds `swiftflow_desktop` for the host triple and `swift run`s the app
in `SwiftFlowTest` (override with `SWIFTFLOW_APP_DIR`) — the same app
directory `build-ios.sh` builds, not a desktop copy of it.

That works because the app's `Package.swift` reads `SWIFTFLOW_PLATFORM`,
which this script exports as `desktop`: an executable target depending
on this package, versus an iOS library target depending on `../apple`
when it's unset. The sources don't branch, only the manifest does.

Note that `Package.swift` cannot infer which Rust target directory to
link against — a manifest only sees the *host* it runs on, never the
target being built for. `SWIFTFLOW_RUST_TRIPLE` states it; both build
scripts set it, and the manifest falls back to the host triple.

Only `libswiftflow_desktop.a` gets linked on desktop. A Rust staticlib
embeds all of its upstream crates, so that one archive already exports
every `sf_*` symbol `swiftflow_wgpu` and `swiftflow_core` define; adding
`-lswiftflow_wgpu` as well would supply them twice.

## Requirements

wgpu needs Vulkan on Linux and Windows, Metal on macOS, or DX12 on
Windows. The GL backend is deliberately *not* enabled as a fallback: the
renderer binds storage buffers in vertex shaders (merged rects,
materials and images all do), which GLES does not reliably support, so
falling back to it would draw subtly wrong output instead of failing
cleanly. If no adapter is found, the error message says exactly this.
