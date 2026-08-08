# Architecture notes

Decisions and invariants that aren't obvious from reading any single file.
If you're changing layout, text rendering, state, or concurrency, read this
first — several of these exist because getting them wrong was a real bug.

## Core invariants

- **Physical pixels everywhere.** Layout, glyph rasterization, and node
  frames are all in physical pixels. `DeviceScale.current` is applied at
  the Swift boundary; NDC conversion happens in the vertex shader — not
  before.
- **Node identity** is a structural path hash (type + path) via FNV-1a,
  computed through `BuildContext`.
- **`@State` storage keys** use `#fileID:#line:#column` — stable across
  rebuilds without requiring explicit identity.
- **Concurrency: nothing on the build path is `@MainActor`.** The tree is
  rebuilt synchronously, start to finish, on whichever single thread the
  host drives — a CADisplayLink on iOS, a winit callback on desktop, a
  Choreographer callback on Android — and nothing else touches it while
  that runs. That, not an actor, is what makes it safe.
  `NodeRegistry`, `FrameArena`, `BuildContext` and `GestureRouter` are
  `nonisolated(unsafe)` for the same reason.
  Android forced the issue: `android_main` runs on a thread
  `android-activity` spawns, so Swift's main executor is bound elsewhere
  and every dynamic dispatch into isolated code tripped
  `swift_task_checkIsolated` → `dispatch_assert_queue` → SIGTRAP. No
  annotation can fix that; only dropping a claim that was never true.
- **Clipping is real, and opt-in.** `SFClip { rect, radii: [f32; 4] }`
  carries per-corner radii; `DrawList` owns the stack and stamps the
  current clip onto each `DrawItem` as it is emitted, so batching can
  reorder freely without a command losing its clip. A node opts in with
  `clip_content` — making it unconditional would eat shadows, offsets and
  the navigation layers that deliberately draw outside their parent.

## State

Four mechanisms, and which to reach for is decided by *who owns the
value* rather than by taste.

- **`@State`** — one value owned by one view. Stored in `NodeRegistry`
  keyed by `#fileID:#line:#column`, so it survives the per-frame rebuild
  without the view needing an explicit identity. Writing it calls
  `markDirty`, which is what schedules a frame.
- **`Binding`** — a read/write window onto someone else's value.
- **`Observable` / `@Observed`** — state that lives outside the tree and
  several views read. See below.
- **`@Environment`** — a value passed implicitly down a subtree.

### Observation

Every host skips the rebuild when nothing is dirty:

```swift
guard NodeRegistry.shared.needsRender || anyScrollActive || anyAnimationActive
```

`@State` and taps set that flag. A write to a store outside the tree —
`Workspace.shared`, a document model, a cache — set nothing, so the model
changed and the screen didn't. That is invisible while every mutation
happens inside a button action, and appears the moment one doesn't: a
file finishes loading, a timer fires, an IME commits.

**This is much smaller than SwiftUI's, on purpose.** SwiftUI's
`@Observable` answers "*which views* read this property", so it needs a
keypath-granular registrar, `access(keyPath:)` on every read, and a
`withObservationTracking` scope at the observer. SwiftFlow rebuilds the
whole tree every frame, so the only question worth answering is the one
the hosts already ask — rebuild, or skip. A write sets one flag and there
is nothing to track. `NodeBuilder` needs no observation scope at all.

The consequence, stated because it is a real difference: the signal is
**coarse**. Any write to any observable wakes the loop, not only one a
view read. Finer granularity would only pay off if parts of the tree were
skipped, and nothing here works that way. The macro emits an `access()`
call anyway — a no-op today — so granularity stays addable without
touching a single call site.

**Not Apple's `@Observable`**, which requires iOS 17 / macOS 14 and the
`Observation` framework. That floor is the reason ours exists: a
framework wanting to lower its minimum cannot adopt one that sets a high
one, and the Android Swift SDK may not ship Observation at all. Ours is
plain Swift with no availability constraint — `init` accessors and macros
are compile-time features, so the generated code runs wherever Swift
does.

The macro lives in its own package (`macros/`) because a plugin needs
swift-syntax: a multi-minute compile, and a *host* executable SwiftPM
must build while cross-compiling. `SwiftFlowCore` must not carry that, so
an app opts in the way it picks a host package. `@Observed` is the
macro-free spelling and behaves identically — which is also the fallback
for a platform where the plugin can't be built.

### Environment

A process-global with a save/restore stack, which would be indefensible
in most frameworks and is correct here for exactly the reason
`BuildContext` is: the tree is built synchronously, depth-first, on one
thread, and nothing else touches it while that runs (Core invariants,
above). So "the current environment" is unambiguous at every point in the
build.

`.environment(_:_:)` **passes the child's node through untouched**,
following `WeightModifier`/`ExpandModifier` and *not*
`FrameModifier`/`PaddingModifier`, which wrap content in a single-child
ZStack. A wrapper node would be invisible in the API and visible in the
layout, and would break `.weight()` on everything below it — weight being
ignored inside a hugging container.

`@Environment` reads at **access**, never at `init`. A property wrapper
is initialised when the view *value* is constructed, which is not when
`body` runs; capturing in `init` reads from the wrong point in the tree,
and looks right in a flat test. `@State` avoids the same trap the same
way.

## View system

- View dispatch resolves protocol witnesses through existentials using a
  generic `open<V: View>(_ v: V)` trick; `Never` conforms to `View`.
- `TupleView` is matched via `TupleViewProtocol.buildNodes()` rather than
  existential pattern matching.
