import Foundation
import SwiftDiagnostics
import SwiftSyntax
import SwiftSyntaxMacros

public struct PreviewMacro: DeclarationMacro {
    public static func expansion(
        of node: some FreestandingMacroExpansionSyntax,
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        // The name is the first argument, and must be a plain string literal:
        // the CLI reads the very same characters out of source text, so it
        // can't be an expression only the compiler could evaluate.
        guard let first = node.arguments.first?.expression,
              let literal = first.as(StringLiteralExprSyntax.self),
              let name = literal.previewRawText
        else {
            context.diagnose(Diagnostic(node: Syntax(node), message: PreviewError.nameLiteral))
            return []
        }

        // The body is the trailing closure, or a second positional closure.
        let closure: ClosureExprSyntax
        if let trailing = node.trailingClosure {
            closure = trailing
        } else if let second = node.arguments.dropFirst().first?.expression
            .as(ClosureExprSyntax.self) {
            closure = second
        } else {
            context.diagnose(Diagnostic(node: Syntax(node), message: PreviewError.bodyClosure))
            return []
        }

        let symbol = PreviewSymbol.derive(from: name)

        // A `@_cdecl` thunk: the C symbol is the stable, per-preview name the
        // index reaches by `@_silgen_name` and the host can `dlsym`. The Swift
        // name is fixed (`__SFPreviewThunk`) so the macro introduces a *named*,
        // not arbitrary, declaration — the only kind allowed at global scope;
        // the compiler uniquifies it across expansions. `PreviewBox` resolves
        // through the app's `import SwiftFlow`, which re-exports SwiftFlowCore.
        // A top-level `@_cdecl` thunk. The C symbol is the stable, per-preview
        // name the index reaches by `@_silgen_name`; the Swift name is the fixed
        // `__SFPreviewThunk` because a freestanding macro may introduce only a
        // *named* (not arbitrary) declaration at global scope. That fixity is the
        // one-preview-per-module limitation (see the macro's doc comment).
        return [
            """
            @_cdecl(\(literal: symbol))
            func __SFPreviewThunk() -> UnsafeMutableRawPointer {
                Unmanaged.passRetained(PreviewBox(\(closure))).toOpaque()
            }
            """
        ]
    }
}

/// The one place the discovery symbol is spelled, matching `previews.rs`
/// byte-for-byte: sanitize the name's UTF-8 to an identifier tail, then a
/// zero-padded 8-hex FNV-1a of the same bytes for stability and uniqueness.
enum PreviewSymbol {
    static func derive(from name: String) -> String {
        var sanitized = ""
        for byte in name.utf8 {
            if isIdentifierByte(byte) {
                sanitized.unicodeScalars.append(UnicodeScalar(byte))
            } else {
                sanitized += "_"
            }
        }

        var hash: UInt32 = 2166136261
        for byte in name.utf8 {
            hash ^= UInt32(byte)
            hash = hash &* 16777619
        }

        return "__SFPreview_\(sanitized)_\(String(format: "%08x", hash))"
    }

    private static func isIdentifierByte(_ byte: UInt8) -> Bool {
        (byte >= 48 && byte <= 57)      // 0-9
            || (byte >= 65 && byte <= 90)   // A-Z
            || (byte >= 97 && byte <= 122)  // a-z
    }
}

extension StringLiteralExprSyntax {
    /// The characters between the quotes exactly as written in source —
    /// escapes left unresolved — because that is what the CLI's scanner sees
    /// and hashes. Interpolation has no source-stable text, so it's rejected.
    var previewRawText: String? {
        var text = ""
        for segment in segments {
            guard case .stringSegment(let piece) = segment else { return nil }
            text += piece.content.text
        }
        return text
    }
}

enum PreviewError: String, DiagnosticMessage {
    case nameLiteral
    case bodyClosure

    var message: String {
        switch self {
        case .nameLiteral:
            return """
                #Preview's first argument must be a plain string literal — the \
                CLI reads it out of source to name the preview.
                """
        case .bodyClosure:
            return "#Preview needs a trailing closure returning the view to show."
        }
    }

    var diagnosticID: MessageID { MessageID(domain: "SwiftFlowMacros", id: rawValue) }
    var severity: DiagnosticSeverity { .error }
}
