import CSwiftFlow

/// The motion behind the tab bar's selection pill.
///
/// The pill is one shape that travels between items, not a background that
/// blinks on underneath each — so where it is cannot come out of the view
/// tree. It is a single spring in item-index space (2.5 is halfway between
/// the third item and the fourth), stepped with the rest of the frame's
/// animations and read back when the bar builds.
///
/// Item boxes are measured rather than computed. The bar hugs its labels, so
/// the slots are not all the same width, and interpolating between measured
/// boxes is what lets the pill change width as it travels the way iOS 26's
/// does. Two smaller springs ride along: `pop`, the bounce the newly selected
/// icon takes, `presence`, which fades the pill out when the selection lands
/// on a tab that sits outside the group, and `expansion`, which collapses the
/// group behind one icon while the trailing tab grows into a search field.
final class TabBarAnimator {
    nonisolated(unsafe) static let shared = TabBarAnimator()

    private var position: Float = 0
    private var velocity: Float = 0
    private var target: Float = 0
    private var travelling = false

    private(set) var pop: Float = 0
    private var popVelocity: Float = 0
    private var popping = false

    private var presence: Float = 0
    private var presenceTarget: Float = 0

    private var expansion: Float = 0
    private var expansionVelocity: Float = 0
    private var expansionTarget: Float = 0
    private var expanding = false

    private var itemCount = 0
    private var placed = false
    private var dragging = false

    private var naturalGroup: Float = 0
    private var naturalHeight: Float = 0
    private var naturalRow: Float = 0
    private var naturalAnchor: Float = 0
    private var naturalItemHeight: Float = 0

    var isAnimating: Bool {
        travelling || popping || expanding || presence != presenceTarget
    }

    /// How far the bar is through becoming a search field: 0 is tabs with a
    /// search button, 1 is a search field with the tabs behind one icon.
    var searchExpansion: Float { min(max(expansion, 0), 1) }

    /// The slot the group keeps showing once it has collapsed — wherever the
    /// selection was when it left.
    var anchorSlot: Int { Int(target.rounded()) }

    /// Points the pill at `index` within the group, where `nil` means the
    /// selection left the group entirely — the pill holds its place and fades
    /// rather than flying off to a slot that isn't there.
    func retarget(to index: Int?, itemCount count: Int, expanded: Bool) {
        presenceTarget = index == nil ? 0 : 1

        let wanted: Float = expanded ? 1 : 0
        if wanted != expansionTarget {
            expansionTarget = wanted
            expanding = true
        }

        let reshaped = count != itemCount
        itemCount = count

        guard let index else { return }
        let slot = Float(index)

        // Mid-drag the finger owns the target and the selection follows it, so
        // the selection landing here says nothing new. Writing the slot back
        // would hand the pill an integer to chase and turn a smooth drag into
        // a lurch between tabs.
        if dragging { return }

        // A different number of tabs is a different bar. Land on the new
        // layout rather than fly across one that just moved underneath.
        if !placed || reshaped {
            placed = true
            position = slot
            target = slot
            velocity = 0
            travelling = false
            presence = presenceTarget
            return
        }

        guard slot != target else { return }
        target = slot
        travelling = true
        popVelocity = TabBarMetrics.popImpulse
        popping = true
    }

    func step(dt: Float) {
        if travelling {
            // Under a finger the pill rides a stiffer spring, so it reads as
            // attached rather than chased.
            let response = dragging ? TabBarMetrics.pillDragResponse : TabBarMetrics.pillResponse
            let (x, v) = scalarSpringStep(
                x0: position - target, v0: velocity,
                omega: springOmega(response),
                zeta: TabBarMetrics.pillDamping, dt: dt
            )
            position = target + x
            velocity = v
            if abs(x) < 0.002 && abs(v) < 0.01 {
                position = target
                velocity = 0
                travelling = false
            }
        }

        if popping {
            let (x, v) = scalarSpringStep(
                x0: pop, v0: popVelocity,
                omega: springOmega(TabBarMetrics.popResponse),
                zeta: TabBarMetrics.popDamping, dt: dt
            )
            pop = x
            popVelocity = v
            if abs(x) < 0.002 && abs(v) < 0.01 {
                pop = 0
                popVelocity = 0
                popping = false
            }
        }

        if expanding {
            let (x, v) = scalarSpringStep(
                x0: expansion - expansionTarget, v0: expansionVelocity,
                omega: springOmega(TabBarMetrics.expansionResponse),
                zeta: TabBarMetrics.expansionDamping, dt: dt
            )
            expansion = expansionTarget + x
            expansionVelocity = v
            if abs(x) < 0.002 && abs(v) < 0.01 {
                expansion = expansionTarget
                expansionVelocity = 0
                expanding = false
            }
        }

        if presence != presenceTarget {
            let travel = dt / TabBarMetrics.presenceDuration
            presence = presenceTarget > presence
                ? min(presenceTarget, presence + travel)
                : max(presenceTarget, presence - travel)
        }
    }

