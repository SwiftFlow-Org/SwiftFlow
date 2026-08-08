import CSwiftFlow
import Foundation

struct AnimatableSnapshot: Equatable {
    var scaleX: Float
    var scaleY: Float
    var fillR: Float
    var fillG: Float
    var fillB: Float
    var fillA: Float
    var cornerRadius: Float
    var borderR: Float
    var borderG: Float
    var borderB: Float
    var borderA: Float
    var borderWidth: Float
    var colorR: Float
    var colorG: Float
    var colorB: Float
    var colorA: Float
    var fontSize: Float
    var offsetX: Float
    var offsetY: Float
    var contentBlur: Float
    var fixedWidth: Float
    var fixedHeight: Float
    var paddingTop: Float
    var paddingBottom: Float
    var paddingLeading: Float
    var paddingTrailing: Float
    var spacing: Float

    var sizingX: SFSizing
    var sizingY: SFSizing

    init(
        scaleX: Float, scaleY: Float,
        fillR: Float, fillG: Float, fillB: Float, fillA: Float,
        cornerRadius: Float,
        borderR: Float, borderG: Float, borderB: Float, borderA: Float,
        borderWidth: Float,
        colorR: Float, colorG: Float, colorB: Float, colorA: Float,
        fontSize: Float,
        offsetX: Float, offsetY: Float,
        contentBlur: Float,
        fixedWidth: Float, fixedHeight: Float,
        paddingTop: Float, paddingBottom: Float,
        paddingLeading: Float, paddingTrailing: Float,
        spacing: Float,
        sizingX: SFSizing, sizingY: SFSizing
    ) {
        self.scaleX = scaleX
        self.scaleY = scaleY
        self.fillR = fillR
        self.fillG = fillG
        self.fillB = fillB
        self.fillA = fillA
        self.cornerRadius = cornerRadius
        self.borderR = borderR
        self.borderG = borderG
        self.borderB = borderB
        self.borderA = borderA
        self.borderWidth = borderWidth
        self.colorR = colorR
        self.colorG = colorG
        self.colorB = colorB
        self.colorA = colorA
        self.fontSize = fontSize
        self.offsetX = offsetX
        self.offsetY = offsetY
        self.contentBlur = contentBlur
        self.fixedWidth = fixedWidth
        self.fixedHeight = fixedHeight
        self.paddingTop = paddingTop
        self.paddingBottom = paddingBottom
        self.paddingLeading = paddingLeading
        self.paddingTrailing = paddingTrailing
        self.spacing = spacing
        self.sizingX = sizingX
        self.sizingY = sizingY
    }

    static let zero = AnimatableSnapshot(
        scaleX: 1, scaleY: 1,
        fillR: 0, fillG: 0, fillB: 0, fillA: 0,
        cornerRadius: 0,
        borderR: 0, borderG: 0, borderB: 0, borderA: 0,
        borderWidth: 0,
        colorR: 0, colorG: 0, colorB: 0, colorA: 0,
        fontSize: 0,
        offsetX: 0, offsetY: 0,
        contentBlur: 0,
        fixedWidth: 0, fixedHeight: 0,
        paddingTop: 0, paddingBottom: 0,
        paddingLeading: 0, paddingTrailing: 0,
        spacing: 0,
        sizingX: SF_SIZING_HUG, sizingY: SF_SIZING_HUG
    )

    static func extract(from node: SFNode) -> AnimatableSnapshot {
        AnimatableSnapshot(
            scaleX: node.scale.x, scaleY: node.scale.y,
            fillR: node.fill.r, fillG: node.fill.g, fillB: node.fill.b, fillA: node.fill.a,
            cornerRadius: node.cornerRadius,
            borderR: node.border.color.r, borderG: node.border.color.g,
            borderB: node.border.color.b, borderA: node.border.color.a,
            borderWidth: node.border.width,
            colorR: node.color.r, colorG: node.color.g, colorB: node.color.b, colorA: node.color.a,
            fontSize: node.fontSize,
            offsetX: node.offsetX, offsetY: node.offsetY,
            contentBlur: node.contentBlur,
            fixedWidth: node.fixedWidth, fixedHeight: node.fixedHeight,
            paddingTop: node.padding.top, paddingBottom: node.padding.bottom,
            paddingLeading: node.padding.leading, paddingTrailing: node.padding.trailing,
            spacing: node.spacing,
            sizingX: node.sizingX, sizingY: node.sizingY
        )
    }

    func apply(to node: inout SFNode) {
        node.scale = SFScale(x: scaleX, y: scaleY)
        node.fill = SFColor(r: fillR, g: fillG, b: fillB, a: fillA)
        node.cornerRadius = cornerRadius
        node.border = SFBorder(
            color: SFColor(r: borderR, g: borderG, b: borderB, a: borderA),
            width: borderWidth,
            _pad: (0, 0, 0)
        )
        node.color = SFColor(r: colorR, g: colorG, b: colorB, a: colorA)
        node.fontSize = fontSize
        node.offsetX = offsetX
        node.offsetY = offsetY
        node.contentBlur = contentBlur
        node.fixedWidth = fixedWidth
        node.fixedHeight = fixedHeight
        node.padding = SFEdgeInsets(
            top: paddingTop, bottom: paddingBottom,
            leading: paddingLeading, trailing: paddingTrailing
        )
        node.spacing = spacing

    }

