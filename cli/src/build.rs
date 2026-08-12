use std::path::{Path, PathBuf};

use crate::gradle::GradleProject;
use crate::proc::{exists, find, Run};
use crate::{Platform, Project};

pub struct Build {
    pub project: Project,
    pub framework: PathBuf,
    pub home: PathBuf,
    pub platform: Platform,

    pub run: bool,
}

impl Build {
    fn rust_dir(&self) -> PathBuf {
        self.framework.join("rust")
    }

    fn target_dir(&self) -> PathBuf {
        self.rust_dir().join("target")
    }

    fn app_name(&self) -> &str {
        &self.project.target
    }

    fn lib_dir(&self, triple: &str) -> PathBuf {
        self.target_dir().join(triple).join("release")
    }

    pub fn go(&self) -> Result<(), String> {
        match self.platform {
            Platform::Desktop => self.desktop(),
            Platform::Ios => self.ios(),
            Platform::Android => self.android(),
        }
    }

    fn build_rust(&self, package: &str, triple: &str) -> Result<PathBuf, String> {
        self.build_rust_with_api(package, triple, None)
    }

    fn build_rust_with_api(
        &self,
        package: &str,
        triple: &str,
        api: Option<&str>,
    ) -> Result<PathBuf, String> {
        println!("▶ Building Rust for {triple}...");
        let rust_dir = self.rust_dir();
        let mut cargo = Run::new("cargo");

        if let Some(api) = api {
            match ndk_clangxx(triple, api) {
                Some(compiler) => {
                    let underscored = triple.replace('-', "_");
                    cargo = cargo
                        .env(format!("CXX_{underscored}"), &compiler)
                        .env(format!("CXX_{triple}"), &compiler);
                }
                None => {
                    return Err(format!(
                        "No C++ compiler for {triple} in the NDK.\n  \
                         GameActivity's glue is C++, so the Android build needs one.\n  \
                         Looked for aarch64/x86_64-linux-android{api}-clang++ under\n  \
                         $ANDROID_NDK_HOME/toolchains/llvm/prebuilt.\n  \
                         Set ANDROID_NDK_HOME to an NDK r27d or later."
                    ))
                }
            }
        }
        cargo
            .arg("build")
            .arg("--target-dir")
            .arg(self.target_dir())
            .arg("--target")
            .arg(triple)
            .arg("--release")
            .arg("--manifest-path")
            .arg(rust_dir.join("Cargo.toml"))
            .arg("-p")
            .arg(package)
            .current_dir(&rust_dir)
            .run()?;

        let lib_dir = self.lib_dir(triple);
        let archive = lib_dir.join(format!("lib{package}.a"));
        if !archive.is_file() {
            return Err(format!(
                "the Rust build produced no static library at {}",
                archive.display()
            ));
        }
        self.check_exports(&archive, package)?;
        println!("✓ Rust build complete");
        Ok(lib_dir)
    }

    fn check_exports(&self, archive: &Path, package: &str) -> Result<(), String> {
        let header = self.framework.join("Sources/CSwiftFlow/SwiftFlowMetal.h");
        let source = match std::fs::read_to_string(&header) {
            Ok(source) => source,
            Err(_) => return Ok(()),
        };
        let expected = declared_symbols(&source, package);
        if expected.is_empty() {
            return Ok(());
        }

        let listing = match Run::new("nm").arg("-g").arg(archive).capture() {
            Ok(listing) => listing,
            Err(_) => return Ok(()),
        };
        let missing: Vec<&String> = expected
            .iter()
            .filter(|symbol| !defines(&listing, symbol))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }

