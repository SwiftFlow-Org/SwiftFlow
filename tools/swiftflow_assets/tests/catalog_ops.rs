use std::fs;
use std::path::{Path, PathBuf};
use swiftflow_assets::{flatten, Catalog, Scale};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sf-assets-{label}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_png(path: &Path) {
    const PIXEL: [u8; 67] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::write(path, PIXEL).unwrap();
}

#[test]
fn opening_creates_a_catalogue_xcode_would_recognise() {
    let tmp = TempDir::new("open");
    let root = tmp.path().join("Media.xcassets");
    Catalog::open(&root).unwrap();

    let contents = fs::read_to_string(root.join("Contents.json")).unwrap();
    assert!(contents.contains("\"author\" : \"xcode\""));
    assert!(contents.contains("\"version\" : 1"));
}

#[test]
fn creating_a_set_makes_an_imageset_folder() {
    let tmp = TempDir::new("create");
    let mut catalog = Catalog::open(tmp.path()).unwrap();
    catalog.create_set("logo").unwrap();

    assert!(tmp.path().join("logo.imageset/Contents.json").is_file());
    assert_eq!(catalog.sets.len(), 1);
    assert_eq!(catalog.sets[0].name, "logo");
    assert!(catalog.sets[0].is_empty());
}

#[test]
fn duplicate_and_malformed_names_are_refused() {
    let tmp = TempDir::new("names");
    let mut catalog = Catalog::open(tmp.path()).unwrap();
    catalog.create_set("logo").unwrap();

    assert!(catalog.create_set("logo").is_err(), "duplicate");

    assert!(catalog.create_set("nested/logo").is_err(), "separator");
    assert!(catalog.create_set("").is_err(), "empty");
    assert!(catalog.create_set(".hidden").is_err(), "leading dot");
}

#[test]
fn filling_a_slot_copies_and_renames_the_source() {
    let tmp = TempDir::new("slot");
    let source = tmp.path().join("some-download.png");
    write_png(&source);

    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("logo").unwrap();
    catalog.set_slot("logo", Scale::Three, &source).unwrap();

    assert!(root.join("logo.imageset/logo@3x.png").is_file());
    let set = catalog.get("logo").unwrap();
    assert_eq!(
        set.slots()[2].filename.as_deref(),
        Some("logo@3x.png"),
        "3x well should be filled"
    );
    assert_eq!(set.slots()[0].filename, None, "1x well untouched");

    assert!(source.is_file());
}

#[test]
fn the_1x_slot_has_no_suffix() {
    let tmp = TempDir::new("onex");
    let source = tmp.path().join("src.png");
    write_png(&source);

    let mut catalog = Catalog::open(tmp.path().join("Media.xcassets")).unwrap();
    catalog.create_set("icon").unwrap();
    catalog.set_slot("icon", Scale::One, &source).unwrap();

    assert_eq!(
        catalog.get("icon").unwrap().slots()[0].filename.as_deref(),
        Some("icon.png")
    );
}

#[test]
fn clearing_a_slot_removes_the_file_but_keeps_the_well() {
    let tmp = TempDir::new("clear");
    let source = tmp.path().join("src.png");
    write_png(&source);

    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("logo").unwrap();
    catalog.set_slot("logo", Scale::Two, &source).unwrap();
    catalog.clear_slot("logo", Scale::Two).unwrap();

    assert!(!root.join("logo.imageset/logo@2x.png").exists());

    let set = catalog.get("logo").unwrap();
    assert_eq!(set.slots().len(), 3);
    assert!(set.is_empty());
}

#[test]
fn renaming_carries_the_files_along() {
    let tmp = TempDir::new("rename");
    let source = tmp.path().join("src.png");
    write_png(&source);

    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("old").unwrap();
    catalog.set_slot("old", Scale::Two, &source).unwrap();
    catalog.rename_set("old", "new").unwrap();

    assert!(!root.join("old.imageset").exists());
    assert!(root.join("new.imageset/new@2x.png").is_file());
    assert_eq!(
        catalog.get("new").unwrap().slots()[1].filename.as_deref(),
        Some("new@2x.png"),
        "Contents.json must point at the renamed file, not the old one"
    );
}

#[test]
fn deleting_removes_the_whole_set() {
    let tmp = TempDir::new("delete");
    let mut catalog = Catalog::open(tmp.path()).unwrap();
    catalog.create_set("logo").unwrap();
    catalog.delete_set("logo").unwrap();

    assert!(catalog.sets.is_empty());
    assert!(!tmp.path().join("logo.imageset").exists());
}

#[test]
fn sets_nested_in_groups_are_still_found() {
    let tmp = TempDir::new("groups");
    let root = tmp.path().join("Media.xcassets");

    let nested = root.join("Icons/Small/buried.imageset");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("Contents.json"),
        r#"{"images":[{"filename":"buried.png","idiom":"universal","scale":"1x"}],"info":{"author":"xcode","version":1}}"#,
    )
    .unwrap();
    write_png(&nested.join("buried.png"));

    let catalog = Catalog::open(&root).unwrap();
    assert_eq!(catalog.sets.len(), 1);
    assert_eq!(catalog.sets[0].name, "buried");
}

#[test]
fn appicon_and_color_sets_are_left_alone() {
    let tmp = TempDir::new("otherkinds");
    let root = tmp.path().join("Media.xcassets");
    fs::create_dir_all(root.join("AppIcon.appiconset")).unwrap();
    fs::create_dir_all(root.join("AccentColor.colorset")).unwrap();

    let catalog = Catalog::open(&root).unwrap();
    assert!(
        catalog.sets.is_empty(),
        "only image sets are this editor's business"
    );

    assert!(root.join("AppIcon.appiconset").exists());
}

