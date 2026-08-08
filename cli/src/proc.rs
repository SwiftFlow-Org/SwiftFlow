use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct Run {
    command: Command,
    label: String,
}

impl Run {
    pub fn new(program: impl AsRef<OsStr>) -> Self {
        let program = program.as_ref().to_string_lossy().into_owned();
        Run {
            command: Command::new(&program),
            label: program,
        }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.command.env(key, value);
        self
    }

    pub fn current_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.command.current_dir(dir);
        self
    }

    pub fn run(mut self) -> Result<(), String> {
        let status = self.command.status().map_err(|e| self.spawn_error(e))?;
        if status.success() {
            Ok(())
        } else {
            Err(match status.code() {
                Some(code) => format!("{} exited with status {code}", self.label),
                None => format!("{} was killed by a signal", self.label),
            })
        }
    }

    pub fn capture(mut self) -> Result<String, String> {
        let output = self
            .command
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| self.spawn_error(e))?;
        if !output.status.success() {
            return Err(format!("{} failed", self.label));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn spawn_error(&self, error: std::io::Error) -> String {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("{} is not on PATH", self.label)
        } else {
            format!("could not run {}: {error}", self.label)
        }
    }
}

pub fn exists(program: &str) -> bool {
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || candidate.with_extension("exe").is_file()
    })
}

pub fn find(dir: &Path, max_depth: usize, predicate: &dyn Fn(&Path) -> bool) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    walk(dir, max_depth, &mut |path| {
        if predicate(path) {
            found.push(path.to_path_buf());
        }
    });
    found
}

fn walk(dir: &Path, depth_left: usize, visit: &mut dyn FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        visit(&path);
        if depth_left > 1 && path.is_dir() {
            walk(&path, depth_left - 1, visit);
        }
    }
}
