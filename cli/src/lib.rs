pub mod build;
pub mod config;
pub mod gradle;
pub mod proc;
pub mod xtool;

use std::fmt;
use std::path::{Path, PathBuf};

pub fn home(env: &dyn Env) -> PathBuf {
    if let Some(explicit) = env.var("SWIFTFLOW_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(explicit);
    }
    match env.var("HOME").filter(|v| !v.is_empty()) {
        Some(h) => PathBuf::from(h).join(".swiftflow"),

        None => PathBuf::from(".swiftflow"),
    }
}

pub fn framework_root(home: &Path, pin: Option<&str>) -> PathBuf {
    match pin.map(str::trim).filter(|p| !p.is_empty()) {
        Some(version) => home.join("versions").join(version),
        None => home.join("current"),
    }
}

pub fn framework_version(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join("VERSION"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn rust_cache(home: &Path, version: &str) -> PathBuf {
    home.join("cache").join("rust").join(version)
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,

    pub name: String,

    pub target: String,

    pub pin: Option<String>,

    pub config: crate::config::Config,
}

pub fn find_project(start: &Path) -> Option<Project> {
    let mut dir = Some(start);
    while let Some(current) = dir {
        if current.join("Package.swift").is_file() {
            let name = current
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "App".to_string());

            let config = match crate::config::Config::load(current) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            };
            let pin = config.pin(current);
            let target = swift_target(current, &name);
            return Some(Project {
                root: current.to_path_buf(),
                name,
                target,
                pin,
                config,
            });
        }
        dir = current.parent();
    }
    None
}

fn swift_target(root: &Path, dir_name: &str) -> String {
    let expected = swift_identifier(dir_name);
    if root.join("Sources").join(&expected).is_dir() {
        return expected;
    }
    let mut dirs = std::fs::read_dir(root.join("Sources"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned());
    match (dirs.next(), dirs.next()) {

        (Some(only), None) => only,
        _ => expected,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Desktop,
    Android,
}

impl Platform {

    pub fn env_value(self) -> &'static str {
        match self {
            Platform::Ios => "ios",
            Platform::Desktop => "desktop",
            Platform::Android => "android",
        }
    }

    pub fn default_for_host() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Ios
        } else {
            Platform::Desktop
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.env_value())
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ios" | "iphone" => Ok(Platform::Ios),
            "desktop" | "mac" | "macos" | "linux" => Ok(Platform::Desktop),
            "android" => Ok(Platform::Android),
            other => Err(format!(
                "unknown platform {other:?} — expected ios, desktop or android"
            )),
        }
    }
}

pub trait Env {
    fn var(&self, key: &str) -> Option<String>;
}

pub struct RealEnv;

impl Env for RealEnv {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

pub struct ScaffoldFile {
    pub path: String,
    pub contents: String,
    pub executable: bool,
}

pub fn swift_identifier(name: &str) -> String {
    let mut out = String::new();
    let mut capitalise = true;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if capitalise {
                out.extend(ch.to_uppercase());
                capitalise = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalise = true;
        }
    }

    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'A');
    }
    if out.is_empty() {
        out.push_str("App");
    }
    out
}

pub fn scaffold(name: &str, pin: Option<&str>) -> Vec<ScaffoldFile> {
    let target = swift_identifier(name);
    let mut files = vec![
        ScaffoldFile {
            path: "Package.swift".into(),
            contents: package_manifest(&target),
            executable: false,
        },
        ScaffoldFile {
            path: format!("Sources/{target}/{target}App.swift"),
            contents: app_source(&target),
            executable: false,
        },
        ScaffoldFile {
            path: format!("Sources/{target}/ContentView.swift"),
            contents: CONTENT_VIEW.to_string(),
            executable: false,
        },
        ScaffoldFile {
            path: format!("Sources/{target}/Assets/.gitkeep"),
            contents: String::new(),
            executable: false,
        },
        ScaffoldFile {
            path: ".gitignore".into(),
            contents: GITIGNORE.to_string(),
            executable: false,
        },
    ];
    files.push(ScaffoldFile {
        path: crate::config::FILE_NAME.into(),
        contents: swiftflow_toml(name, &target, pin),
        executable: false,
    });
    files
}