    /// Hands the pill to the finger. The spring stays in charge — it just
    /// gets a target that moves with every event, which is what keeps the
    /// stretch (and the frame loop) working exactly as they do on a tap.
    func drag(to slot: Float) {
        dragging = true
        target = slot
        travelling = true
    }

    /// Lets go, on `slot`. Whatever speed the finger left behind carries into
    /// the landing.
    func endDrag(landingOn slot: Int) {
        dragging = false
        target = Float(slot)
        travelling = true
        popVelocity = TabBarMetrics.popImpulse
        popping = true
    }

    /// Turns a point along the row into a continuous slot by walking the
    /// measured item centres — the inverse of what `pill(measuring:)` does.
    /// Past either end it clamps.
    func slot(atRowX x: Float, measuring ids: [UInt32]) -> Float? {
        guard let origin = NodeFrames.shared.frame(for: ids.first ?? 0)?.inPoints() else {
            return nil
        }
        var centres: [Float] = []
        centres.reserveCapacity(ids.count)
        for id in ids {
            guard let box = NodeFrames.shared.frame(for: id)?.inPoints() else { return nil }
            centres.append(box.x - origin.x + box.width / 2)
        }
        guard let first = centres.first, let last = centres.last else { return nil }

        if x <= first { return 0 }
        if x >= last { return Float(centres.count - 1) }
        for i in 0..<(centres.count - 1) where x <= centres[i + 1] {
            let span = max(centres[i + 1] - centres[i], 0.001)
            return Float(i) + (x - centres[i]) / span
        }
        return Float(centres.count - 1)
    }

    /// The bar's two boxes for this frame.
    ///
    /// The group's width is left alone until it has to move: it hugs its
    /// items, which is both how it should look at rest and how it gets
    /// measured.
    func layout(measuring ids: [UInt32], group groupID: UInt32) -> TabBarLayout {
        let expansion = searchExpansion
        let atRest = TabBarLayout(
            rowWidth: nil, groupWidth: nil, groupHeight: nil,
            trailingWidth: TabBarMetrics.trailingDiameter,
            contentOffset: 0, contentRise: 0, expansion: expansion
        )

        // At rest is the only moment the group's natural size is on offer, so
        // everything the collapse needs is taken now. Reading it later would
        // chase itself: the labels inside are being squeezed by the closing
        // box and measure narrower every frame, which would narrow the box.
        if expansion <= 0.001 {
            if let box = NodeFrames.shared.frame(for: groupID)?.inPoints(), box.width > 0 {
                naturalGroup = box.width
                naturalHeight = box.height
            }
            if let row = rowGeometry(measuring: ids) {
                naturalRow = row.width
                naturalAnchor = row.anchorCentre
                naturalItemHeight = row.itemHeight
            }
            return atRest
        }

        // A circle, so the width it collapses to is its own height — and the
        // height is pinned alongside it, since a box that is only told its
        // width is at the mercy of whatever its content does on the way down.
        let collapsed = naturalHeight
        let available = Screen.width - 2 * TabBarMetrics.horizontalPadding
        guard collapsed > 0, naturalGroup > collapsed, available > naturalGroup else {
            return atRest
        }

        let trailing = TabBarMetrics.trailingDiameter
        let grown = max(trailing, available - TabBarMetrics.groupSpacing - collapsed)
        // Overflow pins to the leading edge here — `align_offset` never goes
        // negative — so the row's left edge is where the collapsing box's is,
        // and the anchor has to be walked from there to the middle.
        let centred = collapsed / 2 - TabBarMetrics.groupInset

        return TabBarLayout(
            rowWidth: naturalRow,
            groupWidth: naturalGroup + (collapsed - naturalGroup) * expansion,
            groupHeight: naturalHeight,
            trailingWidth: trailing + (grown - trailing) * expansion,
            contentOffset: (centred - naturalAnchor) * expansion,
            contentRise: iconRise * expansion,
            expansion: expansion
        )
    }

    /// How far the row has to come down for the anchor's *icon* to sit in the
    /// middle of the circle, rather than its box. An item is an icon above a
    /// label, so centring the box leaves the icon high by half the label.
    private var iconRise: Float {
        let iconCentre = TabBarMetrics.itemPaddingVertical + TabBarMetrics.iconSize / 2
        return max(naturalItemHeight / 2 - iconCentre, 0)
    }