- `PaddingModifier` and `FrameModifier` wrap content in a single-child
  `SF_AXIS_DEPTH` ZStack — neither is a field set on the content node.
  Collapsing them back onto it is what made padding inflate leaf shapes
  and made a second `.frame()` erase the first; see Sizing and padding.
- ZStack hug sizing is two-pass: lay out non-fill children first to derive
  content size, then constrain fill children to it.
- `Button` uses a pluggable `ButtonStyle` protocol. `background` /
  `overlay` / `padding` modifiers exist.

## Text pipeline

The pipeline went bitmap → SDF-from-bitmap → beziers. Current state:

- Quadratic beziers extracted from the font via `ttf-parser`.
- CPU rasterizer using winding-number ray casting with R2 quasirandom
  jitter sampling.
- Atlas format is `Bgra8Unorm` with a B/R channel swap.
- Text blur is a real 7x7 binomial convolution over the glyph's atlas
  region, with the quad grown by the blur radius. `glyph_alpha` returns 0
  outside the glyph rather than clamping, so a blurred letter doesn't
  smear its neighbours into itself.

Six fixes were what made it production-quality — **do not regress
these**:

1. Rasterize at `font_size * scale` (physical pixel size).
2. Render glyph quads at 1:1 texel ratio — no scale division in `text.rs`.
3. Scale jitter to em-space pixel size, not raw ±0.5 screen units.
4. Baseline: `y = baseline_y - render_h - offset_y`, all in physical
   pixels. A text run puts `baseline_y` one cap height (`glyph('H')`)
   below the top of the frame, so baselines stay on a fixed grid across
   lines instead of shifting with whatever each line contains —
   `FontSystem::measure` depends on that and says so. An icon run uses
   its own face's ascender instead; see below.
5. **Quadratic roots use Citardauq's form, not `(-b ± √d) / 2a`.** This
   is what actually made bold `y`, `v`, `w`, `x` and `k` render with a
   hard staircase down one edge. Those diagonals are straight lines the
   font stores as *quadratics* with the control point at the midpoint,
   which makes the `t²` coefficient `a` zero — exactly zero at a static
   weight, but a variable-font instance interpolates the control point
   and leaves `a` at float noise. The real segment measured a = -6.1e-5
   against b² = 3.4e5, so an `a.abs() < 1e-6` linearity test missed it by
   a factor of sixty and the quadratic branch ran with an `a` a billion
   times smaller than `b`. `√(b² - 4ac)` rounds to exactly `|b|`, one
   root's numerator cancels to nothing, and the answer is whatever the
   rounding says — it came out a constant 0.5 for twenty scanlines and
   then a constant 1.0, pinning the edge to the control point and then to
   the endpoint. Hence bold only, and diagonals only: a straight line
   built by `line_to` has a binary-exact midpoint, so `a` really is zero
   and the stable path was never needed. Citardauq shares one square root
   between the roots, puts the subtraction where nothing cancels, and
   degrades to the linear root for free as `a` vanishes.
   `tests/near_straight_quads.rs` carries the real segment's numbers and
   fails against the naive formula.
6. **The sample pattern is rotated per pixel** (`pixel_rotation`,
   Cranley-Patterson). Not the staircase fix — that is item 5 — but a
   real one: without it every pixel sampled at identical sub-pixel
   positions, so coverage error was systematic rather than noise (lag-1
   autocorrelation 0.74 at 32 samples). Mean edge error at 32 samples
   improved 0.040 → 0.029. `tests/glyph_antialiasing.rs` guards it, and
   asserts accuracy alongside coherence since a badly distributed pattern
   would also score low coherence.

Temporal accumulation is implemented: `GlyphAccumulator` stores f32
coverage and `refine()` adds 8 samples per pass. Two limits keep it off
the critical path, and both were the startup lag before they existed —
`REFINE_TARGET_SAMPLES` (128) is where a glyph stops refining, and
`REFINE_BUDGET_PER_FRAME` (8) is how many glyphs may refine on one frame,
served round-robin. The atlas uploads only its dirty rows
(`FontSystem::atlas_dirty_rows`), not all 16 MB.

`swiftflow_get_atlas_debug` exports the CPU-side atlas over FFI. It has no
callers: the `CGImage` viewer it was written for was Apple-only and no
longer exists.

### Icons

Phosphor Icons ride the same pipeline as glyphs, because an icon set has
the properties it already solves: monochrome, vector, and only the ones
on screen should cost anything.

**Routing is by family, and this section used to say the opposite.** It
claimed the codepoint was enough — Phosphor occupies U+E000..U+EE82,
entirely inside the Private Use Area, "and Inter maps nothing there".

Inter maps 652 codepoints there. Phosphor maps 1512. They overlap on 306.
Codepoint routing resolved every one of those 306 to the icon, which is
what an `Icon` wants and is wrong for a run of text, so the ambiguity
never surfaced as a bug — it was latent, not absent, and it became real
the moment a second text face was bundled (JetBrains Mono carries
Powerline symbols at U+E0A0).

So `SFNode` carries an `SFFontFamily`, the atlas key is
`(char, size, weight, family)`, and `Icon` is one of the families. The
draw and layout passes force it for `SF_NODE_ICON` rather than trusting
the field, so an icon cannot draw from a text face by omission.
`is_icon` survives for the one thing it is genuinely good for: telling
the draw pass that a run is a single icon glyph, so its baseline sits on
the em box rather than a cap height.

`tests/font_family_routing.rs` pins all of it, including the shared
codepoint resolving two different ways.

A second icon font is now possible — it becomes another family, rather
than another set of codepoints nothing could tell apart.

