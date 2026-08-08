import SwiftSyntaxMacros
import SwiftSyntaxMacrosTestSupport
import XCTest

@testable import SwiftFlowMacrosPlugin

final class ObservableMacroTests: XCTestCase {
    let macros: [String: Macro.Type] = [
        "Observable": ObservableMacro.self,
        "ObservationTracked": ObservationTrackedMacro.self,
        "ObservationIgnored": ObservationIgnoredMacro.self,
    ]

    func testStoredPropertyBecomesObserved() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                var n = 0
            }
            """,
            expandedSource: """
            final class Store {
                var n = 0 {
                    @storageRestrictions(initializes: _n)
                    init(initialValue) {
                        _n = initialValue
                    }
                    get {
                        _observationRegistrar.access()
                        return _n
                    }
                    set {
                        _n = newValue
                        _observationRegistrar.didMutate()
                    }
                }

                private var _n = 0

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            macros: macros
        )
    }

    func testPropertyWithoutADefaultKeepsItsInitialiser() {
        assertMacroExpansion(
            """
            @Observable
            final class Document {
                var buffer: TextBuffer
                init(buffer: TextBuffer) {
                    self.buffer = buffer
                }
            }
            """,
            expandedSource: """
            final class Document {
                var buffer: TextBuffer {
                    @storageRestrictions(initializes: _buffer)
                    init(initialValue) {
                        _buffer = initialValue
                    }
                    get {
                        _observationRegistrar.access()
                        return _buffer
                    }
                    set {
                        _buffer = newValue
                        _observationRegistrar.didMutate()
                    }
                }

                private var _buffer: TextBuffer
                init(buffer: TextBuffer) {
                    self.buffer = buffer
                }

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Document: Observable {
            }
            """,
            macros: macros
        )
    }

    func testLetIsLeftAlone() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                let id = 1
            }
            """,
            expandedSource: """
            final class Store {
                let id = 1

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            macros: macros
        )
    }

    func testComputedPropertyIsLeftAlone() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                var doubled: Int { 2 }
            }
            """,
            expandedSource: """
            final class Store {
                var doubled: Int { 2 }

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            macros: macros
        )
    }

    func testObservationIgnoredIsLeftAlone() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                @ObservationIgnored var cache: [String: Int] = [:]
            }
            """,
            expandedSource: """
            final class Store {
                var cache: [String: Int] = [:]

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            macros: macros
        )
    }

    func testStaticPropertyIsLeftAlone() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                static var shared = 0
            }
            """,
            expandedSource: """
            final class Store {
                static var shared = 0

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            macros: macros
        )
    }

    func testDidSetIsDiagnosedRatherThanSilentlyDropped() {
        assertMacroExpansion(
            """
            @Observable
            final class Store {
                var n = 0 {
                    didSet { print(n) }
                }
            }
            """,
            expandedSource: """
            final class Store {
                var n = 0 {
                    didSet { print(n) }
                }

                private let _observationRegistrar = ObservationRegistrar()
            }

            extension Store: Observable {
            }
            """,
            diagnostics: [
                DiagnosticSpec(
                    message: MacroError.observerOnStoredProperty.message,
                    line: 3,
                    column: 5
                )
            ],
            macros: macros
        )
    }

    func testStructIsRejected() {
        assertMacroExpansion(
            """
            @Observable
            struct Store {
                var n = 0
            }
            """,
            expandedSource: """
            struct Store {
                var n = 0 {
                    @storageRestrictions(initializes: _n)
                    init(initialValue) {
                        _n = initialValue
                    }
                    get {
                        _observationRegistrar.access()
                        return _n
                    }
                    set {
                        _n = newValue
                        _observationRegistrar.didMutate()
                    }
                }

                private var _n = 0
            }

            extension Store: Observable {
            }
            """,
            diagnostics: [
                DiagnosticSpec(
                    message: MacroError.classesOnly.message,
                    line: 1,
                    column: 1
                )
            ],
            macros: macros
        )
    }

    func testExistingConformanceIsNotDuplicated() {
        assertMacroExpansion(
            """
            @Observable
            final class Store: Observable {
                var n = 0
            }
            """,
            expandedSource: """
            final class Store: Observable {
                var n = 0 {
                    @storageRestrictions(initializes: _n)
                    init(initialValue) {
                        _n = initialValue
                    }
                    get {
                        _observationRegistrar.access()
                        return _n
                    }
                    set {
                        _n = newValue
                        _observationRegistrar.didMutate()
                    }
                }

                private var _n = 0

                private let _observationRegistrar = ObservationRegistrar()
            }
            """,
            macros: macros
        )
    }
}
