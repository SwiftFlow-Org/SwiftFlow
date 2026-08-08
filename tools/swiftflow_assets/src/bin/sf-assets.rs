use std::path::PathBuf;
use std::process::ExitCode;
use swiftflow_assets::{flatten, Catalog, Scale};

const USAGE: &str = "\
sf-assets — SwiftFlow asset catalogue tool

USAGE:
    sf-assets flatten <catalogue> <output-dir>
    sf-assets list <catalogue>

    flatten   Copy every filled image into <output-dir> using the
              name@Nx.ext layout SwiftFlowCore's AssetCatalog resolves.
              Removes images left over from previous runs.

    list      Print each image set and which scales are filled.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [cmd, catalog, out] if cmd == "flatten" => {
            run_flatten(PathBuf::from(catalog), PathBuf::from(out))
        }
        [cmd, catalog] if cmd == "list" => run_list(PathBuf::from(catalog)),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sf-assets: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_flatten(catalog: PathBuf, out: PathBuf) -> swiftflow_assets::Result<()> {
    let report = flatten(&catalog, &out)?;

    println!(
        "sf-assets: wrote {} image(s) to {}",
        report.written.len(),
        out.display()
    );
    if !report.removed.is_empty() {
        println!("sf-assets: removed {} stale file(s)", report.removed.len());
    }

    for name in &report.empty {
        println!("warning: image set '{name}' has no images");
    }
    for slot in &report.missing {
        println!("warning: {slot} refers to a file that is not on disk");
    }
    for name in &report.conflicts {
        println!("warning: image set '{name}' collides with another of the same name");
    }
    Ok(())
}

fn run_list(catalog: PathBuf) -> swiftflow_assets::Result<()> {
    let catalog = Catalog::open(catalog)?;
    if catalog.sets.is_empty() {
        println!("(empty catalogue)");
        return Ok(());
    }
    for set in &catalog.sets {
        let mut filled: Vec<&str> = Vec::new();

        if set.unscaled_file().is_some() {
            filled.push("single");
        }
        filled.extend(
            Scale::ALL
                .iter()
                .filter(|&&s| set.file_for(s).is_some())
                .map(|s| s.as_str()),
        );
        let summary = if filled.is_empty() {
            "empty".to_string()
        } else {
            filled.join(" ")
        };
        println!("{:<32} {summary}", set.name);
    }
    Ok(())
}