        Err(format!(
            "{} declares {} that {} does not define:\n  {}\n\n  \
             The C header and the Rust archive have drifted apart. Usually \
             the archive is\n  stale — cargo reused output from before \
             those functions existed. Delete it\n  and build again:\n\n    \
             rm -rf {}\n",
            header.display(),
            if missing.len() == 1 {
                "a function"
            } else {
                "functions"
            },
            archive.display(),
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n  "),
            self.target_dir().display(),
        ))
    }

    fn flatten_assets(&self) -> Result<Option<PathBuf>, String> {
        let catalogue = self.project.root.join("Assets.xcassets");
        if !catalogue.is_dir() {
            return Ok(None);
        }

        let tools = self.framework.join("tools");
        let binary = tools.join("target/release/sf-assets");
        if !binary.is_file() {
            println!("▶ Building sf-assets (first run)...");
            Run::new("cargo")
                .args([
                    "build",
                    "--release",
                    "-p",
                    "swiftflow_assets",
                    "--bin",
                    "sf-assets",
                ])
                .current_dir(&tools)
                .run()?;
        }

        let out = self
            .project
            .root
            .join("Sources")
            .join(self.app_name())
            .join("Assets");
        println!("▶ Flattening assets...");
        Run::new(&binary)
            .arg("flatten")
            .arg(&catalogue)
            .arg(&out)
            .run()?;
        Ok(Some(out))
    }

    fn swift_env(&self, command: Run, triple: &str, lib_dir: &Path) -> Run {
        command
            .env("SWIFTFLOW_PLATFORM", self.platform.env_value())
            .env("SWIFTFLOW_RUST_TRIPLE", triple)
            .env("SWIFTFLOW_RUST_LIB_DIR", lib_dir)
            .env("SWIFTFLOW_HOME", &self.home)
    }

    fn force_relink(&self, archive: &Path) -> Result<(), String> {
        let modified = std::fs::metadata(archive)
            .and_then(|m| m.modified())
            .map_err(|e| format!("{}: {e}", archive.display()))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("{}: {e}", archive.display()))?
            .as_nanos()
            .to_string();

        let stamp = self.project.root.join(".build/swiftflow-rust-stamp");
        if std::fs::read_to_string(&stamp).ok().as_deref() == Some(modified.as_str()) {
            return Ok(());
        }

        touch_swift_sources(&self.project.root.join("Sources").join(self.app_name()))?;
        if let Some(dir) = stamp.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&stamp, modified).map_err(|e| format!("{}: {e}", stamp.display()))
    }

    fn desktop(&self) -> Result<(), String> {
        let triple = host_triple()?;
        let lib_dir = self.build_rust("swiftflow_desktop", &triple)?;
        self.flatten_assets()?;
        self.force_relink(&lib_dir.join("libswiftflow_desktop.a"))?;

        println!("▶ Building and running the app...");

        let verb = if self.run { "run" } else { "build" };
        self.swift_env(Run::new("swift"), &triple, &lib_dir)
            .arg(verb)
            .current_dir(&self.project.root)
            .run()
    }

    fn ios(&self) -> Result<(), String> {
        const TRIPLE: &str = "aarch64-apple-ios";

        let lib_dir = self.build_rust("swiftflow_wgpu", TRIPLE)?;
        self.flatten_assets()?;
        let _xtool =
            crate::xtool::scoped(&self.project.root, self.app_name(), &self.project.config)?;

        if !self.run {
            println!("▶ Building the app...");
            return self
                .swift_env(Run::new("swift"), TRIPLE, &lib_dir)
                .args(["build", "-c", "release"])
                .current_dir(&self.project.root)
                .run();
        }

        println!("▶ Building and running with xtool...");
        self.swift_env(Run::new("xtool"), TRIPLE, &lib_dir)
            .arg("dev")
            .current_dir(&self.project.root)
            .run()?;

        if let Some(target) = std::env::var("SWIFTFLOW_IOS_LAUNCH_ID")
            .ok()
            .filter(|v| !v.is_empty())
        {
            if exists("pymobiledevice3") {
                Run::new("pymobiledevice3")
                    .args(["developer", "dvt", "launch", "--stream", &target])
                    .run()?;
            }
        }
        Ok(())
    }

    fn android(&self) -> Result<(), String> {
        let abi = env_or("SWIFTFLOW_ANDROID_ABI", "arm64-v8a");

        let api = match std::env::var("SWIFTFLOW_ANDROID_API")
            .ok()
            .filter(|v| !v.is_empty())
        {
            Some(explicit) => explicit,
            None => self
                .project
                .config
                .android
                .min_sdk
                .map(|v| v.to_string())
                .unwrap_or_else(|| "28".to_string()),
        };

        let (rust_triple, swift_sdk) = match abi.as_str() {
            "arm64-v8a" => (
                "aarch64-linux-android".to_string(),
                format!("aarch64-unknown-linux-android{api}"),
            ),
            "x86_64" => (
                "x86_64-linux-android".to_string(),
                format!("x86_64-unknown-linux-android{api}"),
            ),
            other => {
                return Err(format!(
                    "unsupported ABI {other:?} — expected arm64-v8a or x86_64"
                ))
            }
        };

        if abi == "x86_64" {
            println!("ℹ x86_64 build — for an emulator. SwiftFlow needs Vulkan, so use an");
            println!("  API 33+ system image with hardware acceleration; a software-GL AVD");
            println!("  will start and render nothing.");
        }

        let gradle_dir = self.gradle_project(&api)?;
        check_android_sdk(&gradle_dir)?;
        check_swift_sdk(&swift_sdk)?;

        let lib_dir = self.build_rust_with_api("swiftflow_android", &rust_triple, Some(&api))?;

        if let Some(assets) = self.flatten_assets()? {
            let apk_assets = gradle_dir.join("app/src/main/assets");
            std::fs::create_dir_all(&apk_assets)
                .map_err(|e| format!("{}: {e}", apk_assets.display()))?;

            for entry in std::fs::read_dir(&assets).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        std::fs::copy(&path, apk_assets.join(name))
                            .map_err(|e| format!("copying {}: {e}", path.display()))?;
                    }
                }
            }
        }

        self.write_android_entry()?;

        let so = self.android_swift(&abi, &swift_sdk, &rust_triple, &lib_dir)?;
        self.android_package(&gradle_dir, &abi, &rust_triple, &so)
    }

    fn write_android_entry(&self) -> Result<(), String> {
        let sources = self.project.root.join("Sources").join(self.app_name());
        let app = main_type(&sources).ok_or_else(|| {
            format!(
                "no `@main` type found under {}.\n  \
                 Android starts in native code and calls up into Swift, so the \
                 build has to\n  know which type to start.",
                sources.display()
            )
        })?;

        let dir = self.project.root.join(".build").join("swiftflow-entry");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let body = format!(
            "import SwiftFlow\n\n@_cdecl(\"{ANDROID_ENTRY}\")\npublic func {ANDROID_ENTRY}() {{\n    {app}.main()\n}}\n"
        );
        let path = dir.join("AndroidEntry.swift");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(body.as_str()) {
            std::fs::write(&path, &body).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    }

    fn gradle_project(&self, api: &str) -> Result<PathBuf, String> {
        let owned = self.project.root.join("android");
        if owned.join("settings.gradle.kts").is_file() {
            return Ok(owned);
        }
        let generated = self.project.root.join(".build").join("android");
        GradleProject::resolve(
            &self.project.root,
            self.app_name(),
            api,
            &self.project.config,
        )
        .write(&generated)?;
        Ok(generated)
    }

    fn android_swift(
        &self,
        abi: &str,
        swift_sdk: &str,
        rust_triple: &str,
        lib_dir: &Path,
    ) -> Result<PathBuf, String> {
        println!("▶ Building Swift for {swift_sdk}...");

        let sources = self.project.root.join("Sources").join(self.app_name());
        touch_swift_sources(&sources)?;

        let scratch = self.project.root.join(".build").join(abi);

        let mut common: Vec<String> = vec![
            "--swift-sdk".into(),
            swift_sdk.into(),
            "-c".into(),
            "release".into(),
            "--scratch-path".into(),
            scratch.to_string_lossy().into_owned(),
            "--static-swift-stdlib".into(),
        ];

        let swift_arch = rust_triple.split('-').next().unwrap_or("aarch64");
        match static_resource_dir(swift_arch) {
            Some(dir) => {
                common.push("-Xlinker".into());
                common.push(format!("-L{}", dir.display()));
            }
            None => {
                println!("⚠ No swift_static-{swift_arch}/android directory in the SDK.");
                println!("  Anything using URLSession will fail to link.");
            }
        }

        let build_args: Vec<String> = std::iter::once("build".to_string())
            .chain(common.iter().cloned())
            .chain(
                ["-Xlinker", "-u", "-Xlinker", "ANativeActivity_onCreate"]
                    .iter()
                    .map(|s| s.to_string()),
            )
            .collect();

        self.swift_env(Run::new("swift"), rust_triple, lib_dir)
            .args(&build_args)
            .current_dir(&self.project.root)
            .run()?;

        let show_args: Vec<String> = std::iter::once("build".to_string())
            .chain(common.iter().cloned())
            .chain(std::iter::once("--show-bin-path".to_string()))
            .collect();
        let bin_path = self
            .swift_env(Run::new("swift"), rust_triple, lib_dir)
            .args(&show_args)
            .current_dir(&self.project.root)
            .capture()?;

        let so = PathBuf::from(bin_path).join(format!("lib{}.so", self.app_name()));
        if !so.is_file() {
            return Err(format!(
                "the Swift build produced no shared library at {}",
                so.display()
            ));
        }
        println!("✓ Swift build complete");
        Ok(so)
    }

    fn android_package(
        &self,
        gradle_dir: &Path,
        abi: &str,
        rust_triple: &str,
        so: &Path,
    ) -> Result<(), String> {
        let jni_dir = gradle_dir.join("app/src/main/jniLibs").join(abi);
        std::fs::create_dir_all(&jni_dir).map_err(|e| format!("{}: {e}", jni_dir.display()))?;
        let so_name = so.file_name().ok_or("the Swift output has no file name")?;
        std::fs::copy(so, jni_dir.join(so_name)).map_err(|e| format!("copying the .so: {e}"))?;

        copy_libcxx(rust_triple, &jni_dir)?;

        if let Some(dir) = std::env::var("SWIFTFLOW_ANDROID_RUNTIME_DIR")
            .ok()
            .filter(|v| !v.is_empty())
        {
            println!("▶ Copying the Swift runtime from SWIFTFLOW_ANDROID_RUNTIME_DIR...");
            for entry in std::fs::read_dir(&dir)
                .map_err(|e| format!("{dir}: {e}"))?
                .flatten()
            {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "so") {
                    if let Some(name) = path.file_name() {
                        std::fs::copy(&path, jni_dir.join(name))
                            .map_err(|e| format!("copying {}: {e}", path.display()))?;
                    }
                }
            }
        }

        println!("▶ Assembling the APK...");

        let abi_flag = format!("-PswiftflowAbi={abi}");
        let wrapper = gradle_dir.join("gradlew");

        if wrapper.is_file() {
            Run::new(&wrapper)
                .args(["assembleDebug", &abi_flag])
                .current_dir(gradle_dir)
                .run()?;
        } else if exists("gradle") {
            Run::new("gradle")
                .args(["assembleDebug", &abi_flag])
                .current_dir(gradle_dir)
                .run()?;
        } else {
            return Err(format!(
                "neither ./gradlew nor gradle found. Install Gradle, or run\n  \
                 'gradle wrapper' once in {} to create the wrapper.",
                gradle_dir.display()
            ));
        }

        let apk = gradle_dir.join("app/build/outputs/apk/debug/app-debug.apk");
        if !apk.is_file() {
            return Err(format!("Gradle produced no APK at {}", apk.display()));
        }

        if !self.run {
            println!("✓ Built {}", apk.display());
            return Ok(());
        }

        if !exists("adb") {
            println!(
                "✓ Built {} (adb not on PATH, so not installing)",
                apk.display()
            );
            return Ok(());
        }

        println!("▶ Installing...");
        Run::new("adb").arg("install").arg("-r").arg(&apk).run()?;

        let application_id = gradle_application_id(gradle_dir)
            .ok_or("could not read applicationId out of the Gradle project")?;
        Run::new("adb")
            .args([
                "shell",
                "am",
                "start",
                "-n",
                &format!("{application_id}/android.app.NativeActivity"),
            ])
            .run()?;
        println!("✓ Running. Logs: adb logcat -s SwiftFlow");
        Ok(())
    }
}