Weight selects one of five static faces rather than moving an axis,
bucketed onto SwiftUI's 100-900 scale (thin/light/regular/bold/fill), so
`.fontWeight(.bold)` works and an outline/filled tab pair falls out of
the weight alone. Each face is ~0.5 MB and individually trimmable via
cargo features; with all of them compiled out `is_icon` goes quiet rather
than handing the loader a codepoint it has no data for. Duotone is
absent: it is two glyphs in two colours, and every draw path here carries
one colour per glyph.

`SF_NODE_ICON` exists for the **sizing rule**, not the drawing — it emits
an ordinary text draw command. Icons are drawn to fill an em box wider
than their ink, and by different amounts each (a house is 56% of the em
wide, a gear 82%), so hugging the ink the way `Text` does would give
every icon in a row a different size and baseline. The node reserves the
em square; the draw pass centres the ink by putting the baseline on the
icon face's own ascender. `tests/icon_geometry.rs` pins both halves.

The catalogue is generated: `tools/fetch-phosphor.sh` vendors the faces
and a name→codepoint table, `tools/generate-icons.py` writes
`PhosphorIcons.swift`. Both inputs are committed, so the build works
offline and a font cannot renumber every icon underneath a release.

## Fonts

Three families are compiled in: Inter (sans), JetBrains Mono
(monospaced) and the five Phosphor faces (icons). Serif and Rounded exist
in the API, have no face, and resolve to sans — `SFFontFamily::is_bundled`
is how an app tells the difference rather than by comparing pixels.

`SFNode.font_family` carries the choice, the glyph atlas is keyed by
`(char, size, weight, family)`, and `Font.Design` is what writes it from
Swift. That enum had existed and been stored on `Font` since the
beginning without ever reaching `SFNode` — `.monospaced` compiled, and
rendered as Inter.

Both text faces are variable on the `wght` axis, so one code path
instances either; ttf-parser clamps out-of-range values, which is why
asking Mono (100-800) for 900 gives its heaviest rather than failing.
The three disagree on units per em — 2048, 1000 and 1024 — so `GlyphData`
reports the value from the face actually parsed. Caching one and stamping
it on every glyph is a 2x scaling bug, and was one.

The monospaced face earns its place beyond looking right: every ASCII
glyph advances exactly 0.6em, which is what lets a text grid be computed
rather than measured. Nothing in the framework can measure text during a
build — layout runs in Rust after the tree is handed over — so an editor
placing a caret from a click has no other route.

## Liquid Glass

`.glassEffect()` is opt-in and additive. `Material`, `.specular()` and
the chrome that uses them (`NavStack`, `TabView`, `Presentation`,
`ButtonStyle`) are untouched — a glass nav bar is a decision an app
makes, not one a framework upgrade makes for it.

**The bevel is the whole difference.** A `Material` blurs what is behind
a shape and tints it: that is frosted plastic, because the image behind
is softened but never *moved*. Glass is a lens, and the lens is its
*shoulder* — the rounded edge where the surface turns from flat to
vertical over the last few pixels before the border.

`fs_material` models that literally. `bevel_normal` gives the surface a
quarter-round profile whose height and width are both the bevel width, so
its slope is dimensionless; the view ray is then put through WGSL's
`refract` at `GLASS_IOR`, and the backdrop is sampled along the refracted
direction. The offset is normalized by `sqrt(1 - eta²)` — the exact
lateral component at grazing incidence — so a material's `refraction`
keeps meaning what it always meant, pixels of bend at the rim, and the
`Glass` presets did not change strength when the model did.

This replaced an `edge²` weighting on the SDF normal. That version peaked
at the border but ran smoothly *through* it, so it could slide the
backdrop and never squeeze it — and the compression of the last few pixels
before the edge is the most recognisable thing about iOS glass.

Three things ride on the same bevel normal: **Fresnel**, an even bright
ring the whole way round, brightest where the surface is steepest;
**glare**, a directional lobe on top of it with a weaker one opposite;
and an **inner shadow** on the far side, which is what makes the pane read
as thick rather than printed on. **Dispersion** refracts red and blue at
`GLASS_IOR ± GLASS_DISPERSION` rather than scaling the green offset, so
the prism split follows the bevel's own falloff and is a rim effect
without needing to be masked into one. **Vibrancy** pushes the backdrop's
saturation up before the tint goes on, because glass concentrates colour
where a white tint alone only washes it out.

Most of the rest was already here for other reasons: the progressive
feather, shadows, squircle corners, and the metaball merge (`fs_merged`,
IQ smooth-min) that fuses neighbouring shapes the way
`GlassEffectContainer` does.

Two implementation facts worth knowing:

- **The backdrop comes off the halving chain, not a snapshot.** Refraction
  needs detail to bend, and it cannot sample `scene_texture` directly —
  the composite draws *into* that, and sampling a render target you are
  writing is a hazard wgpu rejects. It samples `pyr_half` instead, the
  first level of the reduction that feeds the blur, built inside the
  material's own run and therefore always the backdrop as it stands.
  Half-resolution rather than full, and that is not a compromise: the
  displacement varies per pixel, so it compresses the backdrop near the
  rim, and a full-resolution fetch under a compressing map is undersampled
  by definition. A true box average cannot be. This also makes glass over
  glass refract the composited result, since each material rebuilds the
  chain after the one before it.
- **Every glass effect is gated on `refraction > 0`.** The sharp-rim mix,
  the adaptive tint, the vibrancy, and all three lighting terms would
  otherwise change how every already-shipped `Material` renders. Notably
  the *old* fixed-axis specular trim survives on that path unchanged, so
  `.specular()` on a plain nav bar is still the thin two-corner highlight
  it was rather than a Fresnel ring.
  `tests/glass_optics.rs::a_material_without_refraction_is_untouched`
  pins all of it, sweeping distances, radii and normals.

