import Foundation

@inline(__always) func sfSqrt(_ x: Float) -> Float {
    Float(Double(x).squareRoot())
}
@inline(__always) func sfExp(_ x: Float) -> Float { Float(exp(Double(x))) }
@inline(__always) func sfSin(_ x: Float) -> Float { Float(sin(Double(x))) }
@inline(__always) func sfCos(_ x: Float) -> Float { Float(cos(Double(x))) }
@inline(__always) func sfPow(_ x: Float, _ y: Float) -> Float {
    Float(pow(Double(x), Double(y)))
}
