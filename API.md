# SwiftFlow — the app-facing API

Everything an app can reach. If it isn't here, it's internal to the
framework and will move without warning.

The shape is SwiftUI's, and where a name matches SwiftUI the behaviour is
meant to match too. The places it deliberately doesn't are marked
**Differs**. `ARCHITECTURE.md` covers the why; this is the what.

Sizes are in **points** (`Float`, not `CGFloat` — there is no CoreGraphics
here). Points become physical pixels at the FFI boundary.

---

## Contents

- [App and scenes](#app-and-scenes)
- [Views](#views)
  - [Primitives](#primitives)
  - [Stacks and layout containers](#stacks-and-layout-containers)
  - [Scrolling](#scrolling)
  - [Lists](#lists)
  - [Controls](#controls)
  - [Shapes](#shapes)
  - [Empty states](#empty-states)
- [Modifiers](#modifiers)
  - [Layout](#layout-modifiers)
  - [Appearance](#appearance-modifiers)
  - [Materials and glass](#materials-and-glass)
  - [Gestures](#gestures)
  - [Text input](#text-input)
  - [Swipe actions](#swipe-actions)
- [Navigation](#navigation)
- [Tabs](#tabs)
- [Toolbars](#toolbars)
- [Animation and transitions](#animation-and-transitions)
- [State](#state)
- [Environment](#environment)
- [Types](#types)
  - [Color](#color)
  - [Font](#font)
  - [Icon and Image](#icon-and-image)
  - [Alignment, Edge, EdgeInsets](#alignment-edge-edgeinsets)
- [Device and screen](#device-and-screen)
- [Lifecycle](#lifecycle)
- [Logging](#logging)

---

## App and scenes

```swift
@main
struct MyApp: SwiftFlowApp {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
```

| Symbol | Notes |
|---|---|
| `protocol SwiftFlowApp` | `init()` + `@SceneBuilder var body: Body` |
| `protocol Scene` | |
| `struct WindowGroup<Content: View>: Scene` | `init(@ViewBuilder content:)` |
| `@resultBuilder SceneBuilder` | |

**Differs:** one scene. No `Settings`, no `DocumentGroup`, no multi-window.

---

## Views

```swift
protocol View {
    associatedtype Body: View
    @ViewBuilder var body: Body { get }
}
```

Two refinements exist for the framework's own types and are usable by
apps that build nodes directly:

- `protocol PrimitiveView: View where Body == Never` — has no body; must
  implement `toSFNode() -> SFNode`.
- `protocol RecursiveView: View where Body: View` — the ordinary case.

`AnyView` erases: `AnyView(erasing: someView)` or `AnyView(someView)`.

`ViewBuilder` supports `if`/`else`, `if let`, `switch`, and variadic
blocks. There is no ten-child limit — it uses parameter packs.

### Primitives

| View | Init |
|---|---|
| `Text` | `Text(_ content: String)` |
| `Image` | `Image(_ name: String)`, `Image(named:)`, `Image.system(_:)` |
| `AsyncImage` | `AsyncImage(url:)`, `AsyncImage(url:content:)` |
| `Icon` | `Icon.<name>` from the generated catalogue, or `Icon(scalar:)` |
| `TextField` | `TextField(_ placeholder: String, text: Binding<String>, font: Font = .body, onSubmit: (() -> Void)? = nil)` |
| `Divider` | `Divider()` |
| `Spacer` | `Spacer(minLength: Float = 0)` |
| `EmptyView` | `EmptyView()` |
| `Color` | is itself a view — `Color.red` fills its box |

`Text` chaining (each returns `Text`, so they compose):

```swift
Text("Hello")
    .font(.headline)
    .fontWeight(.semibold)
    .foregroundColor(.primary)     // or .foregroundStyle(_:)
    .lineLimit(2)                  // nil = unlimited
    .multilineTextAlignment(.center)
```

`.multilineTextAlignment(_:)` also exists as a `View` modifier, but it
writes the field on whatever node the content produced and stops there —
it does not walk into a stack and re-align every string inside it, which
is what SwiftUI's does. Put it on the `Text`.

`TextAlignment` is `.leading` / `.center` / `.trailing`. It is its own
type rather than `HorizontalAlignment` because it answers a different
question — where *lines* sit inside one view, not where a *view* sits
inside its parent — and the two are routinely opposite.

Only visible when the box is wider than a line: a paragraph that wrapped,
or a `Text` given a `.frame(width:)`. Alignment is applied **per line**,
so a centred paragraph centres each line rather than centring the block.

`Image`:

```swift
Image("photo")
    .resizable()
    .scaledToFit()                 // or .scaledToFill()
    .aspectRatio(contentMode: .fit)
    .foregroundColor(.accent)      // tint
```

`Icon`:

```swift
Icon.house
    .size(24)
    .weight(.bold)
    .foregroundColor(.primary)
```

### Stacks and layout containers

```swift
VStack(alignment: HorizontalAlignment = .center, spacing: Float = 8) { … }
HStack(alignment: VerticalAlignment = .center, spacing: Float = 8) { … }
ZStack(alignment: Alignment = .center) { … }
```

`ForEach` over a collection, keyed by identity:

```swift
ForEach(items, id: \.id) { item in Row(item) }
ForEach(items) { item in Row(item) }          // Identifiable
ForEach(0..<5, id: \.self) { i in … }
```

**Differs:** `ForEach` is the only place the framework can tell a
*removal* from a *rebuild*, so it is what makes insertion and removal
animate rather than cut. Use it even for a fixed handful of rows if they
come and go.

**Differs:** stacks are **Hug** by default on both axes. Use `.expands()`
to make a stack itself fill (see [Layout](#layout-modifiers)).

### Scrolling

```swift
ScrollView(_ axis: Axis = .vertical, name: String? = nil) { … }
ScrollView(.horizontal) { geometry in … }     // ScrollGeometry form
```

`ScrollGeometry` gives the live offset and extents, which is what
virtualised content is built from:

```swift
public let offset: Float          // current scroll position, points
public let viewportLength: Float
public let contentLength: Float
```

`name:` pins a scroll view's identity when the call site moves. Without
it, identity comes from `#fileID`/`#line`/`#column`.

**Differs:** a horizontal `ScrollView` must be told its height —
`.frame(height:)` — because its content decides how far it runs sideways,
not how tall it is.

### Lists

Windowed: only the visible rows are built, however long the data is.

```swift
List { Text("One"); Text("Two") }              // static rows

List(items, id: \.id) { item in Row(item) }    // keyed
List(items) { item in Row(item) }              // Identifiable
```

Styling:

```swift
List(items) { … }
    .listStyle(.inset)              // .inset (default) | .plain
    .listRowSeparator(false)
    .listRowInsets(EdgeInsets(top: 8, bottom: 8, leading: 16, trailing: 16))
```

**Differs:** rows size themselves. Nothing has to be told a row height —
a row that has been on screen is measured, one that hasn't is estimated
until it is.

### Controls

```swift
@State private var name = ""

TextField("Your name", text: $name)
TextField("Search", text: $query, font: .title3) { runSearch() }   // onSubmit
```

`TextField` is the whole input path in one view — focus on tap, committed
text, IME preedit drawn underlined at the caret, backspace/delete, arrows,
home/end, enter (`onSubmit`), escape to resign focus. It exists partly so
there is one thing to drop into an app to find out whether typing works
on a platform at all; the three hosts each reach the framework through a
different file.

**Differs:** tapping puts the caret at the end rather than at the
character you tapped. Glyph positions live in the renderer and nothing
reports them back, so there is nothing to hit-test against yet.

For an editor — anything that wants raw keys rather than a field — use
`.textInput(onInsert:onKey:)` directly. See [Text input](#text-input).

```swift
Button("Save") { save() }
Button { save() } label: { Label() }
```

`ButtonStyle` for custom looks:

```swift
public protocol ButtonStyle {
    associatedtype Body: View
    func makeBody(configuration: ButtonStyleConfiguration) -> Body
}
```

`ButtonStyleConfiguration` carries `label: AnyView` and `isPressed: Bool`.

### Shapes

`RoundedRectangle(cornerRadius:)`, `Circle()`, `Capsule()`. Each takes:

```swift
.fill(_ color: Color)         -> FilledShape
.fill(_ material: Material)   -> MaterialFilledShape
.stroke(_ color: Color, lineWidth: Float = 1)
```

A `cornerRadius` of `-1` means fully rounded (the shader resolves it to
`min(width, height) / 2` after layout).

### Empty states

```swift
ContentUnavailableView("No results", icon: .magnifyingGlass,
                       description: "Try a different search")

ContentUnavailableView(icon: .tray, description: "Nothing here yet") {
    Button("Add one") { add() }
}

ContentUnavailableView.search                  // the stock search case
ContentUnavailableView.search(text: query)
```

**Differs:** an empty `List` renders an empty list. Showing this instead
is an explicit `if items.isEmpty`, not automatic.

---

## Modifiers

### Layout modifiers

```swift
.frame(width: Float? = nil, height: Float? = nil,
       minWidth: Float? = nil, maxWidth: Float? = nil,
       minHeight: Float? = nil, maxHeight: Float? = nil,
       alignment: Alignment? = nil)

.padding()                          // 16 on all edges
.padding(_ all: Float)
.padding(_ edges: Edge, _ value: Float = 20)
.padding(_ insets: EdgeInsets)

.offset(x: Float = 0, y: Float = 0)  // render-time; siblings don't reflow
.weight(_ weight: Float = 1)
.expands()
```

**Arguments must be written in declaration order** — `width, height,
minWidth, maxWidth, minHeight, maxHeight, alignment`. `.frame(maxWidth:
.infinity, height: 44)` is a compile error; `.frame(height: 44, maxWidth:
.infinity)` is what you meant. A lint in the Rust test suite catches this.

**Sizing is per axis.** Each axis of `.frame()` answers one of three
questions independently — was a size named, was `.infinity` named, or was
nothing named (in which case the axis inherits Fill from the content, and
hugs it otherwise). So:

```swift
.frame(maxWidth: .infinity)              // full width, own height
.frame(width: 74)                        // 74 wide, own height
.frame(width: 100, maxHeight: .infinity) // both take effect
.frame(height: 0)                        // really zero, not "unstated"
```

**`.weight()` vs `.expands()` vs `.frame(maxWidth: .infinity)`** — these
are three different things and the difference bites:

- `.frame(maxWidth: .infinity)` **wraps** the view in a filling box. The
  view inside is unchanged, so a wrapped `VStack` is *still hugging*, and
  weights inside it are dropped.
- `.expands()` sets Fill on the view's **own** node — no wrapper. This is
  what a container needs so its children have leftover space to divide.
- `.weight(_:)` takes a **share** of a stack's leftover main-axis space.
  Fill means *all* of the space; weight means a share of it. Two filling
  children in one stack both take the whole thing and the second lands
  off the end of the first.

```swift
VStack {
    Editor().weight(1)      // gets everything the status bar leaves
    StatusBar()
}
.expands()                  // ← not .frame(maxHeight: .infinity)
```

`Spacer()` is weight 1 in the same pool, so `A.weight(1)` beside a
`Spacer()` splits in half. Weight is **ignored in a hugging container** —
"a share of the leftover" is meaningless when the container is defined as
the sum of its children.

### Appearance modifiers

```swift
.background<V: View>(_ background: V)
.cornerRadius(_ radius: Float)
.clipShape<S: View>(_ shape: S)
.opacity(_ value: Float)
.scale(_ value: Float)
.blur(radius: Float)
.shadow(radius: Float, opacity: Float = 0.25)
.specular()                        // opt-in Liquid-Glass edge trim
.mergeable(_ allowed: Bool)        // opt out of the metaball merge
.font(_:) .fontWeight(_:) .fontDesign(_:) .foregroundColor(_:)
```

**Differs:** `.padding()` and `.frame()` build a real node; everything
else writes a field on the existing one. So `.padding(20).frame(width:
200)` is 200 wide overall and `.frame(width: 200).padding(20)` is 240 —
but `.padding(10).cornerRadius(4)` and `.cornerRadius(4).padding(10)` are
the same tree, where SwiftUI's differ.

**Differs:** clipping is opt-in and does not clip. `.clipShape()` rounds
coextensive children; a child that overflows its parent still draws. See
ARCHITECTURE.md.

### Materials and glass

```swift
Material.ultraThinMaterial   // blur 3
Material.thinMaterial        // blur 5
Material.regularMaterial     // blur 7
Material.thickMaterial       // blur 9
Material.ultraThickMaterial  // blur 11

material.progressive(start: Float = 0.5)   // blur ramps out toward the bottom
.progressiveBlur(_ material: Material = .regularMaterial, start: Float = 0.5)
```

Glass — a material that also refracts:

```swift
Glass.regular
Glass.clear
glass.tint(_ color: Color)
glass.interactive(_ enabled: Bool = true)   // press response

.glassEffect(_ glass: Glass = .regular)
.glassEffect(_ glass: Glass = .regular, in shape: S)
```

### Gestures

```swift
.onTap(count: Int = 1, perform: @escaping (Point) -> Void)

.gesture(DragGesture(axis: Axis))
.gesture(LongPressGesture())
```

```swift
DragGesture(...)
    .onChanged { (v: DragValue) in … }
    .onEnded   { (v: DragValue) in … }

LongPressGesture().onEnded { … }
```

`DragValue` carries `location`, `startLocation`, `translation`,
`velocity` (all `Point`).

**Differs:** a `DragGesture(axis: .horizontal)` inside a vertical
`ScrollView` arbitrates by first movement — a drag that starts vertically
stays a scroll for its whole life, and vice versa. No cancellation
mid-gesture.

### Text input

```swift
.textInput(
    isFocused: Bool = true,
    onInsert: @escaping (String) -> Void,
    onKey: @escaping (Key, KeyModifiers) -> Bool = { _, _ in false }
)
```

`onKey` returns `true` to consume. `Key` is a `UInt32` enum of physical
keys; `KeyModifiers` is an option set of `.shift`, `.control`, `.option`,
`.command`.

### Swipe actions

Not confined to `List` — any view inside a scrollable container.

```swift
.swipeActions(edge: SwipeEdge = .trailing) {
    SwipeAction(icon: .trash, tint: .destructive, role: .destructive) {
        delete()
    }
    SwipeAction("Flag", icon: .flag, tint: .orange) { flag() }
}
```

`SwipeEdge` is `.leading` / `.trailing`. `Role` is `.normal` /
`.destructive`.

---

## Navigation

```swift
NavigationStack { RootView() }

NavigationLink(SomeDestination()) { Text("Go") }
```

`Navigator` drives it imperatively, from anywhere — no binding, no path
type, no destination closure:

```swift
Navigator.push(_ view: any View) -> UInt64
Navigator.pop()
Navigator.pop(to id: UInt64)
Navigator.popToRoot()
Navigator.setPath(_ views: [any View])

Navigator.present(_ view: any View,
                  as style: NavPresentation = .sheet,
                  morphingFrom morphSource: String? = nil) -> UInt64
Navigator.dismiss()
Navigator.dismiss<Value: Sendable>(returning value: Value)

Navigator.stack(_ tab: Int) -> NavigationStackModel
Navigator.select(_ tab: Int, then: ((NavigationStackModel) -> Void)? = nil)
```

`NavPresentation` is `.push` / `.sheet` / `.cover`. **Differs:** all
three are one call with one enum, and the stack that owns the push owns
the modal.

Titles and chrome:

```swift
.navigationTitle(_ title: String)
.navigationBarTitleDisplayMode(.large)   // .automatic | .large | .inline
```

**The morphing sheet.** Mark a source and the sheet grows out of it —
no other API:

```swift
Button("Compose") { Navigator.present(Composer(), morphingFrom: "compose") }
    .matchedTransitionSource(id: "compose")
```

Presenting with no `morphingFrom`, or from a source that isn't on screen,
falls back to the ordinary rise-from-the-bottom sheet.

---

## Tabs

```swift
TabView {
    Tab("Home", Icon.house) { HomeView() }
    Tab("Search", Icon.magnifyingGlass, role: .search) { SearchView() }
    Tab("Profile", Icon.user, onReselect: .popToRoot) { ProfileView() }
}
```

```swift
Tab(_ title: String, _ icon: Icon,
    role: TabRole? = nil,
    onReselect: TabReselect? = nil,
    @ViewBuilder content: @escaping () -> Content)

tab.badge(_ count: Int)
```

`TabReselect` is `.popToRoot`, `.scrollToTop`, or `.run(() -> Void)`.

`TabView`'s own configuration, all defaulted:

```swift
TabView(
    background: Material? = …,
    navigationBackground: Material? = …,
    transition: NavTransition = .standard,
    screenBackground: Color = .background,
    interactiveBack: Bool = true
) { … }
```

**Differs:** each tab is wrapped in its own `NavigationStack`
automatically. Don't add one.

---

## Toolbars

```swift
.toolbar {
    ToolbarItem(placement: .topBarTrailing) {
        Button("Done") { done() }
    }
}
```

`ToolbarItemPlacement`: `.automatic`, `.topBarLeading`,
`.topBarTrailing`, `.bottomBar` (see the enum for the full set).

---

## Animation and transitions

```swift
withAnimation(_ animation: Animation = .default) { … }
withoutAnimation { … }

.animation(_ animation: Animation = .default)
.animation(_ animation: Animation = .default, id: UInt32)
.animation(_ animation: Animation = .default, value: someEquatable)
```

Curves:

```swift
Animation.default                  // .easeInOut()
Animation.linear(duration: Double = 0.3)
Animation.easeIn(duration:) / .easeOut(duration:) / .easeInOut(duration:)
Animation.spring(response: Double = 0.4, dampingFraction: Double = 0.8)
```

Transitions, for insertion and removal inside a `ForEach`:

```swift
.transition(_ transition: Transition)
.contentTransition(_ transition: Transition, animation: Animation? = nil)

Transition.identity
Transition.opacity
Transition.slide
Transition.scale(_ factor: Float = 0.92)
Transition.offset(x: Float = 0, y: Float = 0)
Transition.blurReplace / .blurReplace(radius:)
Transition.sheet / .cover
Transition.morph(from: SFRect, to: SFRect)

a.combined(with: b)
Transition.asymmetric(insertion:removal:)
```

**Differs:** animation is keyed by call site (`#fileID`/`#line`/
`#column`). If a modifier moves between builds and you need the animation
to follow, use the `id:` form.

---

## State

```swift
@State private var count = 0            // value state, survives the rebuild
@Binding var text: String               // two-way handle

Binding(get: { … }, set: { … })
```

Reference state uses the observation runtime:

```swift
@Observable
final class Store {
    var items: [Item] = []
}

struct ContentView: View {
    @Observed var store = Store()       // ← not @State
    …
}
```

`@Observable` is a macro; the underlying protocol is
`protocol Observable: AnyObject`. Any `Observable` can hand out a binding
to one of its own properties:

```swift
store.binding(to: \.query)              // -> Binding<String>
```

**Differs:** the whole tree is rebuilt every frame, so state lives in
registries that outlive the rebuild rather than in the view values.
Nothing needs `@StateObject` / `@ObservedObject` / `@EnvironmentObject`;
there is one spelling.

---

## Environment

```swift
struct ThemeKey: EnvironmentKey {
    static let defaultValue = Theme.dark
}

extension EnvironmentValues {
    var theme: Theme {
        get { self[ThemeKey.self] }
        set { self[ThemeKey.self] = newValue }
    }
}

// write
ContentView().environment(\.theme, .light)

// read
@Environment(\.theme) private var theme
```

`ColorScheme` (`.light` / `.dark`) is provided.

---

## Types

### Color

```swift
Color(r:g:b:a:)  Color(hex: 0xRRGGBB, alpha: Float = 1)
color.opacity(_ value: Float)
```

One fixed dark theme, tuned rather than sRGB primaries. Two groups, and
the split is the useful part — **roles** say what a thing *is* (reach for
these), **named** colours say what it *looks like*.

Roles: `background`, `surface`, `overlay`, `fill`, `primary`,
`secondary`, `tertiary`, `placeholder`, `separator`, `border`, `accent`,
`scrim`, `destructive`, `success`, `warning`, `clear`, `black`, `white`.

Named: `red`, `orange`, `yellow`, `green`, `mint`, `teal`, `cyan`,
`blue`, `indigo`, `purple`, `pink`, `brown`, `gray`.

`yellow` will not carry white text at any lightness where it still reads
as yellow — put `.background` on it instead.

### Font

```swift
Font(size: Float, weight: Weight = .regular, design: Design = .default)
Font.system(size:weight:design:)
```

Presets: `largeTitle` 34, `title` 28, `title2` 22, `title3` 20,
`headline` 17 semibold, `body` 17, `callout` 16, `subheadline` 15,
`footnote` 13, `caption` 12, `caption2` 11.

`Font.Weight` maps 1:1 onto the variable-font `wght` axis
(`ultraLight` 100 … `black` 900).

`Font.Design`: `.default` (Inter) and `.monospaced` (JetBrains Mono) are
bundled. `.serif` and `.rounded` fall back to `.default`.

### Icon and Image

`Icon` is a single glyph from the bundled Phosphor faces, sized to its em
box rather than its ink — so a row of icons shares one size and one
baseline. The catalogue is generated; reach for `Icon.<name>`.

`Icon.Weight` picks the face (thin … fill), not a variable axis.

`Image` sources: `Image("assetName")` from the flattened catalogue,
`Image.system(_:)`, or `AsyncImage(url:)`. `ContentMode` is `.fit`,
`.fill`, `.stretch`.

`AsyncImagePhase` is `.empty` / `.success(Image)` / `.failure`.

**Differs:** without `.resizable()` an image is intrinsic — it reserves
its asset's own size. That matches SwiftUI.

### Alignment, Edge, EdgeInsets

```swift
HorizontalAlignment  .leading .center .trailing
VerticalAlignment    .top .center .bottom
Alignment            .topLeading .top .topTrailing
                     .leading .center .trailing
                     .bottomLeading .bottom .bottomTrailing

Edge                 .top .bottom .leading .trailing
                     .horizontal .vertical .all      // OptionSet

EdgeInsets(top:bottom:leading:trailing:)
EdgeInsets(_ all: Float)
EdgeInsets.zero

Axis                 .vertical .horizontal
Point                Point(x:y:)
```

---

## Device and screen

```swift
Screen.width / Screen.height / Screen.isKnown     // points
SafeArea.top / .bottom / .leading / .trailing
DeviceScale.current                               // points -> pixels
DeviceMetrics
```

`NodeFrames` reads back where a node actually landed after layout —
what `matchedTransitionSource` and `List` are built on:

```swift
NodeFrames.shared.frame(for id: UInt32) -> SFRect?
NodeFrames.id(for name: String) -> UInt32
```

A frame is one frame old, and a missing id reports nil rather than
zero-rect nonsense.

---

## Lifecycle

```swift
protocol LifecycleProvider

view.onForeground { … }
view.onBackground { … }
view.onTerminate  { … }
view.onOpenURL    { (url: URL) in … }
```

---

## Logging

```swift
Log.print(_ items: Any...)
```

Routes to the platform's own log — `__android_log_write` on Android,
stderr elsewhere — so it survives on a device where `print` does not.

---

## Things SwiftUI has that this does not

Worth knowing before you reach for them:

- No `GeometryReader`. `ScrollGeometry` covers the scrolling case;
  `NodeFrames` covers reading a frame back after layout.
- No `@StateObject` / `@ObservedObject` / `@EnvironmentObject` — one
  spelling, `@Observed`.
- No `PreferenceKey`, no `alignmentGuide`, no `Layout` protocol.
- No `LazyVStack` / `LazyHStack` — `List` is the windowed container.
- No `NavigationSplitView`, no multi-column.
- No `.animation(_:)` without a value on a leaf that changes identity —
  see the note on call-site keying.
- Real clipping. `.clipShape()` is exact for coextensive boxes only.