Two spare floats in `MaterialGpu.params2` carried the strength and the
press response, so none of this needed a new buffer or binding — only
`SFNode` grew, by `glass_refraction` and `glass_interactive` (288 → 296).
The bevel, index of refraction, dispersion, glare and shadow are all
shader constants: `Glass` exposes blur, tint, refraction and
`.interactive()`, which is the surface Apple exposes, and nothing new
crosses the FFI.

**Not there, and honest about it:** the highlight does not track device
motion. Apple's shifts as the device tilts, and no host here has
attitude plumbing — CoreMotion on iOS, the rotation-vector sensor on
Android, nothing on desktop. The fixed `(-1, -1)` light axis stands in.
Content vibrancy — adaptive contrast for text drawn *on* glass — is also
absent, and is a text-pipeline change rather than a material one.

## Input

`sf_hit_test` walks the node tree in Rust doing recursive frame
containment. Single-pass recursive hit testing, so gesture composition
doesn't hit UIKit's competing-recognizer problems. Touch events enter via
`MetalView` on iOS and `DesktopHost` elsewhere; both forward to
`GestureRouter`, which is where all dispatch lives.

**Dispatch uses `sf_hit_test_path`, not `sf_hit_test`.** "What is under
the finger" is the wrong question: a handler is registered on a node that
usually has drawn content inside it, and that content is deeper. The
chain lets the router find the deepest *ancestor with a handler*. (Taps
worked before this only because `BackgroundShapeModifier` builds its two
children by calling `toSFNode()` directly, leaving their ids at zero for
the walk to skip — a coincidence, not a design.)

**A view can claim its own `node_id`.** `Button` and `GestureModifier`
need to know, while building, the id their handler will be found under.
`NodeBuilder.buildAny` therefore keeps an id a node already carries
rather than overwriting it with the structural one — which is derived
from the *outermost* view's type name, so `Button(…).padding(8)` used to
strand the action on an unreachable id. An explicit identity (a `ForEach`
row, `.id(_:)`) still outranks both, and `buildAny` re-keys the
registries when it does.

**Gesture state is readable during the build**, the same move
`ScrollGeometry` makes, which is why there is no `@GestureState`
equivalent. Declarations are cleared every build (a view that stops
declaring a gesture stops receiving one); state is keyed by the
gesture's own call site and persists (a drag in flight outlives the tree
it started on).

**Arbitration is a stated rule, not a priority system**, and one rule
covers both conflicts a press can have. Once it has moved far enough to
have a direction: a drag gesture claiming that direction takes it;
otherwise the innermost scroll view under it whose axis matches *and
which can still move that way* takes it; otherwise nobody scrolls. So
`DragGesture(axis: .horizontal)` on a row inside a vertical list gets
sideways swipes, a horizontal strip inside a vertical list gets sideways
drags, and a scroll view that has hit its edge hands the press outward
instead of trapping it. Replaces SwiftUI's `.simultaneousGesture` /
`.highPriorityGesture` folklore with something decidable from the
movement.

The decision is **deferred** — at touch-down the direction isn't known,
so nothing is claimed. Both hosts used to call `beginDrag` on the
innermost scroll view the instant a finger landed, which is exactly what
made the innermost one win unconditionally. All scroll tracking now lives
in `GestureRouter`; the hosts forward four pointer events and nothing
else.

Single touch only, on both hosts — so magnify and rotate have no input to
be built from yet.

## Navigation

`Navigator` is process state — a tab selection plus one stack per tab —
sitting beside `NodeRegistry` for the same reason: it is state the frame
loop reads. Bare verbs act on the selected tab's stack;
`Navigator.stack(1)` targets a specific one; first access mints an empty
stack, so there is no registration.

Four things are structural rather than cosmetic:

- **A destination is a view.** `Navigator.push(ProfileView(id: 12))`.
  There is no route type, no destination table and no generic parameter,
  so nothing about navigation can fail to infer. Storing a view is
  correct *here specifically* because a view is a description and the
  tree is rebuilt every frame — `toSFNode()` runs afresh on a pushed
  screen, and its `@State` lives in NodeRegistry keyed by source
  location, so nothing is frozen at the moment of the push.
  **The path must therefore never hold `AnyView`**, which builds its
  node eagerly in `init` and would render the frame it was pushed on
  forever.
- **The cost is deep-linking by path.** A path of views is not `Codable`,
  so `snapshot()`/`restore()` are gone rather than reimplemented.
  Restoring to a screen means pushing that screen at launch.
- **Push, sheet and cover are one call with a parameter.** `pop()` drops
  whatever is actually in front of the user — modal first, then pushed
  screen — which in SwiftUI is split across `dismiss()` and a path
  mutation, and which of those a view needs depends on how it was
  presented.
- **No observation machinery.** SwiftUI needs `@Observable` and a
  dependency graph to answer "what changed"; here the tree is rebuilt
  every frame, so a mutation only says that something did
  (`needsRender = true`).

**A tab is a NavStack root.** `TabView` wraps each tab's content in one,
so `.navigationTitle` works on tab content directly and a push from
inside a tab lands on that tab's own back history. Tab identity is
position — `Navigator.tabSelection` is an `Int` — which is what removed
the last generic parameter from the navigation types.

**Layers go through `ForEach`.** A push, a pop, a present and a dismiss
all render as a one-element ForEach keyed by depth (or modal index),
because ForEach is the only place in this framework that can tell a
removal from a rebuild. That is what makes them animate rather than cut,
and it is why only the top layer is ever built.

