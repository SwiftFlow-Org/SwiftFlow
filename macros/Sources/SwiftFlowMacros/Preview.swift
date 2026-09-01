/// Declares a preview the `swiftflow preview` host renders in isolation.
///
///     #Preview("Counter") {
///         CounterView()
///     }
///
/// Expands to a discovery function whose name the CLI derives independently
/// while scanning source — the macro and `previews.rs` compute the same
/// `__SFPreview_<name>_<hash>` from the preview's name, so no runtime
/// registration is needed and the generated index can reference the symbol by
/// name.
///
/// Write it at file scope. The expansion is a `@_cdecl` thunk named
/// `__SFPreviewThunk`, carrying the per-preview C symbol the index reaches by
/// `@_silgen_name`.
///
/// LIMITATION (v1): the Swift name is fixed, because a freestanding declaration
/// macro may not introduce an *arbitrary* name at global scope, and the compiler
/// does not uniquify a fixed one across expansions. So there may be at most one
/// `#Preview` per app module for now — a second is a redeclaration error.
///
/// The body's type is an unconstrained generic rather than `some View` because
/// this package deliberately doesn't depend on SwiftFlowCore — the returned
/// value is used as a `View` where the expansion lands, in the app's own
/// compilation, which does.
@freestanding(declaration, names: named(__SFPreviewThunk))
public macro Preview<V>(_ name: String, _ body: () -> V) =
    #externalMacro(module: "SwiftFlowMacrosPlugin", type: "PreviewMacro")