fn swiftflow_toml(name: &str, target: &str, pin: Option<&str>) -> String {
    let pin = pin.unwrap_or("current");
    let id = format!("com.swiftflow.{}", target.to_ascii_lowercase());
    format!(
        r#"# One file describing this app on every platform. Canonical values
# are lowered into each platform's own manifest at build time.
#
# Precedence, when a key is expressible in more than one place:
#   raw passthrough  >  [platform] typed  >  [app] canonical

[swiftflow]
# Which SwiftFlow to build against: a version, or `dev` for a working
# tree. Remove the key to follow whatever `current` points at.
version = "{pin}"

[app]
id      = "{id}"
name    = "{name}"
version = "0.1.0"
# Monotonic integer. A store or a device refuses an install that does not
# increase it, which is why it is separate from the version above.
build   = 1
# portrait | landscape | any
# orientation = "portrait"

# Capabilities are canonical names the CLI knows how to lower: iOS gets a
# reason string, Android gets the permission constants. `swiftflow doctor`
# lists the vocabulary. Anything outside it goes in the raw sections below.
#
# [capabilities.camera]
# reason = "Scan pages into the reader."

# [ios]
# deployment_target = "16.0"

# [ios.plist]                    # merged verbatim into Info.plist, wins last
# UIStatusBarStyle = "UIStatusBarStyleLightContent"

# [android]
# min_sdk = 28                   # also picks the Swift SDK's target triple
# target_sdk = 34

# [android.manifest]             # merged onto <application>, wins last
# "android:hardwareAccelerated" = true
"#
    )
}

fn package_manifest(target: &str) -> String {
    format!(

        r##"// swift-tools-version: 6.0
import PackageDescription
import Foundation

// One app, three hosts.
//
// The sources are platform-agnostic — `import SwiftFlow` and
// `@main struct App: SwiftFlowApp` are identical everywhere. What can't
// be shared is the *shape of the package*: iOS wants a library product,
// desktop an executable, Android a shared library NativeActivity can
// dlopen. So the manifest branches rather than the app existing three
// times.
let platform = ProcessInfo.processInfo.environment["SWIFTFLOW_PLATFORM"] ?? "ios"
let desktop = platform == "desktop"
let android = platform == "android"

// ── Finding SwiftFlow ─────────────────────────────────────────────────
// The framework is installed in ~/.swiftflow rather than vendored here.
// What this project pins is a version, in a one-line `.swiftflow-version`
// beside this file:
//
//   0.1.0   an installed release
//   dev     a symlink to a framework working tree
//   (none)  whatever `current` points at
//
// Resolved here rather than by a generator because a manifest runs on the
// host and can read the filesystem — so `swift build` works on a fresh
// clone with no generate step, and the pin is a file you can diff instead
// of an absolute path baked into a manifest.
let projectDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let swiftflowHome = ProcessInfo.processInfo.environment["SWIFTFLOW_HOME"]
    ?? "\(NSHomeDirectory())/.swiftflow"

// Scanned for, not parsed. A manifest has no TOML parser and no way to
// add a dependency on one — but it needs exactly one key out of this
// file, so it looks for `version` under `[swiftflow]` and ignores
// everything else. The CLI reads the file properly.
let pin: String? = {{
    guard let text = try? String(contentsOfFile: "\(projectDir)/SwiftFlow.toml", encoding: .utf8)
    else {{
        // The file this replaced, so a project mid-migration still builds.
        return (try? String(contentsOfFile: "\(projectDir)/.swiftflow-version", encoding: .utf8))?
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }}
    var inSwiftFlowSection = false
    for rawLine in text.split(separator: "\n", omittingEmptySubsequences: false) {{
        var line = String(rawLine)
        if let hash = line.firstIndex(of: "#") {{ line = String(line[..<hash]) }}
        line = line.trimmingCharacters(in: .whitespaces)
        if line.hasPrefix("[") {{
            inSwiftFlowSection = line == "[swiftflow]"
            continue
        }}
        guard inSwiftFlowSection, let equals = line.firstIndex(of: "=") else {{ continue }}
        guard line[..<equals].trimmingCharacters(in: .whitespaces) == "version" else {{ continue }}
        return String(line[line.index(after: equals)...])
            .trimmingCharacters(in: .whitespaces)
            .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
    }}
    return nil
}}()

let swiftflowRoot = (pin?.isEmpty == false)
    ? "\(swiftflowHome)/versions/\(pin!)"
    : "\(swiftflowHome)/current"

let hostPath = android
    ? "\(swiftflowRoot)/android"
    : (desktop ? "\(swiftflowRoot)/desktop" : "\(swiftflowRoot)/apple")
let hostName = android ? "SwiftFlowAndroid" : (desktop ? "SwiftFlowDesktop" : "SwiftFlowApple")

// nil rather than an empty list on Android: SwiftPM 6 has no `.android`
// SupportedPlatform, and stating an iOS or macOS minimum for a build that
// is neither only invites the resolver to check it.
let platforms: [SupportedPlatform]? =
    android ? nil : (desktop ? [.macOS(.v14)] : [.iOS(.v17)])

let product: Product = {{
    if android {{
        return .library(
            name: "{target}",
            type: .dynamic,
            targets: ["{target}", "{target}AndroidEntry"]
        )
    }}
    return desktop
        ? .executable(name: "{target}", targets: ["{target}"])
        : .library(name: "{target}", targets: ["{target}"])
}}()

let sources = ["{target}App.swift", "ContentView.swift"]

// Android starts in native code and calls up into Swift, so it needs a C
// entry point `@main` can't provide. `swiftflow build` generates one into
// .build/swiftflow-entry and it is compiled as its own target, so nothing
// platform-conditional lives in Sources/.
let entryTarget: Target = .target(
    name: "{target}AndroidEntry",
    dependencies: ["{target}", .product(name: "SwiftFlow", package: hostName)],
    path: ".build/swiftflow-entry"
)

// `.executableTarget` on desktop, `.target` everywhere else.
//
// SwiftPM only infers that a target is executable from a file literally
// named `main.swift`. This app's entry point is `@main struct {target}App`
// in {target}App.swift, so pairing the executable product above with a
// plain `.target` fails outright:
//
//   error: executable product '{target}' expects target '{target}' to be
//   executable; an executable target requires a 'main.swift' file
//
// iOS and Android build *library* products — a dynamic one on Android for
// NativeActivity to dlopen — where `.executableTarget` would be equally
// wrong. Hence the branch rather than picking one.
let appTarget: Target = {{
    let deps: [Target.Dependency] = [.product(name: "SwiftFlow", package: hostName)]
    // Sources are listed explicitly because Assets/ lives inside the
    // target directory and is a resource, not a source.
    if desktop {{
        return .executableTarget(
            name: "{target}",
            dependencies: deps,
            path: "Sources/{target}",
            sources: sources,
            resources: [.copy("Assets")]
        )
    }}
    return .target(
        name: "{target}",
        dependencies: deps,
        path: "Sources/{target}",
        sources: sources,
        resources: [.copy("Assets")]
    )
}}()

let package = Package(
    name: "{target}",
    platforms: platforms,
    products: [product],
    dependencies: [
        .package(name: hostName, path: hostPath)
    ],
    targets: android ? [appTarget, entryTarget] : [appTarget]
)
"##
    )
}