Three ambient build-time stores now use the same shape —
`NavigationConfigStore` (toolbar/title), `NavContext` (which stack a
NavLink pushes to), `TabBuildContext` (which scroll view a tab's
`.scrollToTop` aims at). All rely on `toSFNode()` recursion being
strictly depth-first, and all save/restore rather than set/clear so
nesting works.

One gap is the framework's, not the design's: `role: .search` places a
tab at the trailing end but cannot style it as a capsule or make it a
search field yet.

## Sizing and padding

**`.padding()` and `.frame()` build a box; every other modifier writes a
field.** That split is the whole model, and it is what makes ordering
partly — but only partly — meaningful.

A box modifier emits a single-child `SF_AXIS_DEPTH` stack: padding sets
the stack's `padding`, frame sets its `fixedWidth`/`fixedHeight` or
`sizing`, and the content goes inside. So `.padding(20).frame(width: 200)`
is 200 wide overall, `.frame(width: 200).padding(20)` is 240, and
`.frame(100).frame(200)` nests instead of the second write clobbering the
first. `.background()` and `.overlay()` have always built real nodes too,
which is why padding↔background ordering worked even before this.

Two bugs went away with the rewrite, both from padding being a field on
the content node:

- **Padding inflated a leaf.** A Rect draws at its frame and has no
  children to inset, so `Circle().frame(width: 44).padding(10)` grew
  `fixedWidth` to 64 and drew a *64pt circle*. The gap became girth.
- **Repeated `.frame()` overwrote itself**, since both writes landed on
  the same `fixedWidth`.

Sizing of a box: Hug (content plus padding) unless the content is Fill,
in which case Fill is inherited — `.frame(maxWidth: .infinity).padding(20)`
must keep filling, and a Hug ZStack sizes itself from its *non*-Fill
children, of which there would be none.

### Sizing is per axis

`SFNode` carries `sizingX` and `sizingY`, and every box modifier answers
them separately: was a size named on this axis, was `.infinity` named on
it, or was nothing named at all — in which case the axis inherits Fill
from the content and hugs it otherwise. `SFNode.sizing` in Swift is a
convenience that reads the horizontal one and writes both, for the many
nodes that genuinely mean the same thing on each (a `Spacer` fills in
every direction; a `Text` hugs in every direction).

It was one flag until it had cost four separate bugs, each patched
locally before the shape of them was clear:

- **A list row was the height of the viewport.** `.frame(maxWidth:
  .infinity)` meant Fill, and Fill meant both axes.
- **A swipe action's button dragged its row off the screen.**
  `.frame(width: 74)` meant Fixed, Fixed meant both axes, and
  `fixedHeight` was 0 — which the resolver read as "no height was asked
  for" and answered with the whole proposal.
- **`.frame(width: 100, maxHeight: .infinity)` was order-dependent**,
  because both writes landed on the same field.
- **Every windowed list opened scrolled past a screenful of nothing.**
  Its leading pad is a `.frame(height: 0)`, indistinguishable from no
  frame under the same `fixed > 0` test. Measured: a two-row list
  reported 1688 points of content instead of 88.

`SF_SIZING_FIXED` now means the declared size **including zero**, and the
`fixed > 0` sentinel is gone from both `resolve_frame` and
`natural_extent`. The ABI did not grow: the second enum landed in
padding the struct already carried, so `SFNode` is still 296 bytes.

**Everything else still collapses onto one node**, so their relative order
is not recoverable: `.cornerRadius`, `.clipShape`, `.scale`, `.offset`,
`.opacity`, `.blur`, `.shadow`, `.weight`. They are render-time
properties rather than boxes, so the common orderings agree anyway — but
`.padding(10).cornerRadius(4)` and `.cornerRadius(4).padding(10)` are the
same tree, where SwiftUI's differ.

Two consequences of boxes being real nodes, both deliberate:

- **A gesture lands where the order says.** `.onTap {}.padding()` keeps
  the padding ring tappable (the handler is on the outer box);
  `.padding().onTap {}` does not. That is SwiftUI's behaviour and was not
  the old behaviour, where padding was always inside the handler's node.
- **Scroll content is measured recursively** (`natural_extent`). The
  recovery of a scroll's true content size used to read the scroll's
  *direct* child and require a linear stack, which held only while
  modifiers collapsed. A wrapper now sits in between routinely.

### Weight

`SFSizing::Fill` means **all of it**, not **a share of it**, and no amount
of per-axis precision changes that. A Fill child is laid out against the
container's whole available size; with several of them in one stack the
first covers it and the rest are positioned off its end, which is what a
tab bar looked like the moment it had more than one tab.

(The original reason given here was that Fill could not tell
`.frame(maxWidth: .infinity)` from `.frame(maxHeight: .infinity)`, which
was true of the single-flag `sizing` and is not true of `sizingX`/
`sizingY`. The greediness is the durable half of the argument and is on
its own sufficient.)

`.weight(_:)` is the separate, unambiguous field for that. Rules, all
pinned by `tests/weight_distribution.rs`:

- Inflexible children are measured first, so weights divide only what is
  genuinely left over (spacing included).
- A weighted child is laid out **against its share**, not against the
  stack — a nested Fill has to resolve to the column, not the container —
  and its main-axis frame is then forced to that share, so a small child
  still fills its column and stays hit-testable across it.
- **`Spacer` is weight 1 in the same pool.** Two pools would hand the
  leftover out twice. With no weights anywhere the arithmetic reduces to
  `remaining / spacer_count`, which is exactly what spacers did before.
