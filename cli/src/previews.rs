//! Discovery of `#Preview` declarations.
//!
//! Scans a project's `Sources/` for `#Preview("Name") { … }`, derives the same
//! `__SFPreview_<name>_<hash>` symbol the `#Preview` macro emits, rejects
//! duplicate names, and writes `.build/swiftflow-previews/PreviewIndex.swift`:
//! a Swift file that references each symbol and exposes the C entry points the
//! preview host `dlsym`s.
//!
//! The symbol derivation here and in the macro (macros/.../PreviewMacro.swift,
//! `PreviewSymbol.derive`) must stay byte-for-byte identical — same FNV-1a over
//! the name's UTF-8, same sanitization, same 8-hex zero-pad. If the two drift,
//! the generated index references a symbol nothing defines and the preview
//! dylib fails to link. The tests below pin concrete names to concrete symbols;
//! the macro's test target pins the same pairs on the Swift side.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The characters between the quotes exactly as written in source — the
    /// same bytes the macro hashes.
    pub name: String,
    /// `__SFPreview_<sanitized>_<8hex>`.
    pub symbol: String,
    /// Where it was found, for duplicate diagnostics.
    pub file: PathBuf,
}

/// The one derivation, matched in `PreviewSymbol.derive` on the Swift side.
pub fn symbol_for(name: &str) -> String {
    let mut sanitized = String::new();
    for &byte in name.as_bytes() {
        if byte.is_ascii_alphanumeric() {
            sanitized.push(byte as char);
        } else {
            sanitized.push('_');
        }
    }

    let mut hash: u32 = 2166136261;
    for &byte in name.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }

    format!("__SFPreview_{sanitized}_{hash:08x}")
}

/// Every preview in the tree, in a stable order (files sorted, then source
/// order within a file), so the generated index — and the host's picker — is
/// deterministic across runs.
pub fn scan(sources: &Path) -> Result<Vec<Preview>, String> {
    let mut files = Vec::new();
    collect_swift(sources, &mut files);
    files.sort();

    let mut previews = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        for name in scan_text(&text) {
            let symbol = symbol_for(&name);
            previews.push(Preview { name, symbol, file: file.clone() });
        }
    }

    reject_duplicates(&previews)?;
    Ok(previews)
}

fn reject_duplicates(previews: &[Preview]) -> Result<(), String> {
    for (i, a) in previews.iter().enumerate() {
        for b in &previews[i + 1..] {
            // A repeated name (the common case) and a genuine hash collision
            // between different names both land the index on one symbol.
            if a.name == b.name {
                return Err(format!(
                    "two previews are both named {:?}\n  {}\n  {}\n  \
                     preview names must be unique across the project",
                    a.name, a.file.display(), b.file.display()
                ));
            }
            if a.symbol == b.symbol {
                return Err(format!(
                    "previews {:?} and {:?} collide on symbol {} — rename one",
                    a.name, b.name, a.symbol
                ));
            }
        }
    }
    Ok(())
}

fn collect_swift(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // The generated index lives under .build; never scan ourselves.
            let skip = matches!(path.file_name().and_then(|n| n.to_str()), Some(".build"));
            if !skip {
                collect_swift(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("swift") {
            out.push(path);
        }
    }
}

/// Finds `#Preview("...")` and returns each name's raw inner text. Line
/// comments are stripped first so a commented-out preview isn't discovered;
/// string-embedded or block-commented occurrences are a deliberate non-goal.
fn scan_text(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let code = strip_line_comment(line);
        let bytes = code.as_bytes();
        let mut i = 0;
        while let Some(rel) = code[i..].find("#Preview") {
            let mut j = i + rel + "#Preview".len();
            j = skip_ws(bytes, j);
            if bytes.get(j) != Some(&b'(') {
                i = i + rel + "#Preview".len();
                continue;
            }
            j = skip_ws(bytes, j + 1);
            if bytes.get(j) != Some(&b'"') {
                i = i + rel + "#Preview".len();
                continue;
            }
            if let Some((name, end)) = read_string_literal(bytes, j) {
                names.push(name);
                i = end;
            } else {
                i = i + rel + "#Preview".len();
            }
        }
    }
    names
}

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(pos) => &line[..pos],
        None => line,
    }
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while matches!(bytes.get(i), Some(b) if b.is_ascii_whitespace()) {
        i += 1;
    }
    i
}