fn app_source(target: &str) -> String {
    format!(
        r#"import SwiftFlow

@main
struct {target}App: SwiftFlowApp {{
    var body: some Scene {{
        WindowGroup {{
            ContentView()
        }}
    }}
}}
"#
    )
}


const CONTENT_VIEW: &str = r#"import SwiftFlow

struct ContentView: View {
    @State private var count = 0

    var body: some View {
        VStack(spacing: 16) {
            Icon.handWaving.size(48).foregroundColor(.accent)

            Text("Hello, SwiftFlow")
                .font(.title2)
                .fontWeight(.bold)

            Text("Tapped \(count) times")
                .font(.subheadline)
                .foregroundColor(.secondary)

            Button("Tap me") {
                withAnimation(.spring()) { count += 1 }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
        .navigationTitle("Home")
    }
}
"#;

const GITIGNORE: &str = r#".build/
.swiftpm/
xtool/
xtool.yml
*.ipa
.DS_Store
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeEnv(HashMap<String, String>);

    impl FakeEnv {
        fn new(pairs: &[(&str, &str)]) -> Self {
            FakeEnv(
                pairs
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            )
        }
    }

    impl Env for FakeEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    #[test]
    fn home_prefers_an_explicit_override() {
        let env = FakeEnv::new(&[("SWIFTFLOW_HOME", "/opt/sf"), ("HOME", "/home/x")]);
        assert_eq!(home(&env), PathBuf::from("/opt/sf"));
    }

    #[test]
    fn home_falls_back_to_the_user() {
        let env = FakeEnv::new(&[("HOME", "/home/x")]);
        assert_eq!(home(&env), PathBuf::from("/home/x/.swiftflow"));
    }

    #[test]
    fn an_empty_override_is_not_an_override() {

        let env = FakeEnv::new(&[("SWIFTFLOW_HOME", ""), ("HOME", "/home/x")]);
        assert_eq!(home(&env), PathBuf::from("/home/x/.swiftflow"));
    }

    #[test]
    fn a_pin_selects_a_version_and_no_pin_selects_current() {
        let home = Path::new("/h/.swiftflow");
        assert_eq!(
            framework_root(home, Some("0.1.0")),
            PathBuf::from("/h/.swiftflow/versions/0.1.0")
        );
        assert_eq!(
            framework_root(home, Some("dev")),
            PathBuf::from("/h/.swiftflow/versions/dev")
        );
        assert_eq!(
            framework_root(home, None),
            PathBuf::from("/h/.swiftflow/current")
        );

        assert_eq!(
            framework_root(home, Some("  \n")),
            PathBuf::from("/h/.swiftflow/current")
        );
    }

    #[test]
    fn the_cache_is_keyed_by_version_not_by_project() {

        let home = Path::new("/h/.swiftflow");
        assert_eq!(
            rust_cache(home, "0.1.0"),
            PathBuf::from("/h/.swiftflow/cache/rust/0.1.0")
        );
        assert_ne!(rust_cache(home, "0.1.0"), rust_cache(home, "0.2.0"));
    }

    #[test]
    fn platforms_round_trip_through_their_env_spelling() {
        for platform in [Platform::Ios, Platform::Desktop, Platform::Android] {
            assert_eq!(platform.env_value().parse::<Platform>().unwrap(), platform);
        }
    }

    #[test]
    fn an_unknown_platform_says_what_it_expected() {
        let error = "windows".parse::<Platform>().unwrap_err();
        assert!(error.contains("ios"), "unhelpful: {error}");
    }

    #[test]
    fn directory_names_become_swift_identifiers() {
        assert_eq!(swift_identifier("myapp"), "Myapp");
        assert_eq!(swift_identifier("my-cool-app"), "MyCoolApp");
        assert_eq!(swift_identifier("my_cool app"), "MyCoolApp");
        assert_eq!(swift_identifier("Reader"), "Reader");

        assert_eq!(swift_identifier("2048"), "A2048");
        assert_eq!(swift_identifier("---"), "App");
    }

    #[test]
    fn the_scaffold_names_every_file_after_the_target() {
        let files = scaffold("my-cool-app", Some("dev"));
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"Sources/MyCoolApp/MyCoolAppApp.swift"));
        assert!(paths.contains(&"Sources/MyCoolApp/ContentView.swift"));
        assert!(paths.contains(&"SwiftFlow.toml"));