- **Ignored in a Hug container**, where "a share of the leftover" is
  meaningless — honouring it would let a weighted child inflate the stack
  to whatever its parent happened to offer.

That last rule has a sharp edge, which is why `.expands()` exists.
`VStack`/`HStack` are Hug by default, and `.frame(maxHeight: .infinity)`
**wraps** a stack in a filling box rather than making the stack itself
fill — so the obvious spelling of "a column that fills the window, with
one pane taking the slack" silently drops every weight inside it and
collapses to the height of its content. Nothing reports it; the code
looks right. `.expands()` sets `SFSizing::Fill` on the container's *own*
node, in place, the same way `.weight()` sets its field — no wrapper, so
there is no Hug stack left in between. The two compose on one node:
Fill so the pane can host weighted children, weight so it takes a share
of its parent rather than all of it. Both readings are pinned by
`tests/codeflow_shell_layout.rs`, including the collapse.

`NavigationStack`'s bar row depends on the second reading (its
`.padding(.top, SafeArea.top)` precedes a `.frame(height:)` that already
counts the safe area). App code writing `.frame(...).padding(...)` gets
the first.

A related trap, since it cost a screen once: a **Hug stack must report
the gaps between its children**, not just their sum. Nothing inside the
stack looks wrong when it doesn't — the damage lands on its siblings,
which the parent positions against the too-short frame and therefore
draws on top of the stack's last rows.

## Layout engine sequencing

Layout caching, per-node dirty flags, and incremental re-solve are part
of the layout engine's **definition of done**, not a later phase.
Retrofitting dirty-tracking onto a graph that assumed global recompute is
expensive. The FNV-1a path IDs are the natural key for it.

Reactive dependency tracking (auto-invalidation from store reads) can
stay late in the sequence — it fires on discrete events, not per scroll
frame.

## Platform glue

- **iOS:** working wgpu/Metal renderer. Dev workflow uses
  `pymobiledevice3` to launch and `idevicesyslog` for logs; `setbuf(stdout,
  nil)` plus a pipe redirect makes `print()` appear in syslog.
- **Desktop (macOS/Linux/Windows):** `rust/swiftflow_desktop`, a winit
  host. The direction inverts relative to iOS — winit's event loop takes
  the thread, so Swift still owns `main()` but hands control to
  `sf_desktop_run`, which calls back through `SFDesktopCallbacks`.
- **Android:** `rust/swiftflow_android` plus the `android/` host package,
  built by `swiftflow build --platform android` — which also *generates*
  the Gradle project into the app's `.build/android`, since every value
  in it that was ever the app's own now comes from `SwiftFlow.toml` (see
  cli/src/gradle.rs, and "Project configuration"). An app that puts a
  real `android/` beside its Package.swift keeps that one instead. Also a winit host, reusing the desktop one's
  contract, so the earlier plan here — a hand-written JNI event bridge —
  turned out to be unnecessary. What it actually is:
    - winit 0.30's `android-activity` backend supplies the event loop,
      touches and the surface. `swiftflow_wgpu` needed no changes at all:
      the Android `Window` converts to a `SurfaceTarget<'static>` and goes
      through `init_with_target`, the same call the desktop host makes.
      `SFSurfaceKind::RawHandle` is still unimplemented and still unused.
    - The boot inverts one step further than desktop. `GameActivity`
      opens a `.so` and calls into its glue, so Rust owns the entry
      point: `android_main` stashes the `AndroidApp` and calls up
      into Swift's `sf_android_main`, a `@_cdecl` in the app target,
      because `@main`'s generated entry point has no name a C `extern`
      can reach. The Rust staticlib is linked *into* the Swift `.so`, so
      the apparent circularity resolves in one link.
    - Frames are paced by **Choreographer**, not a timer — the Android
      equivalent of the `CADisplayLink` the iOS host uses, and the reason
      scrolling matches on a 90 or 120Hz panel.
      The callback must call `ALooper_wake` explicitly. `ALooper_pollAll`
      dispatches callback-registered fds *without returning to its
      caller*, so a vsync alone leaves the loop blocked and
      `about_to_wait` never runs — which showed up as animations
      advancing only while a finger was on the screen. A `WaitUntil` at
      twice the frame interval backstops a missed wake, so the worst case
      is half rate rather than a freeze.
    - **No Java or Kotlin *sources*, but one Java dependency.** The
      activity is `com.google.androidgamesdk.GameActivity`, used
      unmodified from `androidx.games:games-activity`. It replaced
      `android.app.NativeActivity`, and soft-keyboard text is the entire
      reason — see "Text input". Safe-area insets, the display's corner
      radius and its refresh rate still come from JNI reflection in
      `jni_metrics.rs`; the APK's assets are still unpacked through
      `AAssetManager` in `assets.rs`.
      The swap is four coupled edits, three of which fail at *runtime*
      rather than at build time, so `cli/src/gradle.rs` pins them
      together in one test: the activity name, `hasCode="true"` (an AAR
      means the APK has a DEX), a `Theme.AppCompat` parent (GameActivity
      extends `AppCompatActivity` and throws on a platform theme), and
      the dependency itself. Prefab stays off — the AAR also ships the
      C++ glue, and letting Gradle build it would duplicate the copy
      `android-activity` already compiled into the Rust staticlib.
      It also costs a build requirement: that glue is C++, compiled
      through cc-rs, so an Android build now needs the NDK's `clang++`
      and not just its linker. The CLI resolves it and passes it in
      `CXX_<triple>`, because cc-rs looks for a `<triple>-clang++` the
      NDK does not install under that name.
    - Vulkan in practice, and the reason is narrower than "GLES can't do
      it". GLES 3.1 has SSBOs; what it doesn't guarantee is *vertex-stage*
      ones — `GL_MAX_VERTEX_SHADER_STORAGE_BLOCKS` may legally be 0, and
      wgpu turns that query into `DownlevelFlags::VERTEX_STORAGE`, which
      the three storage bind groups here require because they're declared
      `VERTEX | FRAGMENT`. Plenty of Android GPUs report non-zero, so this
      is a per-device fact rather than a version rule.
      Each of the three vertex shaders reads exactly **one `vec4`** from
      its buffer (`inst.bounds` / `inst.frame`) — everything else,
      `fold_members` included, is fragment-side. So a GLES port's first
      step is small: pass that vec4 as a per-instance vertex attribute and
      drop `VERTEX` from the three `visibility` flags.
      Colour format looked like the larger blocker and wasn't a GLES
      problem at all — the swapchain format is negotiated now. See
      "Outstanding".
    - Nothing here needs a host-specific isolation escape any more. An
      earlier version of this file carried `assumeMain`, a
      `MainActor.assumeIsolated` with the thread check removed, because
      `android_main` runs on a thread `android-activity` spawns rather
      than the one Swift bound its main executor to. That could not work:
      `unsafeBitCast` drops the *type-level* annotation but cannot move
      the runtime executor, so the first dynamic check still trapped. The
      build path is nonisolated instead — see "Core invariants".

