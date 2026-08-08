# CodeFlow

A code editor for SwiftFlow, written in SwiftFlow.

It renders and it edits: a monospaced grid, syntax highlighting, a caret
you can move with the keyboard or place with a click, and typing that
goes into the buffer. What it still doesn't do is in "What's missing"
below, which is the honest part of this file.

```
cd CodeFlow && swiftflow run
```

## Layout

```
SwiftFlow.toml            app identity, capabilities, the framework pin
Package.swift             branches per platform; resolves the pin itself
Sources/CodeFlow/
  CodeFlowApp.swift       @main
  ContentView.swift       the window: sidebar | tabs + editor | status bar
  Model/
    TextBuffer.swift      lines, positions, edits
    Document.swift        one open file
    Workspace.swift       the tree, the open documents, the selection
    Language.swift        extension → keywords, comment syntax
    SampleProject.swift   what it opens with, and the lexer's fixture
  Syntax/
    Token.swift           token kinds and a highlighted line
    Lexer.swift           one pass, line at a time, with carried state
    Highlighter.swift     runs the lexer down a file, and caches it
    Theme.swift           token kind → colour, plus the chrome
  Views/
    Metrics.swift         every fixed size, in one place
    EditorView.swift      the code pane, windowed
    LineRowView.swift     one line: gutter number + coloured runs
    FileTreeView.swift    the sidebar
    TabStripView.swift    open documents
    StatusBarView.swift   the bar along the bottom
```

## Three decisions worth knowing

**Only visible lines are built.** SwiftFlow rebuilds the whole view tree
every frame, and a line is several nodes — a gutter number plus one
`Text` per coloured run, about five on average. A 5000-line file drawn in
full would be 25,000+ nodes constructed 60 to 120 times a second in order
to show forty of them. `EditorView` builds a window instead, sized from
`ScrollGeometry` and padded top and bottom with blank blocks so the
content keeps its true scrollable height.

The offset comes from `ScrollGeometry` read *during* the build, which is
a SwiftFlow-specific capability and the reason this works: a windowed
list driven by a scroll callback into `@State` is one frame behind, and
one frame behind shows as a blank strip at the leading edge every time
you flick it.

**A monospaced grid, so hit-testing is arithmetic.** Every ASCII glyph in
JetBrains Mono advances exactly 0.6em, so the column under a click is a
division and the x of a caret is a multiplication — `Metrics` does both
and measures nothing. That matters more here than it looks: SwiftFlow has
no text-measurement call a view can make during a build, because layout
happens in Rust *after* the tree is handed over. With a proportional face
the only way to place a caret from a click would be for the layout pass
to hand back per-glyph positions, which it does not. The monospaced face
turns a missing feature into a constant.

**Observable global stores, not view properties.** `Workspace.shared` and
`Theme.current` are statics, following `Navigator` rather than inventing
a pattern. In an immediate-mode framework a store held in a plain
property on a view is a *new* store every frame. `@State` is the other
half of that answer and is right for a single value owned by one view; a
workspace is read by the sidebar, the tab strip, the editor and the
status bar at once, which is what makes it a global.

Their mutable properties are `@Observed`, and that is load-bearing rather
than decorative. Every host skips the rebuild when nothing has marked the
frame dirty, so before this a write that didn't happen inside a `Button`
action changed the model and left the screen alone. Every mutation here
*did* happen in a tap, which is exactly why the bug was invisible — the
first thing to hit it would have been opening a real folder from disk, or
an IME commit.

`@Observed` rather than the `@Observable` macro, for now: the macro lives
in the framework's `macros/` package and needs a swift-syntax plugin
built for the host. Switching is a per-property deletion once that plugin
is known to build on every platform this targets.

## One framework change was needed

`.expands()` (`Sources/SwiftFlowCore/Modifiers/Modifiers.swift`) is new,
and CodeFlow's shell could not be laid out without it.

Stacks hug by default; `.weight()` is deliberately ignored inside a
hugging container; and `.frame(maxHeight: .infinity)` *wraps* a stack in
a filling box rather than making the stack fill. Those three are
individually reasonable and together mean the obvious spelling of this
window — a column that fills the screen with the editor taking the slack
— silently drops its weights and collapses to the height of its text.
Nothing warns; the code reads correctly.

`.expands()` sets `Fill` on the container's own node in place, exactly as
`.weight()` sets its own field. `rust/swiftflow_core/tests/codeflow_shell_layout.rs`
pins both the working layout and the collapse it replaces.

## What's missing

Listed rather than left to be discovered.

- **No selection.** One caret, no anchor, so no shift-arrow, no
  select-all, no copy or paste of a range. The buffer's edit methods each
  take a single `Position`; a selection means giving them a range and
  giving the renderer a highlight band, and it is the largest single
  thing left.
- **No undo.** `TextBuffer.revision` counts edits but nothing records
  them. An undo stack wants the inverse of each operation, which is why
  the four mutating methods are the only way to change the text — there
  is exactly one place to capture them from.
- **No saving.** `Document.markSaved()` exists and nothing calls it: the
  sample project has no files on disk to write back to.
- **No horizontal scrolling.** Long lines are clipped by the pane. A
  nested horizontal `ScrollView` per line is the obvious approach and is
  untested.
- **The file tree is the built-in sample.** `Workspace.open(directoryAt:)`
  reads a real directory and works; nothing calls it, because there is no
  folder picker and no meaningful working directory on a phone.
- **The lexer guesses at types.** An identifier starting with a capital
  is `.type`. That is wrong for an uppercase constant, and it is what
  every highlighter without a type-checker does — the alternative is
  compiling the file in order to colour it.
- **`Highlighter`'s cache key is approximate.** It is
  `(document, lineCount, firstLine)`. Nothing mutates a buffer yet so it
  is never stale today; when editing lands it wants to become a revision
  counter bumped on write.

## Verification

The Swift is uncompiled — there is no Swift toolchain in the environment
this was written in. What *was* checked, by translating the algorithms
into a language that could run here:

- The lexer, against 17 cases: keywords, types, strings with escaped
  quotes, hex and decimal numbers, attributes, line comments, block
  comments opening and closing across lines, TOML's different comment
  syntax, and the empty line.
- The windowing arithmetic, swept over file sizes 0–5000, four viewport
  sizes and every scroll position including both overscroll directions,
  asserting that the built range is valid, that total content height
  never drifts, and that the visible band is always covered.
- The buffer and caret: insert, split, multi-line paste, backspace
  joining lines, forward delete pulling one up, auto-indent, the desired
  column surviving a short line, two-stage Home, and word movement —
  plus a 400-trial random sweep of 40 operations each, asserting after
  every one that the caret is in bounds and that no line has acquired an
  embedded newline.
- The caret hit-testing, both directions: that drawing a caret at column
  N and clicking it returns N for 0..199, that the left half of a
  character rounds before it and the right half after, and that clicks
  in the gutter and above the first line clamp instead of going negative.

The windowing sweep caught a crash: during a rubber-band past the bottom
`offset` overshoots the content, and the unclamped window ran `first`
past `last`, which traps in Swift. It is fixed and the fix is covered.

Everything else — that these views compose, that the layout is what the
comments claim — is unverified until it builds.
