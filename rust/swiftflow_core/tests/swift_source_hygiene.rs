use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate should sit two levels below the repository root")
        .to_path_buf()
}

fn swift_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => panic!("cannot read {}: {e}", dir.display()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "swift") {
                found.push(path);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "found no Swift under {} — this test stopped testing anything",
        root.display()
    );
    found
}

fn code_only(source: &str) -> String {

    let stripped: String = source
        .split("\"\"\"")
        .step_by(2)
        .collect::<Vec<_>>()
        .join("\n");

    stripped
        .lines()
        .map(|line| line.split("//").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

const FRAMEWORK_SYMBOLS: [&str; 6] = [
    "@Observed",
    "@State",
    "@Environment",
    "some View",
    "Observable",
    "SwiftFlowApp",
];

#[test]
fn codeflow_files_using_the_framework_import_it() {
    let root = repo_root().join("CodeFlow/Sources");
    for path in swift_files(&root) {
        let source = std::fs::read_to_string(&path).unwrap();
        let code = code_only(&source);

        let used: Vec<&str> = FRAMEWORK_SYMBOLS
            .iter()
            .copied()
            .filter(|symbol| code.contains(symbol))
            .collect();
        if used.is_empty() {
            continue;
        }

        assert!(
            code.lines()
                .any(|line| line.trim_end() == "import SwiftFlow"),
            "{} uses {used:?} but never imports SwiftFlow",
            path.strip_prefix(repo_root()).unwrap_or(&path).display()
        );
    }
}

const FRAME_ORDER: [&str; 7] = [
    "width",
    "height",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "alignment",
];

fn frame_calls(code: &str) -> Vec<(usize, Vec<&str>)> {
    let bytes = code.as_bytes();
    let mut calls = Vec::new();
    let mut search = 0;

    while let Some(found) = code[search..].find(".frame(") {
        let open = search + found + ".frame(".len();
        search = open;

        let mut depth = 1usize;
        let mut labels = Vec::new();
        let mut i = open;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b':' if depth == 1 => {

                    let start = code[..i]
                        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .map_or(0, |b| b + 1);
                    let word = &code[start..i];
                    if let Some(label) = FRAME_ORDER.iter().find(|l| **l == word) {
                        labels.push(*label);
                    }
                }
                _ => {}
            }
            i += 1;
        }

        let line = code[..open].lines().count();
        calls.push((line, labels));
    }
    calls
}

fn is_ordered(labels: &[&str]) -> bool {
    let mut expected = FRAME_ORDER.iter();
    labels
        .iter()
        .all(|label| expected.any(|candidate| candidate == label))
}

#[test]
fn frame_arguments_are_in_declaration_order() {
    let root = repo_root();
    let mut checked = 0;
    for area in ["CodeFlow/Sources", "Sources/SwiftFlowCore"] {
        for path in swift_files(&root.join(area)) {
            let source = std::fs::read_to_string(&path).unwrap();
            let code = code_only(&source);
            for (line, labels) in frame_calls(&code) {
                checked += 1;
                assert!(
                    is_ordered(&labels),
                    "{}:{line}: .frame({}) — arguments must be in the order \
                     {FRAME_ORDER:?}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    labels
                        .iter()
                        .map(|l| format!("{l}:"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    }
    assert!(
        checked > 10,
        "only {checked} frame calls found — the scanner has stopped matching"
    );
}

#[test]
fn the_scanner_rejects_what_the_compiler_rejected() {
    let bad = ".frame(maxWidth: .infinity, height: Metrics.statusBarHeight, alignment: .center)";
    let (_, labels) = frame_calls(bad).remove(0);
    assert_eq!(labels, ["maxWidth", "height", "alignment"]);
    assert!(!is_ordered(&labels), "this is the error that shipped");

    for good in [
        ".frame(height: Metrics.tabHeight, maxWidth: .infinity, alignment: .leading)",
        ".frame(width: 12, height: 1)",
        ".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)",

        ".frame(width: max(a, b), height: wide ? 10 : 20)",
    ] {
        let (_, labels) = frame_calls(good).remove(0);
        assert!(is_ordered(&labels), "{good} is legal Swift: {labels:?}");
    }

    assert_eq!(frame_calls(".frame(width: 1)\n.frame(height: 2)").len(), 2);
}

#[test]
fn strings_and_comments_are_not_source() {
    let sample = code_only("let s = #\"\"\"\n@State private var count = 0\n\"\"\"#\nlet x = 1");
    assert!(!sample.contains("@State"), "raw string leaked: {sample:?}");
    assert!(sample.contains("let x = 1"), "code after a string was lost");

    let doc = code_only("/// `didSet` rather than `@Observed`.\nlet y = 2");
    assert!(!doc.contains("@Observed"), "comment leaked: {doc:?}");
    assert!(doc.contains("let y = 2"));
}

fn axis_writes(code: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        for axis in ["sizingX", "sizingY"] {

            if let Some(at) = line.find(&format!("{axis} =")) {
                if !line[at..].starts_with(&format!("{axis} ==")) {
                    found.push((index + 1, axis));
                }
            }
        }
    }
    found
}

#[test]
fn a_file_that_sets_one_sizing_axis_sets_the_other() {
    let root = repo_root();
    let mut checked = 0;
    for area in ["CodeFlow/Sources", "Sources/SwiftFlowCore"] {
        for path in swift_files(&root.join(area)) {
            let source = std::fs::read_to_string(&path).unwrap();
            let writes = axis_writes(&code_only(&source));
            if writes.is_empty() {
                continue;
            }
            checked += writes.len();
            let name = path.strip_prefix(&root).unwrap_or(&path).display();
            for axis in ["sizingX", "sizingY"] {
                assert!(
                    writes.iter().any(|(_, a)| *a == axis),
                    "{name} assigns {} but never {axis} — the unset axis \
                     stays Hug, which is silent. Use `node.sizing` if both \
                     axes really do mean the same thing.",
                    if axis == "sizingX" { "sizingY" } else { "sizingX" }
                );
            }
        }
    }
    assert!(
        checked >= 8,
        "only {checked} per-axis writes found — the scanner has stopped matching"
    );
}

#[test]
fn the_axis_scanner_reads_assignments_only() {
    assert_eq!(axis_writes("node.sizingX = SF_SIZING_FILL"), [(1, "sizingX")]);
    assert_eq!(axis_writes("guard child.sizingX == SF_SIZING_FILL"), []);
    assert_eq!(axis_writes("node.sizingY = child.sizingY.inherited"), [(1, "sizingY")]);
    assert_eq!(axis_writes("node.sizing = SF_SIZING_HUG"), []);
}