/// Reads a `"..."` starting at `open` (the opening quote), returning the raw
/// inner text (escapes left verbatim, matching the macro) and the index just
/// past the closing quote.
fn read_string_literal(bytes: &[u8], open: usize) -> Option<(String, usize)> {
    let mut i = open + 1;
    let start = i;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,           // skip an escape pair, e.g. \" or \\
            b'"' => {
                let raw = std::str::from_utf8(&bytes[start..i]).ok()?.to_string();
                return Some((raw, i + 1));
            }
            _ => i += 1,
        }
    }
    None
}

/// Writes `PreviewIndex.swift` into `out_dir`, creating it. Overwrites each
/// build so a removed preview drops out of the index.
pub fn write_index(out_dir: &Path, previews: &[Preview]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    let path = out_dir.join("PreviewIndex.swift");
    std::fs::write(&path, render_index(previews)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

fn render_index(previews: &[Preview]) -> String {
    // Each preview's view is boxed by a `@_cdecl` thunk the macro emits, under
    // the stable per-preview symbol. Reach those symbols by `@_silgen_name`
    // forward declaration — they're defined in the same dylib, in the app's own
    // sources — and index them.
    let decls = previews
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "@_silgen_name({:?})\nfunc __sfPreviewThunk{i}() -> UnsafeMutableRawPointer",
                p.symbol
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let makers = previews
        .iter()
        .enumerate()
        .map(|(i, _)| format!("    __sfPreviewThunk{i},"))
        .collect::<Vec<_>>()
        .join("\n");

    let names = previews
        .iter()
        .map(|p| format!("    __sfCopy({:?}),", p.name))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"// Generated by `swiftflow` — do not edit.
//
// Every #Preview found in this project, exposed to the preview host across a
// stable C ABI. Each `__SFPreview_*` symbol is a `@_cdecl` thunk the #Preview
// macro emits; it returns a PreviewBox (a SwiftFlowCore type, so both sides
// agree on it) as an Unmanaged-retained pointer. sf_preview_make forwards it.
import Foundation
import SwiftFlow

{decls}

nonisolated(unsafe) private let __sfPreviewMakers: [() -> UnsafeMutableRawPointer] = [
{makers}
]

nonisolated(unsafe) private let __sfPreviewNames: [UnsafePointer<CChar>] = [
{names}
]

// strdup once, never freed: these strings live as long as the host does.
private func __sfCopy(_ string: String) -> UnsafePointer<CChar> {{
    UnsafePointer(strdup(string)!)
}}

@_cdecl("sf_preview_count")
public func sf_preview_count() -> Int32 {{
    Int32(__sfPreviewMakers.count)
}}

@_cdecl("sf_preview_name")
public func sf_preview_name(_ index: Int32) -> UnsafePointer<CChar>? {{
    let i = Int(index)
    guard i >= 0, i < __sfPreviewNames.count else {{ return nil }}
    return __sfPreviewNames[i]
}}

@_cdecl("sf_preview_make")
public func sf_preview_make(_ index: Int32) -> UnsafeMutableRawPointer? {{
    let i = Int(index)
    guard i >= 0, i < __sfPreviewMakers.count else {{ return nil }}
    return __sfPreviewMakers[i]()
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // These exact pairs are mirrored in the macro's test target. If either
    // side changes, both must — that is the whole contract.
    #[test]
    fn symbols_match_the_pinned_contract() {
        assert_eq!(symbol_for("Counter"), "__SFPreview_Counter_fc0f33c3");
        assert_eq!(symbol_for("Sidebar Row"), "__SFPreview_Sidebar_Row_ed59821b");
        assert_eq!(symbol_for(""), "__SFPreview__811c9dc5");
    }

    #[test]
    fn sanitization_replaces_every_non_alnum_byte() {
        assert_eq!(&symbol_for("A B-c.d")[..14], "__SFPreview_A_");
        assert!(symbol_for("Hello, World!").starts_with("__SFPreview_Hello__World__"));
    }

    #[test]
    fn scan_finds_names_and_skips_comments() {
        let src = r#"
            struct V: View { var body: some View { Text("hi") } }
            #Preview("First") { V() }
            // #Preview("Commented") { V() }
            #Preview ( "Second" ) { V() }
        "#;
        let names = scan_text(src);
        assert_eq!(names, vec!["First".to_string(), "Second".to_string()]);
    }

    #[test]
    fn scan_reads_escaped_quotes_verbatim() {
        let names = scan_text(r#"#Preview("a\"b") { V() }"#);
        assert_eq!(names, vec![r#"a\"b"#.to_string()]);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let previews = vec![
            Preview { name: "Dup".into(), symbol: symbol_for("Dup"), file: "a.swift".into() },
            Preview { name: "Dup".into(), symbol: symbol_for("Dup"), file: "b.swift".into() },
        ];
        assert!(reject_duplicates(&previews).is_err());
    }

    #[test]
    fn the_index_references_every_symbol() {
        let previews = vec![
            Preview { name: "Counter".into(), symbol: symbol_for("Counter"), file: "a.swift".into() },
        ];
        let index = render_index(&previews);
        assert!(index.contains(r#"@_silgen_name("__SFPreview_Counter_fc0f33c3")"#));
        assert!(index.contains("__sfPreviewThunk0,"));
        assert!(index.contains("__sfCopy(\"Counter\"),"));
        assert!(index.contains(r#"@_cdecl("sf_preview_make")"#));
    }

    #[test]
    fn an_empty_project_still_renders_a_valid_index() {
        let index = render_index(&[]);
        assert!(index.contains("sf_preview_count"));
        assert!(index.contains("Int32(__sfPreviewMakers.count)"));
    }
}

// ── Stage 2b: the preview host package ────────────────────────────────────
//
// `swiftflow preview` assembles a throwaway SwiftPM package under
// `.build/swiftflow-previews/` with two products built in one pass — so they
// share a single SwiftFlowCore image (stage 2a):
//
//   PreviewViews (.dynamic)  the app's sources (symlinked) + the generated
//                            index, exposing sf_preview_* across the C ABI
//   PreviewHost  (executable) a small SwiftFlow app that dlopens that dylib,
//                            reads the previews, and renders the selected one.
//
// The host never links the views: it dlopens them, so a trapping preview body
// takes down the host process alone, and a rebuild is just a fresh dylib.

/// The host executable's source. Fixed boilerplate — it doesn't depend on the
/// app, only on the sf_preview_* C ABI the index exposes.
pub const PREVIEW_HOST_MAIN: &str = r##"import Foundation
import SwiftFlow

private typealias CountFn = @convention(c) () -> Int32
private typealias NameFn = @convention(c) (Int32) -> UnsafePointer<CChar>?
private typealias MakeFn = @convention(c) (Int32) -> UnsafeMutableRawPointer?

/// Loads a views dylib and hands its previews back as SwiftFlow views, swapping
/// in a freshly built dylib when one appears (live reload).
final class PreviewCatalog {
    nonisolated(unsafe) static let shared = PreviewCatalog()

    // Bumped on every reload. The framework's observation is coarse — setting
    // this marks the tree dirty, so the window re-renders with the new views.
    @Observed private(set) var revision = 0
    private(set) var names: [String] = []
    private var makeFn: MakeFn?
    private var cache: [Int: any View] = [:]
    private var loadedPath = ""

    /// `currentFile` holds the path of the dylib to show. Loads it only when it
    /// names a dylib we haven't loaded yet.
    func loadIfChanged(from currentFile: String) {
        guard let path = (try? String(contentsOfFile: currentFile, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !path.isEmpty, path != loadedPath
        else { return }

        guard let handle = dlopen(path, RTLD_NOW) else {
            let message = dlerror().map { String(cString: $0) } ?? "unknown error"
            print("preview: could not open \(path): \(message)")
            return
        }
        // Never dlclose the old handle: unloading a Swift dylib with live type
        // metadata crashes. Each reload leaks one handle, by design.
        guard let countSym = dlsym(handle, "sf_preview_count"),
              let nameSym = dlsym(handle, "sf_preview_name"),
              let makeSym = dlsym(handle, "sf_preview_make")
        else {
            print("preview: \(path) is missing sf_preview_* symbols")
            return
        }
        let count = unsafeBitCast(countSym, to: CountFn.self)()
        let nameFn = unsafeBitCast(nameSym, to: NameFn.self)
        makeFn = unsafeBitCast(makeSym, to: MakeFn.self)
        names = (0..<count).map { index in
            nameFn(index).map { String(cString: $0) } ?? "Preview \(Int(index))"
        }
        cache.removeAll()
        loadedPath = path
        revision += 1
    }

    /// Builds (once) and returns the view for a preview. Building here, lazily,
    /// is what keeps a trap contained to the preview the user actually opened.
    func view(at index: Int) -> any View {
        if let cached = cache[index] { return cached }
        guard let makeFn, let raw = makeFn(Int32(index)) else {
            return Text("No preview")
        }
        let box = Unmanaged<PreviewBox>.fromOpaque(raw).takeRetainedValue()
        let view = box.makeView()
        cache[index] = view
        return view
    }
}

struct PreviewHostView: View {
    @State private var selected = 0

    var body: some View {
        let catalog = PreviewCatalog.shared
        let index = catalog.names.isEmpty ? 0 : min(selected, catalog.names.count - 1)
        return VStack(spacing: 0) {
            if catalog.names.count > 1 {
                HStack(spacing: 8) {
                    ForEach(catalog.names.indices, id: \.self) { i in
                        Button(catalog.names[i]) { selected = i }
                    }
                }
                .padding(10)
            }
            AnyView(erasing: catalog.view(at: index))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

@main
struct PreviewHostApp: SwiftFlowApp {
    init() {
        guard let currentFile = CommandLine.arguments.dropFirst().first else {
            print("preview: no reload file path was given")
            return
        }
        PreviewCatalog.shared.loadIfChanged(from: currentFile)

        // Poll the reload file off the render thread. When the CLI rebuilds and
        // points it at a new dylib, hand the reload to the render thread — dlopen
        // and view construction must not race the frame that reads them.
        Thread.detachNewThread {
            var last = (try? String(contentsOfFile: currentFile, encoding: .utf8)) ?? ""
            while true {
                Thread.sleep(forTimeInterval: 0.3)
                let now = (try? String(contentsOfFile: currentFile, encoding: .utf8)) ?? ""
                if now != last {
                    last = now
                    MainQueue.post { PreviewCatalog.shared.loadIfChanged(from: currentFile) }
                }
            }
        }
    }

    var body: some Scene {
        WindowGroup {
            PreviewHostView()
        }
    }
}
"##;

/// The headless host CodeFlow spawns: renders the preview offscreen and
/// publishes each BGRA frame into an mmap'd transport file. No window.
pub const PREVIEW_OFFSCREEN_HOST_MAIN: &str = r##"import Foundation
import SwiftFlow

private typealias MakeFn = @convention(c) (Int32) -> UnsafeMutableRawPointer?

/// Loads a views dylib and returns the first preview as a view, reloading when
/// the CLI points the reload file at a freshly built dylib.
final class OffscreenCatalog {
    nonisolated(unsafe) static let shared = OffscreenCatalog()
    @Observed private(set) var revision = 0
    private var makeFn: MakeFn?
    private var cached: (any View)?
    private var loadedPath = ""

    func loadIfChanged(from currentFile: String) {
        guard let path = (try? String(contentsOfFile: currentFile, encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !path.isEmpty, path != loadedPath
        else { return }
        // Never dlclose: unloading a Swift dylib with live metadata crashes.
        guard let handle = dlopen(path, RTLD_NOW),
              let makeSym = dlsym(handle, "sf_preview_make")
        else { return }
        makeFn = unsafeBitCast(makeSym, to: MakeFn.self)
        cached = nil
        loadedPath = path
        revision += 1
    }

    func view() -> any View {
        if let cached { return cached }
        guard let makeFn, let raw = makeFn(0) else { return Text("No preview") }
        let box = Unmanaged<PreviewBox>.fromOpaque(raw).takeRetainedValue()
        let view = box.makeView()
        cached = view
        return view
    }
}

struct OffscreenRoot: View {
    var body: some View {
        let catalog = OffscreenCatalog.shared
        _ = catalog.revision
        return AnyView(erasing: catalog.view())
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// A file the editor and this process both mmap: a 64-byte header
/// (magic, width, height, and a seqno bumped last) then BGRA pixels.
final class FrameTransport {
    static let headerSize = 64
    static let magic: UInt32 = 0x5346_5056 // 'SFPV'
    private let base: UnsafeMutableRawPointer
    private let capacity: Int
    private var seq: UInt64 = 0

    init?(path: String, maxWidth: Int, maxHeight: Int) {
        capacity = FrameTransport.headerSize + maxWidth * maxHeight * 4
        let fd = open(path, O_RDWR | O_CREAT, 0o644)
        if fd < 0 { return nil }
        if ftruncate(fd, off_t(capacity)) != 0 { close(fd); return nil }
        guard let mapped = mmap(nil, capacity, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0),
              mapped != MAP_FAILED
        else { close(fd); return nil }
        base = mapped
        close(fd) // the mapping keeps the file alive
    }

    func publish(_ pixels: UnsafePointer<UInt8>, width: Int, height: Int) {
        let bytes = width * height * 4
        guard FrameTransport.headerSize + bytes <= capacity else { return }
        memcpy(base.advanced(by: FrameTransport.headerSize), pixels, bytes)
        let words = base.assumingMemoryBound(to: UInt32.self)
        words[0] = FrameTransport.magic
        words[1] = UInt32(width)
        words[2] = UInt32(height)
        // seqno last (release): the reader treats a changed seqno as "new frame ready".
        seq &+= 1
        base.advanced(by: 16).assumingMemoryBound(to: UInt64.self).pointee = seq
    }
}

/// Reads input events the editor pane writes into a shared ring: a 64-byte
/// header holding a monotonically increasing write index, then 256 fixed slots
/// of {kind:u32, x,y,dx,dy:f32}. Single writer, single reader.
final class EventChannel {
    static let slots = 256
    static let slotSize = 32
    static let headerSize = 64
    static let byteSize = headerSize + slots * slotSize
    private let base: UnsafeMutableRawPointer
    private var readIndex: UInt64 = 0

    init?(path: String) {
        let fd = open(path, O_RDWR | O_CREAT, 0o644)
        if fd < 0 { return nil }
        if ftruncate(fd, off_t(EventChannel.byteSize)) != 0 { close(fd); return nil }
        guard let mapped = mmap(nil, EventChannel.byteSize, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0),
              mapped != MAP_FAILED
        else { close(fd); return nil }
        base = mapped
        close(fd)
    }

    func drain() -> [PreviewInput] {
        let write = base.assumingMemoryBound(to: UInt64.self).pointee
        var events: [PreviewInput] = []
        // Skip past anything already overwritten if the writer got far ahead.
        if write > readIndex + UInt64(EventChannel.slots) {
            readIndex = write - UInt64(EventChannel.slots)
        }
        while readIndex < write {
            let slot = base.advanced(by:
                EventChannel.headerSize + Int(readIndex % UInt64(EventChannel.slots)) * EventChannel.slotSize)
            let kind = slot.assumingMemoryBound(to: UInt32.self).pointee
            let f = slot.advanced(by: 4).assumingMemoryBound(to: Float.self)
            switch kind {
            case 1: events.append(.pointerDown(x: f[0], y: f[1]))
            case 2: events.append(.pointerMoved(x: f[0], y: f[1]))
            case 3: events.append(.pointerUp(x: f[0], y: f[1]))
            case 4: events.append(.scroll(x: f[0], y: f[1], dx: f[2], dy: f[3]))
            default: break
            }
            readIndex += 1
        }
        return events
    }
}

let args = CommandLine.arguments
guard args.count >= 6,
      let widthPoints = Int(args[3]),
      let heightPoints = Int(args[4]),
      let scale = Float(args[5])
else {
    FileHandle.standardError.write(
        Data("usage: PreviewOffscreenHost <reloadFile> <transport> <w> <h> <scale>\n".utf8))
    exit(1)
}
let reloadFile = args[1]
let transportPath = args[2]

OffscreenCatalog.shared.loadIfChanged(from: reloadFile)

// Poll the reload file off the render thread; hand reloads to it via MainQueue.
Thread.detachNewThread {
    var last = (try? String(contentsOfFile: reloadFile, encoding: .utf8)) ?? ""
    while true {
        Thread.sleep(forTimeInterval: 0.3)
        let now = (try? String(contentsOfFile: reloadFile, encoding: .utf8)) ?? ""
        if now != last {
            last = now
            MainQueue.post { OffscreenCatalog.shared.loadIfChanged(from: reloadFile) }
        }
    }
}

guard let transport = FrameTransport(
    path: transportPath,
    maxWidth: Int(Float(widthPoints) * scale),
    maxHeight: Int(Float(heightPoints) * scale)
) else {
    FileHandle.standardError.write(Data("preview: could not open transport\n".utf8))
    exit(2)
}

let events = EventChannel(path: transportPath + ".events")

MainActor.assumeIsolated {
    DesktopHost.shared.runOffscreen(
        root: OffscreenRoot(),
        widthPoints: widthPoints,
        heightPoints: heightPoints,
        scale: scale,
        input: { events?.drain() ?? [] }
    ) { pixels, width, height in
        transport.publish(pixels, width: width, height: height)
    }
}
"##;

/// Lays out and writes the preview package under `pkg_dir`. `app_sources` is the
/// app's `Sources/<App>` (symlinked in as `AppSources`), `desktop_pkg`/`macros_pkg`
/// are the resolved framework package paths, and `has_assets` says whether to
/// declare the Assets resource.
pub fn write_preview_package(
    pkg_dir: &Path,
    app_sources: &Path,
    previews: &[Preview],
    desktop_pkg: &Path,
    macros_pkg: &Path,
    has_assets: bool,
) -> Result<(), String> {
    let make = |dir: &Path| std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()));
    make(pkg_dir)?;
    make(&pkg_dir.join("Index"))?;
    make(&pkg_dir.join("Host"))?;

    // AppSources -> the app's real Sources/<App>. A symlink rather than a copy
    // so edits show up on the next `swiftflow preview` with nothing to sync,
    // and so every target path stays inside the package.
    let link = pkg_dir.join("AppSources");
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(app_sources, &link)
        .map_err(|e| format!("linking AppSources -> {}: {e}", app_sources.display()))?;

    std::fs::write(pkg_dir.join("Index/PreviewIndex.swift"), render_index(previews))
        .map_err(|e| format!("writing index: {e}"))?;
    std::fs::write(pkg_dir.join("Host/main.swift"), PREVIEW_HOST_MAIN)
        .map_err(|e| format!("writing host: {e}"))?;
    std::fs::create_dir_all(pkg_dir.join("OffscreenHost"))
        .map_err(|e| format!("{}: {e}", pkg_dir.join("OffscreenHost").display()))?;
    std::fs::write(pkg_dir.join("OffscreenHost/main.swift"), PREVIEW_OFFSCREEN_HOST_MAIN)
        .map_err(|e| format!("writing offscreen host: {e}"))?;
    std::fs::write(pkg_dir.join("Package.swift"), render_manifest(desktop_pkg, macros_pkg, has_assets))
        .map_err(|e| format!("writing manifest: {e}"))?;
    Ok(())
}

fn render_manifest(desktop_pkg: &Path, macros_pkg: &Path, has_assets: bool) -> String {
    let resources = if has_assets {
        ",\n            resources: [.copy(\"Assets\")]"
    } else {
        ""
    };
    format!(
        r#"// swift-tools-version: 6.0
// Generated by `swiftflow preview` — do not edit.
import PackageDescription

let package = Package(
    name: "SwiftFlowPreviews",
    platforms: [.macOS(.v14)],
    products: [
        // One build, two products, so they share a single SwiftFlowCore dylib.
        .library(name: "PreviewViews", type: .dynamic, targets: ["PreviewApp", "PreviewIndex"]),
        .executable(name: "PreviewHost", targets: ["PreviewHost"]),
        .executable(name: "PreviewOffscreenHost", targets: ["PreviewOffscreenHost"]),
    ],
    dependencies: [
        .package(name: "SwiftFlowDesktop", path: {desktop:?}),
        .package(name: "SwiftFlowMacros", path: {macros:?}),
    ],
    targets: [
        // The app's own sources (symlinked). Its @main is inert in a library.
        .target(
            name: "PreviewApp",
            dependencies: [
                .product(name: "SwiftFlow", package: "SwiftFlowDesktop"),
                .product(name: "SwiftFlowMacros", package: "SwiftFlowMacros"),
            ],
            path: "AppSources"{resources}
        ),
        // The generated index, reaching the app's @_cdecl thunks by @_silgen_name;
        // depends on PreviewApp so they link into one dylib.
        .target(
            name: "PreviewIndex",
            dependencies: [
                "PreviewApp",
                .product(name: "SwiftFlow", package: "SwiftFlowDesktop"),
            ],
            path: "Index"
        ),
        .executableTarget(
            name: "PreviewHost",
            dependencies: [.product(name: "SwiftFlow", package: "SwiftFlowDesktop")],
            path: "Host"
        ),
        .executableTarget(
            name: "PreviewOffscreenHost",
            dependencies: [.product(name: "SwiftFlow", package: "SwiftFlowDesktop")],
            path: "OffscreenHost"
        ),
    ]
)
"#,
        desktop = desktop_pkg.to_string_lossy(),
        macros = macros_pkg.to_string_lossy(),
        resources = resources,
    )
}