#[test]
fn flatten_emits_names_the_swift_loader_resolves() {
    let tmp = TempDir::new("flatten");
    let source = tmp.path().join("src.png");
    write_png(&source);

    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("logo").unwrap();
    catalog.set_slot("logo", Scale::Two, &source).unwrap();
    catalog.set_slot("logo", Scale::Three, &source).unwrap();

    let out = tmp.path().join("Assets");
    let report = flatten(&root, &out).unwrap();

    assert_eq!(report.written, vec!["logo@2x.png", "logo@3x.png"]);
    assert!(out.join("logo@3x.png").is_file());
    assert!(!out.join("logo@1x.png").exists(), "1x was never filled");
}

#[test]
fn flatten_clears_stale_output_but_spares_everything_else() {
    let tmp = TempDir::new("stale");
    let source = tmp.path().join("src.png");
    write_png(&source);

    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("keep").unwrap();
    catalog.set_slot("keep", Scale::One, &source).unwrap();

    let out = tmp.path().join("Assets");
    fs::create_dir_all(&out).unwrap();

    write_png(&out.join("deleted@2x.png"));

    fs::write(out.join("notes.txt"), b"hands off").unwrap();

    let report = flatten(&root, &out).unwrap();

    assert!(!out.join("deleted@2x.png").exists());
    assert_eq!(report.removed, vec!["deleted@2x.png"]);
    assert!(out.join("keep@1x.png").is_file());
    assert!(
        out.join("notes.txt").is_file(),
        "non-image files are none of flatten's business"
    );
}

#[test]
fn flatten_handles_a_real_xcode_project_catalogue() {
    let tmp = TempDir::new("xcodeshape");
    let root = tmp.path().join("Assets.xcassets");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Contents.json"),
        r#"{"info":{"author":"xcode","version":1}}"#,
    )
    .unwrap();

    for stock in ["AppIcon.appiconset", "AccentColor.colorset"] {
        let dir = root.join(stock);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Contents.json"),
            r#"{"info":{"author":"xcode","version":1}}"#,
        )
        .unwrap();
    }

    let set = root.join("logo.imageset");
    fs::create_dir_all(&set).unwrap();
    fs::write(
        set.join("Contents.json"),
        r#"{"images":[{"filename":"my-photo@2x.png","idiom":"universal","scale":"2x"}],
            "info":{"author":"xcode","version":1}}"#,
    )
    .unwrap();
    write_png(&set.join("my-photo@2x.png"));

    let out = tmp.path().join("SwiftFlowApp.app/Assets");
    let report = flatten(&root, &out).unwrap();

    assert_eq!(report.written, vec!["logo@2x.png"]);
    assert!(out.join("logo@2x.png").is_file());
    assert!(
        report.empty.is_empty() && report.conflicts.is_empty(),
        "the stock AppIcon/AccentColor sets must not register as image sets"
    );
}

#[test]
fn flatten_survives_metadata_pointing_at_a_missing_file() {
    let tmp = TempDir::new("missingfile");
    let root = tmp.path().join("Media.xcassets");
    let set = root.join("logo.imageset");
    fs::create_dir_all(&set).unwrap();

    fs::write(
        set.join("Contents.json"),
        r#"{"images":[
            {"filename":"logo@2x.png","idiom":"universal","scale":"2x"},
            {"filename":"logo@3x.png","idiom":"universal","scale":"3x"}
        ],"info":{"author":"xcode","version":1}}"#,
    )
    .unwrap();
    write_png(&set.join("logo@2x.png"));

    let out = tmp.path().join("Assets");
    let report = flatten(&root, &out).expect("a missing file is a warning, not a failure");

    assert_eq!(report.written, vec!["logo@2x.png"]);
    assert_eq!(report.missing, vec!["logo (3x)"]);
    assert!(out.join("logo@2x.png").is_file());
}

#[test]
fn flatten_reports_empty_sets_rather_than_failing() {
    let tmp = TempDir::new("emptyset");
    let root = tmp.path().join("Media.xcassets");
    let mut catalog = Catalog::open(&root).unwrap();
    catalog.create_set("placeholder").unwrap();

    let report = flatten(&root, tmp.path().join("Assets")).unwrap();
    assert_eq!(report.empty, vec!["placeholder"]);
    assert!(report.written.is_empty());
}

#[test]
fn flatten_handles_a_single_scale_image_set() {
    let tmp = TempDir::new("singlescale");
    let root = tmp.path().join("Assets.xcassets");
    let set = root.join("book_spine.imageset");
    fs::create_dir_all(&set).unwrap();
    fs::write(
        set.join("Contents.json"),
        r#"{
          "images" : [
            {
              "filename" : "book_spine.png",
              "idiom" : "universal"
            }
          ],
          "info" : { "author" : "xcode", "version" : 1 }
        }"#,
    )
    .unwrap();
    write_png(&set.join("book_spine.png"));

    let out = tmp.path().join("Assets");
    let report = flatten(&root, &out).unwrap();

    assert_eq!(report.written, vec!["book_spine.png"]);
    assert!(out.join("book_spine.png").is_file());
    assert!(
        report.empty.is_empty(),
        "a single-scale set has an image; reporting it empty is what dropped it"
    );
}