    private var asArray: [Float] {
        [scaleX, scaleY, fillR, fillG, fillB, fillA, cornerRadius,
         borderR, borderG, borderB, borderA, borderWidth,
         colorR, colorG, colorB, colorA, fontSize,
         offsetX, offsetY, contentBlur,
         fixedWidth, fixedHeight,
         paddingTop, paddingBottom, paddingLeading, paddingTrailing,
         spacing]
    }

    private init(array a: [Float], sizingX: SFSizing, sizingY: SFSizing) {
        scaleX = a[0]; scaleY = a[1]
        fillR = a[2]; fillG = a[3]; fillB = a[4]; fillA = a[5]
        cornerRadius = a[6]
        borderR = a[7]; borderG = a[8]; borderB = a[9]; borderA = a[10]
        borderWidth = a[11]
        colorR = a[12]; colorG = a[13]; colorB = a[14]; colorA = a[15]
        fontSize = a[16]
        offsetX = a[17]; offsetY = a[18]
        contentBlur = a[19]
        fixedWidth = a[20]; fixedHeight = a[21]
        paddingTop = a[22]; paddingBottom = a[23]
        paddingLeading = a[24]; paddingTrailing = a[25]
        spacing = a[26]
        self.sizingX = sizingX
        self.sizingY = sizingY
    }

    static func lerp(_ a: AnimatableSnapshot, _ b: AnimatableSnapshot, _ t: Float) -> AnimatableSnapshot {
        let aArr = a.asArray
        let bArr = b.asArray
        var result = [Float](repeating: 0, count: aArr.count)
        for i in 0..<aArr.count {
            result[i] = aArr[i] + (bArr[i] - aArr[i]) * t
        }

        return AnimatableSnapshot(array: result, sizingX: b.sizingX, sizingY: b.sizingY)
    }

    static func springStep(
        current: AnimatableSnapshot, velocity: AnimatableSnapshot, target: AnimatableSnapshot,
        response: Float, dampingFraction: Float, dt: Float
    ) -> (current: AnimatableSnapshot, velocity: AnimatableSnapshot, settled: Bool) {
        let omega = (2 * Float.pi) / max(response, 0.01)
        let zeta = max(dampingFraction, 0.001)

        let curArr = current.asArray
        let velArr = velocity.asArray
        let tgtArr = target.asArray

        var newCur = [Float](repeating: 0, count: curArr.count)
        var newVel = [Float](repeating: 0, count: curArr.count)
        var settled = true

        for i in 0..<curArr.count {
            let x0 = curArr[i] - tgtArr[i]
            let v0 = velArr[i]
            let (x, v) = scalarSpringStep(x0: x0, v0: v0, omega: omega, zeta: zeta, dt: dt)
            newCur[i] = tgtArr[i] + x
            newVel[i] = v

            let scale = max(abs(tgtArr[i]), 1)
            if abs(x) > 0.01 * scale || abs(v) > 0.02 * scale { settled = false }
        }

        return (
            AnimatableSnapshot(array: newCur, sizingX: target.sizingX, sizingY: target.sizingY),
            AnimatableSnapshot(array: newVel, sizingX: target.sizingX, sizingY: target.sizingY),
            settled
        )
    }
}

private func scalarSpringStep(x0: Float, v0: Float, omega: Float, zeta: Float, dt: Float) -> (x: Float, v: Float) {
    if zeta < 1.0 {
        let omegaD = omega * sfSqrt(1 - zeta * zeta)
        let a = x0
        let b = (v0 + zeta * omega * x0) / omegaD
        let decay = sfExp(-zeta * omega * dt)
        let cosT = sfCos(omegaD * dt)
        let sinT = sfSin(omegaD * dt)
        let x = decay * (a * cosT + b * sinT)
        let v = decay * (-zeta * omega * (a * cosT + b * sinT) + omegaD * (-a * sinT + b * cosT))
        return (x, v)
    } else {

        let z = zeta == 1.0 ? 1.0001 : zeta
        let sqrtTerm = sfSqrt(z * z - 1.0)
        let r1 = omega * (-z + sqrtTerm)
        let r2 = omega * (-z - sqrtTerm)
        let a = (v0 - x0 * r2) / (r1 - r2)
        let b = x0 - a
        let e1 = sfExp(r1 * dt)
        let e2 = sfExp(r2 * dt)
        let x = a * e1 + b * e2
        let v = a * r1 * e1 + b * r2 * e2
        return (x, v)
    }
}
