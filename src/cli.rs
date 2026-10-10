//! Shared path resolution for the TUI and one-shot CLI commands.

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use crate::sample;

/// Default data root; an explicit TODO_DIR overrides ~/.todot.
pub fn data_dir() -> io::Result<PathBuf> {
    default_dir(std::env::var_os("TODO_DIR"), std::env::var_os("HOME"))
}

fn default_dir(todo_dir: Option<OsString>, home: Option<OsString>) -> io::Result<PathBuf> {
    if let Some(dir) = todo_dir {
        return Ok(PathBuf::from(dir));
    }
    home.map(|h| PathBuf::from(h).join(".todot"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "HOME 未设置，请指定 TODO_DIR 或 TODO_FILE",
            )
        })
}

/// Explicit FILE → TODO_FILE → TODO_DIR/todo.txt → ~/.todot/todo.txt.
/// Never implicitly reads the current directory or opens sample data.
pub fn resolve_path(arg: Option<String>) -> io::Result<PathBuf> {
    ensure_file(decide_path(
        arg,
        std::env::var_os("TODO_FILE"),
        std::env::var_os("TODO_DIR"),
        std::env::var_os("HOME"),
    )?)
}

fn decide_path(
    arg: Option<String>,
    todo_file: Option<OsString>,
    todo_dir: Option<OsString>,
    home: Option<OsString>,
) -> io::Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(PathBuf::from(p));
    }
    if let Some(f) = todo_file {
        return Ok(PathBuf::from(f));
    }
    Ok(default_dir(todo_dir, home)?.join("todo.txt"))
}

/// DONE_FILE overrides the sibling done.txt next to the selected todo file.
pub fn done_path(todo_path: &Path) -> PathBuf {
    if let Some(f) = std::env::var_os("DONE_FILE") {
        return PathBuf::from(f);
    }
    todo_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("done.txt")
}

/// Create missing directories and file without truncating existing data.
pub fn ensure_file(pb: PathBuf) -> io::Result<PathBuf> {
    if let Some(parent) = pb.parent().filter(|p| !p.as_os_str().is_empty()) {
        let mut dirs = std::fs::DirBuilder::new();
        dirs.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            dirs.mode(0o700);
        }
        dirs.create(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(&pb) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    Ok(pb)
}

/// Explicit --sample remains available; never used as a default.
pub fn sample_path() -> io::Result<PathBuf> {
    let dir = std::env::temp_dir();
    let pb = dir.join("todot-sample.txt");
    std::fs::write(&pb, sample::TODO_RAW)?;
    match std::fs::remove_file(dir.join("done.txt")) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    Ok(pb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_precedence_and_default() {
        let file = Some(OsString::from("/env/file.txt"));
        let dir = Some(OsString::from("/env/dir"));
        let home = Some(OsString::from("/home/user"));
        assert_eq!(
            decide_path(
                Some("custom.txt".into()),
                file.clone(),
                dir.clone(),
                home.clone()
            )
            .unwrap(),
            PathBuf::from("custom.txt")
        );
        assert_eq!(
            decide_path(None, file, dir.clone(), home.clone()).unwrap(),
            PathBuf::from("/env/file.txt")
        );
        assert_eq!(
            decide_path(None, None, dir, home.clone()).unwrap(),
            PathBuf::from("/env/dir/todo.txt")
        );
        assert_eq!(
            decide_path(None, None, None, home).unwrap(),
            PathBuf::from("/home/user/.todot/todo.txt")
        );
        assert!(decide_path(None, None, None, None).is_err());
        assert_eq!(
            decide_path(None, Some("explicit.txt".into()), None, None).unwrap(),
            PathBuf::from("explicit.txt")
        );
    }
}
