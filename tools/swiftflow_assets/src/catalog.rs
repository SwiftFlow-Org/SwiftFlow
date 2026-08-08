use crate::contents::{Contents, ImageEntry};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(PathBuf, serde_json::Error),

    InvalidName(String),
    NoSuchSet(String),
    DuplicateName(String),

    UnsupportedFormat(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Json(path, e) => write!(f, "{}: {e}", path.display()),
            Error::InvalidName(n) => write!(f, "'{n}' is not a usable asset name"),
            Error::NoSuchSet(n) => write!(f, "no image set named '{n}'"),
            Error::DuplicateName(n) => write!(f, "an image set named '{n}' already exists"),
            Error::UnsupportedFormat(e) => {
                write!(f, "'{e}' files can't be decoded — use PNG or JPEG")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scale {
    One,
    Two,
    Three,
}

impl Scale {
    pub const ALL: [Scale; 3] = [Scale::One, Scale::Two, Scale::Three];

    pub fn as_str(self) -> &'static str {
        match self {
            Scale::One => "1x",
            Scale::Two => "2x",
            Scale::Three => "3x",
        }
    }

    pub fn factor(self) -> u32 {
        match self {
            Scale::One => 1,
            Scale::Two => 2,
            Scale::Three => 3,
        }
    }

    pub fn parse(s: &str) -> Option<Scale> {
        match s {
            "1x" => Some(Scale::One),
            "2x" => Some(Scale::Two),
            "3x" => Some(Scale::Three),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub scale: Scale,
    pub filename: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImageSet {

    pub name: String,
    pub dir: PathBuf,
    pub contents: Contents,
}

impl ImageSet {

    pub fn slots(&self) -> Vec<Slot> {
        Scale::ALL
            .iter()
            .map(|&scale| Slot {
                scale,
                filename: self.entry(scale).and_then(|e| e.filename.clone()),
            })
            .collect()
    }

    fn entry(&self, scale: Scale) -> Option<&ImageEntry> {
        self.contents.images.iter().find(|e| {
            e.idiom == "universal" && e.scale.as_deref() == Some(scale.as_str())
        })
    }

    fn entry_mut(&mut self, scale: Scale) -> Option<&mut ImageEntry> {
        self.contents.images.iter_mut().find(|e| {
            e.idiom == "universal" && e.scale.as_deref() == Some(scale.as_str())
        })
    }

    pub fn file_for(&self, scale: Scale) -> Option<PathBuf> {
        self.entry(scale)
            .and_then(|e| e.filename.as_ref())
            .map(|f| self.dir.join(f))
    }

    pub fn unscaled_file(&self) -> Option<PathBuf> {
        self.contents
            .images
            .iter()
            .find(|e| e.idiom == "universal" && e.scale.is_none())
            .and_then(|e| e.filename.as_ref())
            .map(|f| self.dir.join(f))
    }

    pub fn is_empty(&self) -> bool {
        self.slots().iter().all(|s| s.filename.is_none()) && self.unscaled_file().is_none()
    }

    fn write(&self) -> Result<()> {
        let json = self
            .contents
            .to_xcode_json()
            .map_err(|e| Error::Json(self.dir.join("Contents.json"), e))?;
        fs::write(self.dir.join("Contents.json"), json)?;
        Ok(())
    }
}

pub struct Catalog {
    pub root: PathBuf,

    pub sets: Vec<ImageSet>,
}

const SUPPORTED_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

impl Catalog {

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let root_contents = root.join("Contents.json");
        if !root_contents.exists() {
            let json = Contents::catalog_root()
                .to_xcode_json()
                .map_err(|e| Error::Json(root_contents.clone(), e))?;
            fs::write(&root_contents, json)?;
        }

        let mut catalog = Catalog {
            root,
            sets: Vec::new(),
        };
        catalog.reload()?;
        Ok(catalog)
    }

    pub fn reload(&mut self) -> Result<()> {
        let mut sets = Vec::new();
        collect_sets(&self.root, &mut sets)?;
        sets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.sets = sets;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ImageSet> {
        self.sets.iter().find(|s| s.name == name)
    }

    fn index_of(&self, name: &str) -> Result<usize> {
        self.sets
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| Error::NoSuchSet(name.to_string()))
    }

    pub fn create_set(&mut self, name: &str) -> Result<()> {
        validate_name(name)?;
        if self.get(name).is_some() {
            return Err(Error::DuplicateName(name.to_string()));
        }

        let dir = self.root.join(format!("{name}.imageset"));
        fs::create_dir_all(&dir)?;
        let set = ImageSet {
            name: name.to_string(),
            dir,
            contents: Contents::empty_image_set(),
        };
        set.write()?;
        self.reload()
    }

    pub fn rename_set(&mut self, old: &str, new: &str) -> Result<()> {
        validate_name(new)?;
        if old == new {
            return Ok(());
        }
        if self.get(new).is_some() {
            return Err(Error::DuplicateName(new.to_string()));
        }

        let index = self.index_of(old)?;
        let set = &self.sets[index];

        let parent = set.dir.parent().unwrap_or(&self.root).to_path_buf();
        let new_dir = parent.join(format!("{new}.imageset"));
        fs::rename(&set.dir, &new_dir)?;

        let mut renamed = ImageSet {
            name: new.to_string(),
            dir: new_dir,
            contents: self.sets[index].contents.clone(),
        };
        for scale in Scale::ALL {
            let Some(current) = renamed.entry(scale).and_then(|e| e.filename.clone()) else {
                continue;
            };
            let Some(ext) = extension_of(Path::new(&current)) else {
                continue;
            };
            let target = slot_filename(new, scale, &ext);
            if target == current {
                continue;
            }
            fs::rename(renamed.dir.join(&current), renamed.dir.join(&target))?;
            if let Some(entry) = renamed.entry_mut(scale) {
                entry.filename = Some(target);
            }
        }
        renamed.write()?;
        self.reload()
    }

    pub fn delete_set(&mut self, name: &str) -> Result<()> {
        let index = self.index_of(name)?;
        fs::remove_dir_all(&self.sets[index].dir)?;
        self.reload()
    }

    pub fn set_slot(&mut self, name: &str, scale: Scale, source: impl AsRef<Path>) -> Result<()> {
        let source = source.as_ref();
        let ext = extension_of(source).ok_or_else(|| {
            Error::UnsupportedFormat(
                source
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default(),
            )
        })?;

        let index = self.index_of(name)?;
        let filename = slot_filename(name, scale, &ext);
        let dest = self.sets[index].dir.join(&filename);

        fs::copy(source, &dest)?;

        let set = &mut self.sets[index];

        if let Some(previous) = set.entry(scale).and_then(|e| e.filename.clone()) {
            if previous != filename {
                let _ = fs::remove_file(set.dir.join(previous));
            }
        }
        match set.entry_mut(scale) {
            Some(entry) => entry.filename = Some(filename),
            None => set.contents.images.push(ImageEntry {
                filename: Some(filename),
                idiom: "universal".to_string(),
                scale: Some(scale.as_str().to_string()),
                extra: serde_json::Map::new(),
            }),
        }
        set.write()?;
        self.reload()
    }

    pub fn clear_slot(&mut self, name: &str, scale: Scale) -> Result<()> {
        let index = self.index_of(name)?;
        let set = &mut self.sets[index];
        let Some(filename) = set.entry(scale).and_then(|e| e.filename.clone()) else {
            return Ok(());
        };
        let _ = fs::remove_file(set.dir.join(filename));
        if let Some(entry) = set.entry_mut(scale) {
            entry.filename = None;
        }
        set.write()?;
        self.reload()
    }
}

fn slot_filename(name: &str, scale: Scale, ext: &str) -> String {
    match scale {
        Scale::One => format!("{name}.{ext}"),
        _ => format!("{name}@{}x.{ext}", scale.factor()),
    }
}

fn extension_of(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    SUPPORTED_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

fn validate_name(name: &str) -> Result<()> {
    let invalid = name.is_empty()
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || name != name.trim();
    if invalid {
        return Err(Error::InvalidName(name.to_string()));
    }
    Ok(())
}

fn collect_sets(dir: &Path, out: &mut Vec<ImageSet>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let Some(folder) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };

        if let Some(name) = folder.strip_suffix(".imageset") {
            let contents_path = path.join("Contents.json");
            let contents = match fs::read_to_string(&contents_path) {
                Ok(text) => {
                    Contents::parse(&text).map_err(|e| Error::Json(contents_path.clone(), e))?
                }

                Err(_) => Contents::empty_image_set(),
            };
            out.push(ImageSet {
                name: name.to_string(),
                dir: path,
                contents,
            });
        } else if !folder.contains('.') {
            collect_sets(&path, out)?;
        }
    }
    Ok(())
}
