use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use swiftflow_cli::build::Build;
use swiftflow_cli::{
    config, find_project, framework_root, framework_version, home, scaffold, Env, Platform,
    Project, RealEnv,
};

#[derive(Parser)]
#[command(
    name = "swiftflow",
    version,
    about = "Create, build and run SwiftFlow apps",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {

    New {

        name: String,

        #[arg(long)]
        pin: Option<String>,

        #[arg(long)]
        force: bool,
    },

    Build {
        #[arg(long, short)]
        platform: Option<Platform>,
    },

    Run {
        #[arg(long, short)]
        platform: Option<Platform>,
    },

    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let env = RealEnv;
    let result = match cli.command {
        Cmd::New { name, pin, force } => cmd_new(&env, &name, pin.as_deref(), force),
        Cmd::Build { platform } => cmd_build(&env, platform, false),
        Cmd::Run { platform } => cmd_build(&env, platform, true),
        Cmd::Doctor => cmd_doctor(&env),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_new(env: &dyn Env, name: &str, pin: Option<&str>, force: bool) -> Result<(), String> {
    let dir = PathBuf::from(name);
    if dir.exists() && !force {
        return Err(format!(
            "{} already exists — pass --force to write into it",
            dir.display()
        ));
    }

    let home_dir = home(env);
    let resolved_pin = match pin {
        Some(explicit) => Some(explicit.to_string()),
        None => framework_version(&framework_root(&home_dir, None)),
    };

    let leaf = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());

    for file in scaffold(&leaf, resolved_pin.as_deref()) {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, &file.contents).map_err(|e| format!("{}: {e}", path.display()))?;
    }

    println!("Created {}", dir.display());
    match resolved_pin {
        Some(pin) => println!("  pinned to SwiftFlow {pin}"),
        None => println!(
            "  no framework installed yet — run scripts/install.sh in a \
             SwiftFlow checkout, then set [swiftflow] version in {}",
            config::FILE_NAME
        ),
    }
    println!();
    println!("  cd {} && swiftflow run", dir.display());
    Ok(())
}

struct Resolved {
    project: Project,
    framework: PathBuf,
    version: String,
    platform: Platform,
    home: PathBuf,
}

fn resolve(env: &dyn Env, platform: Option<Platform>) -> Result<Resolved, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read the working directory: {e}"))?;
    let project = find_project(&cwd).ok_or_else(|| {
        format!(
            "no Package.swift in {} or any parent — is this a SwiftFlow project?",
            cwd.display()
        )
    })?;

    let home_dir = home(env);
    let framework = framework_root(&home_dir, project.pin.as_deref());
    if !framework.join("Package.swift").is_file() {
        return Err(match &project.pin {
            Some(pin) => format!(
                "this project pins SwiftFlow {pin}, which is not installed\n  \
                 looked in {}\n  installed versions: {}",
                framework.display(),
                installed_versions(&home_dir).join(", ")
            ),
            None => format!(
                "no current SwiftFlow — {} does not exist\n  \
                 run scripts/install.sh in a SwiftFlow checkout",
                framework.display()
            ),
        });
    }

    let version = framework_version(&framework).ok_or_else(|| {
        format!("{} has no VERSION file", framework.display())
    })?;

    Ok(Resolved {
        platform: platform.unwrap_or_else(Platform::default_for_host),
        project,
        framework,
        version,
        home: home_dir,
    })
}

fn cmd_build(env: &dyn Env, platform: Option<Platform>, run: bool) -> Result<(), String> {
    let r = resolve(env, platform)?;

    println!(
        "▶ {} {} for {} (SwiftFlow {})",
        if run { "Running" } else { "Building" },
        r.project.name,
        r.platform,
        r.version
    );

    Build {
        project: r.project,
        framework: r.framework,
        home: r.home,
        platform: r.platform,
        run,
    }
    .go()
}

fn cmd_doctor(env: &dyn Env) -> Result<(), String> {
    let home_dir = home(env);
    println!("SwiftFlow home   {}", home_dir.display());

    let versions = installed_versions(&home_dir);
    if versions.is_empty() {
        println!("installed        (none) — run scripts/install.sh in a checkout");
    } else {
        println!("installed        {}", versions.join(", "));
    }

    let current = framework_root(&home_dir, None);
    match framework_version(&current) {
        Some(v) => println!("current          {v}"),
        None => println!("current          (not set)"),
    }

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    match find_project(&cwd) {
        None => println!("\nNo project here — `swiftflow new <name>` starts one."),
        Some(project) => {
            println!("\nproject          {} ({})", project.name, project.root.display());
            println!("pin              {}", project.pin.as_deref().unwrap_or("(none, uses current)"));
            let framework = framework_root(&home_dir, project.pin.as_deref());
            match framework_version(&framework) {
                Some(v) => {
                    println!("resolves to      {} ({v})", framework.display());
                    let target = framework.join("rust/target");
                    println!("rust output      {}", target.display());
                    if let Ok(link) = std::fs::read_link(&target) {
                        println!("                 -> {}", link.display());
                    }
                }
                None => println!(
                    "resolves to      {} — NOT INSTALLED",
                    framework.display()
                ),
            }
            println!("default platform {}", Platform::default_for_host());

            let config_path = config::Config::path(&project.root);
            if config_path.exists() {
                println!("\nconfig           {}", config_path.display());
            } else {
                println!("\nconfig           (none — every field is defaulted)");
            }
            if let Some(id) = project.config.app.id.as_deref() {
                println!("app id           {id}");
            }
            if let Some(name) = project.config.app.name.as_deref() {
                println!("app name         {name}");
            }
            let caps: Vec<&str> = project.config.capabilities.keys().map(|k| k.as_str()).collect();
            if !caps.is_empty() {
                println!("capabilities     {}", caps.join(", "));
            }

            let pending = project.config.unlowered();
            if !pending.is_empty() {
                println!("\nDeclared but not lowered yet:");
                for item in pending {
                    println!("  {item}");
                }
            }
        }
    }
    Ok(())
}

fn installed_versions(home: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(home.join("versions")) {
        for entry in entries.flatten() {
            out.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}
