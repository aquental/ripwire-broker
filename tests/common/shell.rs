//! The bash interpreters a script test runs under: the first on `PATH` and, when it is another
//! file, `/bin/bash` (3.2 on macOS). A script must behave the same under each.
use std::fs;
use std::path::{Path, PathBuf};

pub fn bashes() -> Vec<PathBuf> {
    let first = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|dir| Path::new(dir).join("bash"))
        .find(|bash| bash.is_file())
        .expect("a bash on PATH");
    let mut shells = vec![first];
    let system = PathBuf::from("/bin/bash");
    let same = |a: &Path, b: &Path| fs::canonicalize(a).ok() == fs::canonicalize(b).ok();
    if system.is_file() && !same(&shells[0], &system) {
        shells.push(system);
    }
    shells
}