## Project configuration

One `SwiftFlow.toml` at a project's root describes the app on every
platform; the CLI lowers it into whatever each platform's own manifest
wants (`cli/src/config.rs`, lowered by `cli/src/gradle.rs`). It replaced
`.swiftflow-version`, which held the framework pin and nothing else —
that is `[swiftflow] version` now, and it is still read by the app's
`Package.swift`, which scans for that one key because manifest evaluation
has no TOML parser and no way to add a dependency on one. Nothing else in
the file is read by Swift.

Three tiers, and the split is about what generalises rather than about
convenience:

- **Canonical** (`[app]`, `[capabilities]`) — one meaning everywhere. An
  app has one identifier; that it is `CFBundleIdentifier` on one platform
  and `applicationId` on another is the CLI's problem. A capability is
  declared once and lowers to an `NS*UsageDescription` on iOS and a
  `<uses-permission>` on Android — the pair that is easiest to get out of
  step by hand, because each platform fails differently when it is
  missing (review rejection on one, a runtime denial on the other).
- **Typed platform** (`[ios]`, `[android]`, `[desktop]`) — concepts that
  genuinely do not generalise. `min_sdk` has no iOS meaning and
  `deployment_target` has no Android one, so a shared field would only
  make both wrong.
- **Raw passthrough** (`[ios.plist]`, `[android.manifest]`) — merged
  verbatim, winning last. This is what lets the canonical vocabulary stay
  small: without it, every attribute either platform has ever shipped
  eventually has to become a typed field, or the file gets abandoned the
  first time someone needs one that isn't there.

Precedence is **raw > typed > canonical** — closer to the metal wins.
Concretely, manifest generation writes the derived attributes first and
the raw table second, so raw overwrites by construction rather than by a
comparison someone has to remember to write.

Every section is `deny_unknown_fields`. A misspelled key that silently
does nothing is the worst failure mode a config file has, because it
looks set; `min_sdkk = 26` errors with the file, line, column and the
list of fields that exist. Unknown *capabilities* are rejected the same
way, and the error names both the vocabulary and the raw escape hatch.

`Config::unlowered()` reports what is parsed and validated but lowers to
nothing yet (icon generation, Info.plist merging), and `swiftflow doctor`
prints it. Same reasoning as `deny_unknown_fields`, one step further out:
a key that is accepted and then ignored is indistinguishable from one
that works, until the shipped build.

## Text input

Two channels, because no platform merges them and merging them breaks
input methods:

- **IME** carries *text* — composition-aware, so CJK, emoji pickers, dead
  keys and autocorrect work without the framework knowing about them. It
  can never say "backspace".
- **Keys** carry *commands* — delete, move, tab, escape. Deliberately
  only those: a printable character travelling on both channels is
  inserted twice, and the hosts drop it from the key channel to prevent
  exactly that.

Modelled on winit's `Ime` enum (`Enabled/Preedit/Commit/Disabled`)
because it is the shape all three platforms can express.

**Preedit is not document text.** It is what the input method is
composing, drawn at the caret and replaced wholesale until a commit turns
it into real text; if the IME is dismissed it is discarded. An editor
renders `document` plus `TextInput.preedit` and never inserts the latter.

`TextInput` (Input/TextInput.swift) owns focus and routing, keyed by node
id and cleared in `beginBuild` exactly like `GestureRegistry` — inside
the build, because the host skips rebuilding on idle frames and clearing
above that guard would drop every handler permanently.

`.textInput(isFocused:onInsert:onKey:)` (Input/KeyboardInput.swift) is
how a view reaches that: it mints or reuses the node's id, re-registers
handlers every frame so they close over current state, and claims focus.
Keys arrive as a `Key` enum and `KeyModifiers` option set rather than
raw `UInt32`s.

**There is no `TextField`, deliberately.** A text field is a policy — a
caret, a selection model, a set of editing verbs, a look — and an app
wanting different policy has to fight it. A code editor is exactly that
app: its caret belongs to a document model, its keys mean things a field
has never heard of, and its text is drawn by a syntax highlighter. So
the framework hands over the events. A `TextField` can be built *on*
this; it could not have been built out of one.