        let manifest = &files.iter().find(|f| f.path == "Package.swift").unwrap().contents;
        assert!(manifest.contains(r#"name: "MyCoolApp""#));

        assert!(manifest.contains("SWIFTFLOW_HOME"));
        assert!(manifest.contains(".swiftflow-version"));
        assert!(manifest.contains("versions/"));
    }

    /// file literally named `main.swift`, and these apps use `@main` in

    #[test]
    fn the_manifest_pairs_an_executable_product_with_an_executable_target() {
        let files = scaffold("my-cool-app", Some("dev"));
        let manifest = &files.iter().find(|f| f.path == "Package.swift").unwrap().contents;

        assert!(
            manifest.contains(".executable(name:"),
            "desktop should still build an executable product"
        );
        assert!(
            manifest.contains(".executableTarget("),
            "an executable product needs an executable target"
        );

        assert!(
            manifest.contains(".library(name:"),
            "iOS/Android still build libraries"
        );
        assert!(
            manifest.contains(".target("),
            "the non-desktop branch still needs a plain target"
        );

        assert!(
            manifest.contains("if desktop {"),
            "the target kind must branch on the same condition as the product"
        );
    }

    #[test]
    fn a_project_without_an_installed_framework_still_gets_a_config() {

        let files = scaffold("app", None);
        let config = &files
            .iter()
            .find(|f| f.path == crate::config::FILE_NAME)
            .expect("a config is always written")
            .contents;
        assert!(config.contains(r#"version = "current""#));

        let parsed: crate::config::Config =
            toml::from_str(config).expect("the scaffold must be valid TOML");
        assert_eq!(parsed.app.name.as_deref(), Some("app"));
    }

    #[test]
    fn the_scaffolded_config_round_trips_through_the_parser() {
        let files = scaffold("my-cool-app", Some("dev"));
        let config = &files
            .iter()
            .find(|f| f.path == crate::config::FILE_NAME)
            .unwrap()
            .contents;
        let parsed: crate::config::Config = toml::from_str(config).unwrap();
        assert_eq!(parsed.swiftflow.version.as_deref(), Some("dev"));
        assert_eq!(parsed.app.id.as_deref(), Some("com.swiftflow.mycoolapp"));
    }

    #[test]
    fn find_project_walks_up_from_a_subdirectory() {
        let tmp = std::env::temp_dir().join(format!("sf-cli-{}", std::process::id()));
        let deep = tmp.join("Sources").join("App");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(tmp.join("Package.swift"), "// manifest").unwrap();
        std::fs::write(tmp.join(".swiftflow-version"), "dev\n").unwrap();

        let found = find_project(&deep).expect("should have walked up");
        assert_eq!(found.root, tmp);
        assert_eq!(found.pin.as_deref(), Some("dev"));

        assert_eq!(found.target, "App");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn find_project_gives_up_rather_than_guessing() {
        let tmp = std::env::temp_dir().join(format!("sf-cli-empty-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(find_project(&tmp).is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