/// The symbol `swiftflow_android`'s `android_main` calls up through.
const ANDROID_ENTRY: &str = "sf_android_main";

/// The name of the type marked `@main` under `dir`, if there is one.
fn main_type(dir: &Path) -> Option<String> {
    for path in find(dir, 4, &|p| p.extension().is_some_and(|e| e == "swift")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut marked = false;
        for line in source.lines() {
            let line = line.trim();
            if line == "@main" {
                marked = true;
                continue;
            }
            if !marked || line.is_empty() || line.starts_with("//") || line.starts_with('@') {
                continue;
            }
            let rest = line
                .split_once("struct ")
                .or_else(|| line.split_once("enum "))
                .or_else(|| line.split_once("class "))
                .map(|(_, rest)| rest);
            let Some(rest) = rest else {
                marked = false;
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
            marked = false;
        }
    }
    None
}

fn declared_symbols(header: &str, package: &str) -> Vec<String> {
    let hosts = [
        ("sf_desktop_", "swiftflow_desktop"),
        ("sf_android_", "swiftflow_android"),
    ];
    let mut found = Vec::new();
    for line in header.lines() {
        if line.is_empty()
            || line.starts_with([' ', '\t'])
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.starts_with('#')
        {
            continue;
        }
        let Some(start) = line.find("sf_") else {
            continue;
        };
        let name: String = line[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.len() < 4 || !line[start + name.len()..].starts_with('(') {
            continue;
        }
        let belongs = hosts
            .iter()
            .find(|(prefix, _)| name.starts_with(prefix))
            .is_none_or(|(_, owner)| *owner == package);
        if belongs && !found.contains(&name) {
            found.push(name);
        }
    }
    found
}

fn defines(listing: &str, symbol: &str) -> bool {
    listing.lines().any(|line| {
        let mut fields = line.split_whitespace().rev();
        let Some(name) = fields.next() else {
            return false;
        };
        let Some(kind) = fields.next() else {
            return false;
        };
        name.strip_prefix('_').unwrap_or(name) == symbol && kind != "U" && kind != "u"
    })
}

fn host_triple() -> Result<String, String> {
    if let Some(explicit) = std::env::var("SWIFTFLOW_RUST_TRIPLE")
        .ok()
        .filter(|v| !v.is_empty())
    {
        return Ok(explicit);
    }
    let triple = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    } else {
        return Err("unrecognised host; set SWIFTFLOW_RUST_TRIPLE".into());
    };
    Ok(triple.to_string())
}

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn touch_swift_sources(dir: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "swift") {
            let contents = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            std::fs::write(&path, contents).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn gradle_application_id(gradle_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(gradle_dir.join("app/build.gradle.kts")).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("applicationId") {
            let value = rest.trim_start_matches([' ', '=']).trim();
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

fn sdk_stores() -> Vec<PathBuf> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Vec::new(),
    };
    vec![
        home.join("Library/org.swift.swiftpm/swift-sdks"),
        home.join(".swiftpm/swift-sdks"),
    ]
}

fn installed_bundles() -> Vec<PathBuf> {
    let mut bundles = Vec::new();
    for store in sdk_stores() {
        bundles.extend(find(&store, 1, &|p| {
            p.is_dir() && p.extension().is_some_and(|e| e == "artifactbundle")
        }));
    }
    bundles
}

fn installed_triples() -> Vec<String> {
    let mut triples = Vec::new();
    for bundle in installed_bundles() {
        for json in find(&bundle, 3, &|p| p.extension().is_some_and(|e| e == "json")) {
            if let Ok(text) = std::fs::read_to_string(&json) {
                for token in text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')) {
                    if token.contains("-unknown-linux-android") {
                        triples.push(token.to_string());
                    }
                }
            }
        }
    }
    triples.sort();
    triples.dedup();
    triples
}

fn check_android_sdk(gradle_dir: &Path) -> Result<(), String> {
    let sdk = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .ok()
        .filter(|v| !v.is_empty());
    if let Some(sdk) = sdk {
        if Path::new(&sdk).join("platforms").is_dir() {
            return Ok(());
        }
    }
    let local = gradle_dir.join("local.properties");
    if let Ok(text) = std::fs::read_to_string(&local) {
        if text.lines().any(|l| l.trim_start().starts_with("sdk.dir=")) {
            return Ok(());
        }
    }
    Err(
        "No Android SDK. Gradle needs one; the Swift SDK's NDK is not it.\n\n  \
         If Android Studio is installed:\n    \
         export ANDROID_HOME=\"$HOME/Library/Android/sdk\"\n\n  \
         Otherwise install the command-line tools, then:\n    \
         sdkmanager \"platforms;android-34\" \"build-tools;34.0.0\""
            .into(),
    )
}

fn check_swift_sdk(swift_sdk: &str) -> Result<(), String> {
    if !exists("swift") {
        return Err("No 'swift' on PATH.".into());
    }

    let bundles = installed_bundles();
    if bundles.is_empty() {
        let stores: Vec<String> = sdk_stores()
            .iter()
            .map(|p| format!("    {}", p.display()))
            .collect();
        return Err(format!(
            "No Swift SDK installed — nothing in:\n{}\n\n  \
             To install the official Android SDK (swift.org publishes API 28):\n\n    \
             swift sdk install \\\n      \
             https://download.swift.org/swift-6.3.3-release/android-sdk/swift-6.3.3-RELEASE/swift-6.3.3-RELEASE_android.artifactbundle.tar.gz \\\n      \
             --checksum d160cc3206dd1886dae3fef2337af5e25ec034692cd0ec225721c56cc69da7f5\n\n  \
             Check https://www.swift.org/documentation/articles/swift-sdk-for-android-getting-started.html\n  \
             for the release matching your Swift version — the checksum above is\n  \
             pinned to 6.3.3 and will be refused for any other build.",
            stores.join("\n")
        ));
    }

    let triples = installed_triples();
    if !triples.is_empty() && !triples.iter().any(|t| t == swift_sdk) {
        println!("⚠ No bundle here seems to provide '{swift_sdk}'. Android triples found:");
        for triple in &triples {
            println!("    {triple}");
        }
        println!("  The API level is part of the triple — set SWIFTFLOW_ANDROID_API");
        println!("  to one of the levels above if this turns out to be right.");
        println!("  Trying anyway; swift build has the last word.");
        println!();
    }

    check_toolchain_match()?;
    check_ndk_sysroot()
}

fn check_toolchain_match() -> Result<(), String> {
    let output = match Run::new("swift").arg("--version").capture() {
        Ok(text) => text,
        Err(_) => return Ok(()),
    };
    let host = match output
        .split_whitespace()
        .skip_while(|w| *w != "version")
        .nth(1)
    {
        Some(v) => v
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .to_string(),
        None => return Ok(()),
    };
    if host.is_empty() {
        return Ok(());
    }

    let mut mismatch = None;
    for bundle in installed_bundles() {
        let name = bundle
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.contains("android") {
            continue;
        }
        let sdk: String = name
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .find(|s| s.contains('.'))
            .unwrap_or("")
            .trim_matches('.')
            .to_string();
        if sdk.is_empty() {
            continue;
        }

        if sdk == host {
            return Ok(());
        }
        mismatch = Some(sdk);
    }

    let Some(sdk) = mismatch else { return Ok(()) };
    Err(format!(
        "Toolchain mismatch: host Swift is {host}, the Android SDK is {sdk}.\n\n  \
         A .swiftmodule is only readable by the compiler version that wrote\n  \
         it, and the SDK ships a prebuilt Foundation. These have to be the\n  \
         same Swift release; there is no compatibility window.\n\n  \
         Either install a {host} Android SDK, or move the host to {sdk}:\n\n    \
         swiftly install {sdk} && swiftly use {sdk}\n\n  \
         Official Android support starts at Swift 6.3, so upgrading the host\n  \
         is usually the shorter path. For an older host, finagolfin's\n  \
         community SDKs cover earlier releases:\n    \
         https://github.com/finagolfin/swift-android-sdk/releases\n  \
         Those are built for API 24 — set SWIFTFLOW_ANDROID_API=24 and lower\n  \
         minSdk in android/app/build.gradle.kts to match."
    ))
}

fn check_ndk_sysroot() -> Result<(), String> {
    let mut needs_setup = None;
    for bundle in installed_bundles() {
        if !bundle.join("scripts/setup-android-sdk.sh").is_file() {
            continue;
        }
        let has_sysroot = !find(&bundle, 3, &|p| {
            p.is_dir() && p.file_name().is_some_and(|n| n == "ndk-sysroot")
        })
        .is_empty();
        if has_sysroot {
            return Ok(());
        }
        needs_setup = Some(bundle);
    }

    let Some(bundle) = needs_setup else {
        return Ok(());
    };
    let ndk_set = std::env::var("ANDROID_NDK_HOME").is_ok();
    let steps = if ndk_set {
        "  ./scripts/setup-android-sdk.sh        # ANDROID_NDK_HOME is already set".to_string()
    } else {
        "\n  # NDK r27d or later. If you already have one, just point\n  \
         # ANDROID_NDK_HOME at it and skip the download.\n  \
         curl -fSL -o ndk.zip \\\n    \
         https://dl.google.com/android/repository/android-ndk-r27d-$(uname -s | tr 'A-Z' 'a-z').zip\n  \
         unzip -qo ndk.zip\n  \
         export ANDROID_NDK_HOME=$PWD/android-ndk-r27d\n  \
         ./scripts/setup-android-sdk.sh"
            .to_string()
    };
    Err(format!(
        "The Swift SDK has no ndk-sysroot — its one-time NDK setup hasn't run.\n\n  \
         The bundle ships without one because Google's NDK can't be\n  \
         redistributed. Without it there is no libc and no C headers, so\n  \
         the first #include fails and it reads like a broken checkout.\n\n  \
         cd {}\n{steps}\n\n  \
         Then build again. Set ANDROID_NDK_HOME in your shell profile too —\n  \
         the packaging step needs it for libc++_shared.so.",
        bundle.display()
    ))
}

fn static_resource_dir(swift_arch: &str) -> Option<PathBuf> {
    let suffix = format!("swift_static-{swift_arch}/android");
    for bundle in installed_bundles() {
        let hits = find(&bundle, 8, &|p| {
            p.is_dir() && p.to_string_lossy().ends_with(&suffix)
        });
        if let Some(first) = hits.into_iter().next() {
            return Some(first);
        }
    }
    None
}

fn ndk_clangxx(triple: &str, api: &str) -> Option<PathBuf> {
    let ndk = std::env::var("ANDROID_NDK_HOME")
        .or_else(|_| std::env::var("ANDROID_NDK_ROOT"))
        .ok()
        .filter(|v| !v.is_empty())?;
    let prebuilt = Path::new(&ndk).join("toolchains/llvm/prebuilt");
    let exact = format!("{triple}{api}-clang++");

    if let Some(found) = find(&prebuilt, 4, &|p| {
        p.file_name().is_some_and(|n| n == exact.as_str())
    })
    .into_iter()
    .next()
    {
        return Some(found);
    }

    find(&prebuilt, 4, &|p| {
        p.file_name().is_some_and(|n| n == "clang++")
    })
    .into_iter()
    .next()
}

fn copy_libcxx(rust_triple: &str, jni_dir: &Path) -> Result<(), String> {
    let ndk = std::env::var("ANDROID_NDK_HOME")
        .or_else(|_| std::env::var("ANDROID_NDK_ROOT"))
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(
            "ANDROID_NDK_HOME is unset, so libc++_shared.so can't be found.\n  \
             The Swift runtime needs it; the app would fail to load.\n  \
             Set it to your NDK root (r27d or later) — the Swift SDK's\n  \
             setup-android-sdk.sh needs the same variable.",
        )?;

    let prebuilt = Path::new(&ndk).join("toolchains/llvm/prebuilt");
    let wanted = format!("sysroot/usr/lib/{rust_triple}/libc++_shared.so");
    let found = find(&prebuilt, 8, &|p| p.to_string_lossy().ends_with(&wanted))
        .into_iter()
        .next();

    match found {
        Some(path) => {
            std::fs::copy(&path, jni_dir.join("libc++_shared.so"))
                .map_err(|e| format!("copying libc++_shared.so: {e}"))?;
            println!("✓ libc++_shared.so");
            Ok(())
        }
        None => Err(format!(
            "No libc++_shared.so for {rust_triple} under {}",
            prebuilt.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_triple_is_one_swiftpm_would_also_pick() {
        let triple = host_triple().expect("this host should be recognised");
        assert!(triple.contains('-'), "{triple} is not a triple");

        assert!([
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
        ]
        .contains(&triple.as_str()));
    }

    #[test]
    fn the_application_id_comes_out_of_the_gradle_file() {
        let tmp = std::env::temp_dir().join(format!("sf-gradle-{}", std::process::id()));
        let app = tmp.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("build.gradle.kts"),
            "android {\n    namespace = \"com.example.thing\"\n    defaultConfig {\n        applicationId = \"com.example.thing\"\n    }\n}\n",
        )
        .unwrap();
        assert_eq!(
            gradle_application_id(&tmp).as_deref(),
            Some("com.example.thing")
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_missing_gradle_file_is_not_a_panic() {
        assert_eq!(gradle_application_id(Path::new("/nonexistent")), None);
    }

    #[test]
    fn touching_sources_preserves_them_exactly() {
        let tmp = std::env::temp_dir().join(format!("sf-touch-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let file = tmp.join("A.swift");
        let body = "import SwiftFlow\n// ünïcödé and \u{1F600}\n";
        std::fs::write(&file, body).unwrap();
        std::fs::write(tmp.join("notes.txt"), "untouched").unwrap();

        touch_swift_sources(&tmp).unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), body);
        assert_eq!(
            std::fs::read_to_string(tmp.join("notes.txt")).unwrap(),
            "untouched"
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn env_or_ignores_an_exported_empty_value() {
        std::env::set_var("SF_TEST_EMPTY", "");
        assert_eq!(env_or("SF_TEST_EMPTY", "arm64-v8a"), "arm64-v8a");
        std::env::set_var("SF_TEST_EMPTY", "x86_64");
        assert_eq!(env_or("SF_TEST_EMPTY", "arm64-v8a"), "x86_64");
        std::env::remove_var("SF_TEST_EMPTY");
    }

    #[test]
    fn the_main_type_is_found_however_it_is_written() {
        let tmp = std::env::temp_dir().join(format!("sf-main-{}", std::process::id()));
        let src = tmp.join("Nested");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("App.swift"),
            "import SwiftFlow\n\n@main\nstruct DemoApp: SwiftFlowApp {\n}\n",
        )
        .unwrap();
        assert_eq!(main_type(&tmp).as_deref(), Some("DemoApp"));

        std::fs::write(
            src.join("App.swift"),
            "@main\npublic struct Demo_2: SwiftFlowApp {}\n",
        )
        .unwrap();
        assert_eq!(main_type(&tmp).as_deref(), Some("Demo_2"));

        std::fs::write(src.join("App.swift"), "@main\n@MainActor\nenum Tool {}\n").unwrap();
        assert_eq!(main_type(&tmp).as_deref(), Some("Tool"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn the_generated_entry_point_is_the_symbol_rust_calls() {
        // Two halves of one contract that nothing else compares. They were
        // `sf_android_main` and `swiftflow_android_main`, which links on
        // neither platform and fails only when someone builds for Android.
        let host = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("rust/swiftflow_android/src/lib.rs"),
        )
        .unwrap();
        assert!(
            host.contains(&format!("fn {ANDROID_ENTRY}();")),
            "the Android host does not declare {ANDROID_ENTRY}"
        );
    }

    #[test]
    fn a_project_without_a_main_type_reports_nothing() {
        let tmp = std::env::temp_dir().join(format!("sf-nomain-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("View.swift"), "struct ContentView: View {}\n").unwrap();
        assert_eq!(main_type(&tmp), None);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn the_header_scanner_finds_the_declarations_and_nothing_else() {
        let header = "\
// sf_render_tree is mentioned here in prose, with a call sf_init(x);
typedef struct SFNode {
    uint32_t node_id;
} SFNode;
void sf_render_tree(SFNode* root, float w, float h, float scale);
SFRect sf_get_node_frame(const SFNode* root, uint32_t node_id);
void sf_desktop_run(void* app);
void sf_android_run(void* app);
";
        let core = declared_symbols(header, "swiftflow_wgpu");
        assert_eq!(core, ["sf_render_tree", "sf_get_node_frame"]);
    }

    #[test]
    fn a_declaration_that_wraps_its_parameters_is_still_found() {
        let header = "\
size_t sf_hit_test_path(
    const SFNode* root,
    float x,
    float y
);
";
        assert_eq!(
            declared_symbols(header, "swiftflow_wgpu"),
            ["sf_hit_test_path"]
        );
    }

    #[test]
    fn each_host_owns_only_its_own_entry_point() {
        let header = "\
void sf_init(float scale);
void sf_desktop_run(void* app);
void sf_android_run(void* app);
";
        assert_eq!(
            declared_symbols(header, "swiftflow_desktop"),
            ["sf_init", "sf_desktop_run"]
        );
        assert_eq!(
            declared_symbols(header, "swiftflow_android"),
            ["sf_init", "sf_android_run"]
        );

        assert_eq!(declared_symbols(header, "swiftflow_wgpu"), ["sf_init"]);
    }

    #[test]
    fn the_real_header_declares_what_the_workspace_defines() {
        let header = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("Sources/CSwiftFlow/SwiftFlowMetal.h"),
        )
        .unwrap();
        let found = declared_symbols(&header, "swiftflow_desktop");
        assert!(
            found.contains(&"sf_get_node_frame".to_string()),
            "found {found:?}"
        );
        assert!(found.contains(&"sf_desktop_run".to_string()));
        assert!(!found.contains(&"sf_android_run".to_string()));
        assert_eq!(
            found.len(),
            15,
            "12 core plus the desktop host's three — {found:?}"
        );
    }

    #[test]
    fn a_built_desktop_archive_satisfies_the_header() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let header =
            std::fs::read_to_string(root.join("Sources/CSwiftFlow/SwiftFlowMetal.h")).unwrap();
        let archive = [
            "release",
            "x86_64-unknown-linux-gnu/release",
            "aarch64-apple-darwin/release",
        ]
        .iter()
        .map(|d| {
            root.join("rust/target")
                .join(d)
                .join("libswiftflow_desktop.a")
        })
        .find(|p| p.is_file());
        let Some(archive) = archive else { return };
        let Ok(listing) = Run::new("nm").arg("-g").arg(&archive).capture() else {
            return;
        };

        for symbol in declared_symbols(&header, "swiftflow_desktop") {
            assert!(
                defines(&listing, &symbol),
                "{} declares {symbol}, and {} does not define it",
                "SwiftFlowMetal.h",
                archive.display()
            );
        }
    }

    #[test]
    fn nm_output_is_read_for_definitions_not_references() {
        let mach = "0000000000000000 T _sf_get_node_frame\n                 U _sf_missing\n";
        let elf = "0000000000000000 T sf_get_node_frame\n                 U sf_missing\n";
        for listing in [mach, elf] {
            assert!(defines(listing, "sf_get_node_frame"));
            assert!(
                !defines(listing, "sf_missing"),
                "a `U` entry is the symbol being *wanted*, which is exactly \
                 the state that produces the link error this check exists for"
            );
            assert!(!defines(listing, "sf_absent"));
        }
    }
}