    /// The row as it stands: how wide it is, and where the anchor item — the
    /// tab the group keeps showing once it is a single circle — sits along
    /// it, measured from its leading edge.
    private func rowGeometry(
        measuring ids: [UInt32]
    ) -> (width: Float, anchorCentre: Float, itemHeight: Float)? {
        guard !ids.isEmpty else { return nil }
        let anchor = min(max(anchorSlot, 0), ids.count - 1)
        guard
            let first = NodeFrames.shared.frame(for: ids[0])?.inPoints(),
            let last = NodeFrames.shared.frame(for: ids[ids.count - 1])?.inPoints(),
            let box = NodeFrames.shared.frame(for: ids[anchor])?.inPoints()
        else { return nil }

        return (
            last.x + last.width - first.x,
            box.x - first.x + box.width / 2,
            box.height
        )
    }

    /// How far the pill runs past its own width, as a fraction of it.
    private var stretch: Float {
        min(abs(velocity) * TabBarMetrics.pillStretchPerVelocity, TabBarMetrics.pillMaxStretch)
    }

    /// The pill's box for this frame, in points from the row's leading edge.
    ///
    /// `ids` are the item nodes in group order; a slot the renderer has not
    /// measured yet hides the pill rather than putting it at zero.
    func pill(measuring ids: [UInt32]) -> TabPillGeometry {
        guard placed, !ids.isEmpty, presence > 0.001 else { return .hidden }

        let low = min(max(Int(position), 0), ids.count - 1)
        let high = min(low + 1, ids.count - 1)
        let t = min(max(position - Float(low), 0), 1)

        guard
            let origin = NodeFrames.shared.frame(for: ids[0])?.inPoints(),
            let a = NodeFrames.shared.frame(for: ids[low])?.inPoints(),
            let b = NodeFrames.shared.frame(for: ids[high])?.inPoints()
        else { return .hidden }

        var width = a.width + (b.width - a.width) * t
        var x = (a.x + (b.x - a.x) * t) - origin.x

        // Glass reaches ahead of itself. Most of the give goes on the leading
        // edge, the rest trails, and the width it gains it loses in height.
        let give = stretch * width
        width += give
        if velocity > 0 {
            x -= give * (1 - TabBarMetrics.pillStretchBias)
        } else if velocity < 0 {
            x -= give * TabBarMetrics.pillStretchBias
        }

        return TabPillGeometry(
            x: x,
            width: width,
            squash: 1 - stretch * TabBarMetrics.pillSquash,
            opacity: presence
        )
    }
}

/// The tab group and the trailing tab, mid-morph, for one frame.
struct TabBarLayout {
    /// The row pinned to the width it had before any of this started, so the
    /// labels inside it don't re-wrap and re-pack as the group closes over
    /// them — the collapse is a cover, not a squeeze. `nil` at rest, where
    /// the row hugs and pinning it would only freeze a stale measurement.
    var rowWidth: Float?

    /// `nil` while the group is at its natural width, which is a hug and not
    /// a number.
    let groupWidth: Float?

    /// Pinned as soon as the group starts closing, so the box the width is
    /// racing toward cannot move underneath it.
    let groupHeight: Float?

    let trailingWidth: Float

    /// How far the row of items slides so the anchor ends up centred, across
    /// and then down.
    let contentOffset: Float
    let contentRise: Float

    let expansion: Float

    /// How much of an item is left as the group closes over it. The anchor
    /// keeps its icon; everything else is gone well before the box is small
    /// enough for the overflow to show, since nothing here clips.
    func itemOpacity(slot: Int, anchor: Int) -> Float {
        slot == anchor ? 1 : max(0, 1 - expansion * TabBarMetrics.collapseFadeRate)
    }

    /// The label goes with the box — a collapsed group is one icon.
    func labelOpacity(slot: Int, anchor: Int) -> Float {
        min(itemOpacity(slot: slot, anchor: anchor), 1 - expansion)
    }
}

/// Where the selection pill sits inside the group, for one frame.
struct TabPillGeometry {
    /// The leading edge, in points from the row's own leading edge.
    let x: Float
    let width: Float
    let squash: Float
    let opacity: Float

    static let hidden = TabPillGeometry(x: 0, width: 0, squash: 1, opacity: 0)
}

private func springOmega(_ response: Float) -> Float {
    (2 * Float.pi) / max(response, 0.01)
}
