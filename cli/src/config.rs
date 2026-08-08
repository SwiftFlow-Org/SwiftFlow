use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = "SwiftFlow.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub swiftflow: SwiftFlowSection,
    #[serde(default)]
    pub app: AppSection,

    #[serde(default)]
    pub capabilities: BTreeMap<String, Capability>,
    #[serde(default)]
    pub ios: IosSection,
    #[serde(default)]
    pub android: AndroidSection,
    #[serde(default)]
    pub desktop: DesktopSection,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwiftFlowSection {

    pub version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSection {

    pub id: Option<String>,

    pub name: Option<String>,

    pub version: Option<String>,

    pub build: Option<u32>,

    pub icon: Option<String>,
    pub orientation: Option<Orientation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Portrait,
    Landscape,
    Any,
}

impl Orientation {

    pub fn android(self) -> &'static str {
        match self {
            Orientation::Portrait => "portrait",
            Orientation::Landscape => "landscape",
            Orientation::Any => "fullSensor",
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {

    pub reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IosSection {
    pub deployment_target: Option<String>,

    pub category: Option<String>,
    pub scheme: Option<String>,
    pub project: Option<String>,

    #[serde(default)]
    pub plist: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidSection {
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,

    pub namespace: Option<String>,

    pub application_id: Option<String>,
    pub gradle_module: Option<String>,
    #[serde(default)]
    pub icon: AndroidIcon,

    #[serde(default)]
    pub manifest: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidIcon {
    pub foreground: Option<String>,
    pub background: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopSection {
    pub bin: Option<String>,
    pub window_icon: Option<String>,
}

impl Config {
    pub fn path(project_root: &Path) -> PathBuf {
        project_root.join(FILE_NAME)
    }

    pub fn load(project_root: &Path) -> Result<Config, String> {
        let path = Self::path(project_root);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let config: Config = toml::from_str(&text)
            .map_err(|e| format!("{}: {e}", path.display()))?;

        config.validate().map_err(|why| format!("{}: {why}", path.display()))?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(id) = &self.app.id {
            validate_reverse_dns(id).map_err(|why| format!("[app] id {id:?}: {why}"))?;
        }
        if let Some(id) = &self.android.application_id {
            validate_reverse_dns(id)
                .map_err(|why| format!("[android] application_id {id:?}: {why}"))?;
        }
        for name in self.capabilities.keys() {
            if Capabilities::lookup(name).is_none() {
                return Err(format!(
                    "unknown capability {name:?}\n  known: {}\n  \
                     Anything outside this vocabulary goes in [ios.plist] or \
                     [android.manifest] instead.",
                    Capabilities::names().join(", ")
                ));
            }
        }
        Ok(())
    }

    pub fn pin(&self, project_root: &Path) -> Option<String> {
        if let Some(version) = self.swiftflow.version.as_ref() {
            let trimmed = version.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        std::fs::read_to_string(project_root.join(".swiftflow-version"))
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    pub fn android_application_id(&self) -> Option<&str> {
        self.android
            .application_id
            .as_deref()
            .or(self.app.id.as_deref())
    }

    pub fn android_namespace(&self) -> Option<&str> {
        self.android
            .namespace
            .as_deref()
            .or_else(|| self.android_application_id())
    }

    pub fn unlowered(&self) -> Vec<&'static str> {
        let mut pending = Vec::new();
        if self.app.icon.is_some() {
            pending.push("[app] icon — per-platform icon generation");
        }
        if self.android.icon.foreground.is_some() || self.android.icon.background.is_some() {
            pending.push("[android.icon] — adaptive icon layers");
        }
        if self.desktop.window_icon.is_some() {
            pending.push("[desktop] window_icon — .desktop generation");
        }
        if !self.ios.plist.is_empty() || self.ios.category.is_some() {
            pending.push("[ios.plist] / [ios] category — Info.plist merging");
        }
        pending
    }
}

fn validate_reverse_dns(id: &str) -> Result<(), String> {
    if !id.contains('.') {
        return Err("expected reverse-DNS, like com.example.app".into());
    }
    for segment in id.split('.') {
        if segment.is_empty() {
            return Err("has an empty segment".into());
        }
        if segment.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(format!("segment {segment:?} starts with a digit"));
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!("segment {segment:?} has characters Android rejects"));
        }
    }
    Ok(())
}

pub struct Lowering {
    pub name: &'static str,

    pub android: &'static [&'static str],

    pub ios_usage_key: Option<&'static str>,
}

pub struct Capabilities;

impl Capabilities {

    pub const ALL: &'static [Lowering] = &[
        Lowering {
            name: "camera",
            android: &["android.permission.CAMERA"],
            ios_usage_key: Some("NSCameraUsageDescription"),
        },
        Lowering {
            name: "microphone",
            android: &["android.permission.RECORD_AUDIO"],
            ios_usage_key: Some("NSMicrophoneUsageDescription"),
        },
        Lowering {
            name: "location-when-in-use",
            android: &[
                "android.permission.ACCESS_FINE_LOCATION",
                "android.permission.ACCESS_COARSE_LOCATION",
            ],
            ios_usage_key: Some("NSLocationWhenInUseUsageDescription"),
        },
        Lowering {
            name: "location-always",
            android: &[
                "android.permission.ACCESS_FINE_LOCATION",
                "android.permission.ACCESS_COARSE_LOCATION",
                "android.permission.ACCESS_BACKGROUND_LOCATION",
            ],
            ios_usage_key: Some("NSLocationAlwaysAndWhenInUseUsageDescription"),
        },
        Lowering {
            name: "photos",
            android: &["android.permission.READ_MEDIA_IMAGES"],
            ios_usage_key: Some("NSPhotoLibraryUsageDescription"),
        },
        Lowering {
            name: "contacts",
            android: &["android.permission.READ_CONTACTS"],
            ios_usage_key: Some("NSContactsUsageDescription"),
        },
        Lowering {
            name: "calendar",
            android: &["android.permission.READ_CALENDAR"],
            ios_usage_key: Some("NSCalendarsUsageDescription"),
        },
        Lowering {
            name: "bluetooth",
            android: &[
                "android.permission.BLUETOOTH_CONNECT",
                "android.permission.BLUETOOTH_SCAN",
            ],
            ios_usage_key: Some("NSBluetoothAlwaysUsageDescription"),
        },
        Lowering {
            name: "notifications",
            android: &["android.permission.POST_NOTIFICATIONS"],

            ios_usage_key: None,
        },
        Lowering {
            name: "network",
            android: &["android.permission.INTERNET"],

            ios_usage_key: None,
        },
    ];

    pub fn lookup(name: &str) -> Option<&'static Lowering> {
        Self::ALL.iter().find(|c| c.name == name)
    }

    pub fn names() -> Vec<&'static str> {
        Self::ALL.iter().map(|c| c.name).collect()
    }
}

impl Config {

    pub fn android_permissions(&self) -> Vec<String> {
        let mut permissions = vec!["android.permission.INTERNET".to_string()];
        for name in self.capabilities.keys() {
            if let Some(lowering) = Capabilities::lookup(name) {
                permissions.extend(lowering.android.iter().map(|p| p.to_string()));
            }
        }
        permissions.sort();
        permissions.dedup();
        permissions
    }

    pub fn ios_usage_descriptions(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for (name, capability) in &self.capabilities {
            let Some(lowering) = Capabilities::lookup(name) else {
                continue;
            };
            let Some(key) = lowering.ios_usage_key else {
                continue;
            };
            if let Some(reason) = &capability.reason {
                out.push((key, reason.clone()));
            }
        }
        out
    }

    pub fn capabilities_missing_reasons(&self) -> Vec<&str> {
        self.capabilities
            .iter()
            .filter(|(name, capability)| {
                capability.reason.is_none()
                    && Capabilities::lookup(name).is_some_and(|l| l.ios_usage_key.is_some())
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        config.validate()?;
        Ok(config)
    }

    #[test]
    fn an_absent_file_is_not_an_error() {
        let config = Config::load(Path::new("/nonexistent")).unwrap();
        assert!(config.app.id.is_none());
    }

    #[test]
    fn a_typo_is_an_error_rather_than_silence() {

        let error = parse("[app]\nnaem = \"Oops\"\n").unwrap_err();
        assert!(error.contains("naem"), "unhelpful: {error}");
    }

    #[test]
    fn the_pin_moves_out_of_the_old_file() {
        let config = parse("[swiftflow]\nversion = \"dev\"\n").unwrap();
        assert_eq!(config.pin(Path::new("/nowhere")).as_deref(), Some("dev"));
    }

    #[test]
    fn the_old_pin_file_still_works_during_a_migration() {
        let dir = std::env::temp_dir().join(format!("sf-pin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".swiftflow-version"), "0.1.0\n").unwrap();
        let config = Config::default();
        assert_eq!(config.pin(&dir).as_deref(), Some("0.1.0"));

        let config = parse("[swiftflow]\nversion = \"dev\"\n").unwrap();
        assert_eq!(config.pin(&dir).as_deref(), Some("dev"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn typed_platform_values_beat_canonical_ones() {

        let config = parse(
            "[app]\nid = \"com.example.app\"\n\
             [android]\napplication_id = \"com.example.app.pro\"\n",
        )
        .unwrap();
        assert_eq!(config.android_application_id(), Some("com.example.app.pro"));

        assert_eq!(config.android_namespace(), Some("com.example.app.pro"));
    }

    #[test]
    fn canonical_is_used_when_nothing_overrides_it() {
        let config = parse("[app]\nid = \"com.example.app\"\n").unwrap();
        assert_eq!(config.android_application_id(), Some("com.example.app"));
    }

    #[test]
    fn a_bad_identifier_is_caught_here_rather_than_by_gradle() {
        for bad in ["noDots", "com..app", "com.1app", "com.my-app"] {
            let error = parse(&format!("[app]\nid = \"{bad}\"\n")).unwrap_err();
            assert!(error.contains("id"), "{bad} produced: {error}");
        }
        assert!(parse("[app]\nid = \"com.example.my_app\"\n").is_ok());
    }

    #[test]
    fn capabilities_lower_to_permissions() {
        let config = parse(
            "[capabilities.camera]\nreason = \"Scan pages.\"\n\
             [capabilities.location-when-in-use]\nreason = \"Nearby content.\"\n",
        )
        .unwrap();
        let permissions = config.android_permissions();
        assert!(permissions.contains(&"android.permission.CAMERA".to_string()));

        assert!(permissions.contains(&"android.permission.ACCESS_FINE_LOCATION".to_string()));
        assert!(permissions.contains(&"android.permission.ACCESS_COARSE_LOCATION".to_string()));

        assert!(permissions.contains(&"android.permission.INTERNET".to_string()));

        let usage = config.ios_usage_descriptions();
        assert!(usage.contains(&("NSCameraUsageDescription", "Scan pages.".into())));
    }

    #[test]
    fn an_unknown_capability_names_the_vocabulary() {
        let error = parse("[capabilities.telepathy]\nreason = \"Why not.\"\n").unwrap_err();
        assert!(error.contains("telepathy"));
        assert!(error.contains("camera"), "should list what is known: {error}");
        assert!(error.contains("[android.manifest]"), "should point at the escape hatch");
    }

    #[test]
    fn a_capability_without_a_reason_is_reported() {

        let config = parse("[capabilities.camera]\n").unwrap();
        assert_eq!(config.capabilities_missing_reasons(), vec!["camera"]);

        let config = parse("[capabilities.notifications]\n").unwrap();
        assert!(config.capabilities_missing_reasons().is_empty());
    }

    #[test]
    fn declared_but_not_yet_lowered_is_reported_not_ignored() {
        let config = parse("[app]\nicon = \"assets/icon.png\"\n").unwrap();
        assert!(!config.unlowered().is_empty());
        assert!(config.unlowered()[0].contains("icon"));
    }

    #[test]
    fn the_users_own_example_parses() {

        let config = parse(
            r##"
[app]
id      = "com.example.swiftflow"
name    = "SwiftFlow Example"
version = "0.1.0"
build   = 1
orientation = "portrait"

[capabilities.camera]
reason = "Scan pages into the reader."

[capabilities.location-when-in-use]
reason = "Show nearby content."

[ios]
deployment_target = "16.0"
category = "public.app-category.books"
scheme  = "SwiftFlowExample"
project = "SwiftFlowExample.xcodeproj"

[ios.plist]
UIStatusBarStyle = "UIStatusBarStyleLightContent"
ITSAppUsesNonExemptEncryption = false

[android]
min_sdk       = 26
target_sdk    = 34
namespace     = "com.example.swiftflow"
gradle_module = "app"

[android.icon]
foreground = "assets/icon-fg.png"
background = "#0B0B0F"

[android.manifest]
"android:hardwareAccelerated" = true

[desktop]
bin = "swiftflow-example"
"##,
        )
        .unwrap();
        assert_eq!(config.app.build, Some(1));
        assert_eq!(config.android.min_sdk, Some(26));
        assert_eq!(config.app.orientation, Some(Orientation::Portrait));
        assert_eq!(config.ios.plist.len(), 2);
        assert_eq!(config.android.manifest.len(), 1);
    }
}
