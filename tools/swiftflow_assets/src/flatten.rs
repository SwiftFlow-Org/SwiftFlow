use crate::catalog::{Catalog, Result, Scale};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct FlattenReport {

    pub written: Vec<String>,

    pub conflicts: Vec<String>,

    pub empty: Vec<String>,

    pub missing: Vec<String>,

    pub removed: Vec<String>,
}

pub fn flatten(catalog_root: impl AsRef<Path>, out_dir: impl AsRef<Path>) -> Result<FlattenReport> {
    let out_dir = out_dir.as_ref();
    let catalog = Catalog::open(catalog_root)?;
    fs::create_dir_all(out_dir)?;

    let mut report = FlattenReport::default();
    let mut claimed: HashSet<String> = HashSet::new();

    for set in &catalog.sets {
        if set.is_empty() {
            report.empty.push(set.name.clone());
            continue;
        }

        let mut sources: Vec<(Option<Scale>, std::path::PathBuf)> = Vec::new();
        if let Some(path) = set.unscaled_file() {
            sources.push((None, path));
        }
        for scale in Scale::ALL {
            if let Some(path) = set.file_for(scale) {
                sources.push((Some(scale), path));
            }
        }

        for (scale, source) in sources {

            let label = scale.map(|s| s.as_str()).unwrap_or("single scale");
            if !source.is_file() {
                report.missing.push(format!("{} ({})", set.name, label));
                continue;
            }
            let Some(ext) = source
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
            else {
                continue;
            };

            let filename = match scale {

                Some(scale) => format!("{}@{}x.{}", set.name, scale.factor(), ext),
                None => format!("{}.{}", set.name, ext),
            };
            if !claimed.insert(filename.clone()) {
                report.conflicts.push(set.name.clone());
                continue;
            }

            fs::copy(&source, out_dir.join(&filename))?;
            report.written.push(filename);
        }
    }

    for entry in fs::read_dir(out_dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if claimed.contains(&name) || !path.is_file() {
            continue;
        }
        let is_ours = matches!(
            path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .as_deref(),
            Some("png") | Some("jpg") | Some("jpeg")
        );
        if is_ours {
            fs::remove_file(&path)?;
            report.removed.push(name);
        }
    }

    report.written.sort();
    report.removed.sort();
    Ok(report)
}
