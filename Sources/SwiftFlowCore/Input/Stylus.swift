import CSwiftFlow

/// What is doing the pointing.
public enum PointerTool: Sendable, Equatable {
    case mouse
    case touch
    /// A pen's tip: a graphics tablet, a pen display, an Apple Pencil.
    case pen
    /// The eraser end of a pen, or a pen with its eraser button held.
    case eraser
}

/// One reading from a pen.
///
/// Fields a device does not report keep their neutral value: full pressure
/// for a pen that has no pressure sensor, no tilt, no rotation.
public struct StylusSample: Sendable, Equatable {
    public var tool: PointerTool

    /// 0 (barely touching) to 1 (as hard as the device measures).
    public var pressure: Float

    /// How far the pen leans from upright, in radians, along the screen's x
    /// (positive leaning right) and y (positive leaning toward the bottom).
    public var tiltX: Float
    public var tiltY: Float

    /// The pen's turn about its own axis, in radians, where the device
    /// reports it (Wacom Art Pen, Apple Pencil Pro).
    public var rotation: Float

    /// Seconds, on the host's monotonic clock.
    public var timestamp: Double

    public init(
        tool: PointerTool = .pen,
        pressure: Float = 1,
        tiltX: Float = 0,
        tiltY: Float = 0,
        rotation: Float = 0,
        timestamp: Double = 0
    ) {
        self.tool = tool
        self.pressure = pressure
        self.tiltX = tiltX
        self.tiltY = tiltY
        self.rotation = rotation
        self.timestamp = timestamp
    }

    /// The angle between the pen and the screen, π/2 when upright.
    public var altitude: Float {
        let lean = sfSqrt(tiltX * tiltX + tiltY * tiltY)
        return Float.pi / 2 - min(lean, Float.pi / 2)
    }

    /// Which way the pen leans, in radians from the positive x axis.
    public var azimuth: Float {
        sfAtan2(tiltY, tiltX)
    }
}

/// The pen as the host last saw it.
///
/// A host that reads a tablet sets `current` just before it hands the
/// matching pointer event to `GestureRouter`, and clears it when the pen
/// leaves proximity, so `.onPointer` handlers get the pen data that goes
/// with each position. Hosts with no tablet support never set it, and
/// pointer events then report `.mouse` or `.touch`.
public final class Stylus {
    nonisolated(unsafe) public static let shared = Stylus()

    /// The latest reading, or `nil` when no pen is in range.
    nonisolated(unsafe) public var current: StylusSample?

    /// Whether the host's pointer comes from a touchscreen rather than a
    /// mouse, when no pen is in range. Set once by the host.
    nonisolated(unsafe) public var pointerIsTouch = false

    /// The tool behind the next pointer event.
    var tool: PointerTool {
        current?.tool ?? (pointerIsTouch ? .touch : .mouse)
    }
}
