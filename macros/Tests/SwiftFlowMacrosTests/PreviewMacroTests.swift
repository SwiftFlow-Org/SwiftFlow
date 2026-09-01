import SwiftSyntaxMacros
import SwiftSyntaxMacrosTestSupport
import XCTest

@testable import SwiftFlowMacrosPlugin

final class PreviewMacroTests: XCTestCase {
    let macros: [String: Macro.Type] = ["Preview": PreviewMacro.self]

    // The other half of the cross-language contract: these exact pairs are
    // pinned identically in cli/src/previews.rs (`symbols_match_the_pinned_contract`).
    // If the derivations ever diverge, one of these two tests fails.
    func testSymbolDerivationMatchesThePinnedContract() {
        XCTAssertEqual(PreviewSymbol.derive(from: "Counter"), "__SFPreview_Counter_fc0f33c3")
        XCTAssertEqual(PreviewSymbol.derive(from: "Sidebar Row"), "__SFPreview_Sidebar_Row_ed59821b")
        XCTAssertEqual(PreviewSymbol.derive(from: ""), "__SFPreview__811c9dc5")
    }

    func testExpandsToADiscoveryFunctionBoxingTheView() {
        assertMacroExpansion(
            """
            #Preview("Counter") {
                CounterView()
            }
            """,
            expandedSource: """
            @_cdecl("__SFPreview_Counter_fc0f33c3")
            func __SFPreviewThunk() -> UnsafeMutableRawPointer {
                Unmanaged.passRetained(PreviewBox({
                CounterView()
                        })).toOpaque()
            }
            """,
            macros: macros
        )
    }
}