Platform reach differs, and an editor should expect it:

| | keys | commit | inline preedit |
|---|---|---|---|
| desktop | ✅ | ✅ | ✅ |
| iOS | via `UIKeyInput` | ✅ | ❌ — needs `UITextInput` |
| Android | ✅ | ✅ GameTextInput, or `KeyEvent.text` | ✅ via GameTextInput |

Android has **two** inbound channels for characters, and which one is
live depends on whether an input connection exists:

- **GameTextInput**, whenever the IME is active. This is the one that can
  compose, so it is the only route for CJK, prediction, swipe and emoji.
- **`KeyEvent.text`**, when it isn't. winit fills this from the device's
  own `KeyCharacterMap`, folding in a pending combining accent, so a
  physical keyboard types through it — as does a soft keyboard that sends
  key events instead of using an input connection. (`ReceivedCharacter`
  is the name this went by before winit 0.29; the variant no longer
  exists.)

They must never both be live for the same keystroke. Once an IME is
active Android routes hardware keys through it too, so the text lands in
GameTextInput's buffer *and* would arrive as key text — every letter
inserted twice. The host forwards key text only while the IME bridge is
disabled, which is exactly when nothing reaches that buffer.
`text_input::printable_text` adds the two rules that are the function's
own: never for a key with a command meaning (Enter's text is `\r`), and
never a control character (Escape's is `\u{1b}`).

**Android does not use winit for this, and cannot.** It is worth being
precise, because winit's documentation describes `WindowEvent::Ime` as
the way text arrives and that is true on desktop only. winit 0.30.13's
Android backend contains three IME functions, all outbound —
`set_ime_allowed` (show/hide the keyboard), `set_ime_cursor_area` (a
no-op) and `set_ime_purpose` (a no-op) — and nothing that constructs an
`Ime` event; only the x11, wayland, orbital, windows and macos backends
do. It also *discards* the event that carries the text: `android-activity`
delivers `InputEvent::TextEvent` through the same iterator winit drains
for touches and keys, and winit's match falls through to
`warn!("Unknown android_activity input event")`. winit 0.31.0-beta.2 is
no escape either — the Android backend was removed from the crate in
that restructure.

So `rust/swiftflow_android/src/ime.rs` reads GameTextInput directly, once
per loop iteration, next to the `set_ime_allowed` handling it mirrors.

That means bridging two models. SwiftFlow's `TextInput` is an *event* API
(`commit`, `setPreedit`); GameTextInput is a *shadow buffer* — it owns a
string, a selection and a composing region, and the IME edits it. The
bridge keeps that buffer empty except while something is in flight: a
composing region is reported as a preedit and the buffer is left strictly
alone (touching it mid-composition cancels the composition), and once the
region clears, the text is committed and the buffer reset. The cost is
that the IME never sees the surrounding document, so cross-word
autocorrect has less to work with; the alternative is mirroring the whole
document into GameTextInput on every keystroke and reconciling two models
of the same text.

One hazard worth naming: the composing region's indices are **byte**
offsets that arrive from Java and are clamped only to the string's
length, not to a character boundary. Slicing with them directly panics on
exactly the multi-byte text an IME exists for, which is why the bridge
goes through a boundary-safe slice.

## Outstanding

- Full modifier suite.
- Liquid glass and `ProgressView` (the animation system they were waiting
  on now exists: curves, `withAnimation`, `.transition()` for views
  coming and going, and `.contentTransition()` for a view whose content
  changed underneath it).
- Android validation end to end: everything above is written and the
  Rust half cross-compiles clean, but nothing has run on a device.
- A GLES fallback for Android, if a real device ever turns out to need
  one. Costed, not started, in two unequal halves:
    1. **Vertex-stage storage, small.** Each of the three vertex shaders
       reads exactly one `vec4` from its storage buffer (`inst.bounds` /
       `inst.frame`). Pass it as a per-instance vertex attribute instead
       and drop `VERTEX` from the three `visibility` flags, and
       `DownlevelFlags::VERTEX_STORAGE` stops being required at all.
    2. **Colour format — done, and it was not a GLES problem.** The
       swapchain format was hardcoded `Bgra8Unorm`, which Apple's
       CAMetalLayer happens to be. Android Vulkan offers only
       `[Rgba8Unorm, Rgba8UnormSrgb, Rgba16Float]`, so `surface.configure`
       rejected it and wgpu aborted. It's now negotiated from
       `surface.get_capabilities`, preferring `Bgra8Unorm`, then any
       non-sRGB format (an `*Srgb` target would apply a linear→sRGB
       conversion on write and shift every colour). Only the surface
       config and the present pipeline needed it — the scene renders to
       an offscreen `Bgra8Unorm` texture, so nothing else changed.
  Not worth doing on spec: the Swift SDK's own floor (API 28, 64-bit)
  already excludes the devices that would need it, and a second backend
  doubles a visual test matrix that has no automated coverage.
- A `MeshGradient` equivalent (currently stubbed with blurred-cover
  backgrounds in Ichi ports).
- Open questions from the Ichi port: the FlowChips wrap-width
  chicken-and-egg problem. (`.lineLimit` and the scroll-offset question
  are both answered — the latter by `ScrollGeometry`, read during the
  build rather than delivered by a callback, which is why it needs no
  `@State` and is never a frame behind. See its doc comment.)
- Liquid glass specular highlight shader for SDF rounded rects —
  designed, targeted at SwiftFlow rather than Ichi so it's only written
  once.

## Licensing note

SF Symbols and SF Pro have licensing constraints when used outside
Apple's own frameworks. Worth resolving before the design system
hardens.
