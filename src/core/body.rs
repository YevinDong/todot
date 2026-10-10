use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use super::{Store, StoreError};
use crate::todo::Task;
use crate::{note, todo};

struct StagedBody {
    path: PathBuf,
    staged: PathBuf,
    bytes: Vec<u8>,
}

fn rollback(files: &[StagedBody]) -> io::Result<()> {
    let mut failures = Vec::new();
    for file in files.iter().rev() {
        // A hard link restores without overwriting a concurrent external write.
        if let Err(e) =
            fs::hard_link(&file.staged, &file.path).and_then(|_| fs::remove_file(&file.staged))
        {
            failures.push(format!("{}：{e}", file.staged.display()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "正文回滚失败，原内容仍保存在：{}",
            failures.join("；")
        )))
    }
}

impl Store {
    /// Shared by single, bulk, archive and CLI deletion. Completed/shared bodies stay.
    pub(crate) fn commit_deletion(
        &mut self,
        remaining: Vec<Task>,
        removed: &[Task],
        archive: bool,
    ) -> Result<Option<StoreError>, StoreError> {
        let has_body = removed
            .iter()
            .any(|task| note::note_rel_from_raw(&task.raw).is_some());
        let _lock = if has_body && self.notes_dir.exists() {
            Some(note::lock_dir(&self.notes_dir).map_err(StoreError::BodyIo)?)
        } else {
            None
        };
        let mut files: Vec<StagedBody> = Vec::new();
        let stage = (|| -> io::Result<()> {
            if !has_body {
                return Ok(());
            }
            let root = match fs::canonicalize(&self.notes_dir) {
                Ok(root) => root,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e),
            };
            let mut candidates = HashSet::new();
            for task in removed {
                let Some(path) = note::linked_path(&task.raw, &self.notes_dir) else {
                    continue;
                };
                let path = match fs::canonicalize(path) {
                    Ok(path) => path,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(e),
                };
                // Never remove arbitrary external files linked by legacy notes.
                if path.starts_with(&root) {
                    candidates.insert(path);
                }
            }
            if candidates.is_empty() {
                return Ok(());
            }
            let other_path = if archive {
                &self.file_path
            } else {
                &self.archive.path
            };
            let others = match fs::read_to_string(other_path) {
                Ok(raw) => todo::parse_file(&raw),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
                Err(e) => return Err(e),
            };
            for task in remaining.iter().chain(&others) {
                if let Some(path) = note::linked_path(&task.raw, &self.notes_dir) {
                    match fs::canonicalize(path) {
                        Ok(path) => {
                            candidates.remove(&path);
                        }
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e),
                    }
                }
            }
            for path in candidates {
                if !path.is_file() {
                    return Err(io::Error::other("正文路径不是普通文件"));
                }
                let id = note::new_body_rel()?.replace('/', "-");
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let staged = path.with_file_name(format!(".deleted-{name}-{id}"));
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&staged)?;
                if let Err(e) = fs::rename(&path, &staged) {
                    let _ = fs::remove_file(&staged);
                    return Err(e);
                }
                files.push(StagedBody {
                    path,
                    staged,
                    bytes: Vec::new(),
                });
                let file = files.last_mut().expect("just staged body");
                file.bytes = fs::read(&file.staged)?;
            }
            Ok(())
        })();
        if let Err(e) = stage {
            rollback(&files).map_err(StoreError::BodyIo)?;
            return Err(StoreError::BodyIo(e));
        }
        let raw = todo::serialize(&remaining);
        let path = if archive {
            &self.archive.path
        } else {
            &self.file_path
        };
        if let Err(e) = todo::write_atomic(path, &raw) {
            rollback(&files).map_err(StoreError::BodyIo)?;
            return Err(StoreError::Write(e));
        }
        let bodies: Vec<(PathBuf, Arc<[u8]>)> = files
            .iter_mut()
            .map(|file| {
                (
                    file.path.clone(),
                    Arc::from(std::mem::take(&mut file.bytes)),
                )
            })
            .collect();
        if archive {
            self.archive.tasks = remaining;
            self.archive.last_disk = raw;
            self.archive.loader = None;
        } else {
            let previous = std::mem::replace(&mut self.tasks, remaining);
            self.last_disk = raw;
            self.history.push_with_bodies(previous, bodies.clone());
        }
        self.history.remember_bodies(&bodies, &self.notes_dir);
        // ponytail: bounded in-memory backups support session undo; crash-atomic deletion needs a journal.
        let mut failures = Vec::new();
        for file in files {
            if let Err(e) = fs::remove_file(&file.staged) {
                failures.push(format!("{}：{e}", file.staged.display()));
            }
        }
        Ok((!failures.is_empty()).then(|| {
            StoreError::BodyIo(io::Error::other(format!(
                "任务已删除，临时正文清理失败：{}",
                failures.join("；")
            )))
        }))
    }

    pub(crate) fn restore_deleted_bodies(
        &self,
        bodies: &[(PathBuf, Arc<[u8]>)],
    ) -> Result<Option<File>, StoreError> {
        if bodies.is_empty() {
            return Ok(None);
        }
        let lock = note::lock_dir(&self.notes_dir).map_err(StoreError::BodyIo)?;
        for (path, bytes) in bodies {
            match fs::read(path) {
                // Undo restores a missing file, never overwrites newer user content.
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(StoreError::BodyIo(e)),
            }
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(StoreError::BodyIo)?;
            }
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(path).map_err(StoreError::BodyIo)?;
            if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                drop(file);
                let _ = fs::remove_file(path);
                return Err(StoreError::BodyIo(e));
            }
        }
        Ok(Some(lock))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::core::{
        AddOutcome, ArchiveDeleteOutcome, ArchiveOutcome, BulkDeleteOutcome, CompleteOutcome,
        DeleteOutcome, UndoOutcome,
    };

    #[test]
    fn deleting_bodies_preserves_completion_sharing_undo_and_failed_writes() {
        let dir = std::env::temp_dir().join(format!("todot-delete-body-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("notes")).unwrap();
        let path = dir.join("todo.txt");
        let raw = "first note:shared.md\nsecond note:shared.md\n";
        fs::write(&path, raw).unwrap();
        let shared = dir.join("notes/shared.md");
        let content = "# 正文\n\n  缩进和 +标记 @场景 原样保留\n";
        fs::write(&shared, content).unwrap();
        let mut store = Store::open_sync(path.clone(), raw.into(), "2026-10-10".into());
        assert!(matches!(
            store.toggle_complete(0),
            CompleteOutcome::Completed { .. }
        ));
        assert_eq!(fs::read_to_string(&shared).unwrap(), content);
        assert!(matches!(
            store.archive_completed(),
            ArchiveOutcome::Archived { count: 1 }
        ));
        assert!(shared.exists());

        let before = fs::read_to_string(&path).unwrap();
        let history_len = store.history.len();
        fs::create_dir(path.with_extension("tmp")).unwrap();
        assert!(matches!(store.delete(0), DeleteOutcome::Error(_)));
        assert_eq!(store.tasks().len(), 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        assert_eq!(fs::read_to_string(&shared).unwrap(), content);
        assert_eq!(store.history.len(), history_len);
        fs::remove_dir(path.with_extension("tmp")).unwrap();

        assert!(matches!(store.delete(0), DeleteOutcome::Deleted { .. }));
        assert!(
            shared.exists(),
            "completed archive still references shared body"
        );
        assert!(matches!(
            store.archive_delete(0),
            ArchiveDeleteOutcome::Deleted
        ));
        assert!(!shared.exists(), "last reference deletion must remove file");
        assert!(matches!(store.undo(), UndoOutcome::Undone));
        assert_eq!(store.tasks().len(), 1);
        assert_eq!(fs::read_to_string(&shared).unwrap(), content);

        let other = dir.join("notes/other.md");
        fs::write(&other, "另一正文").unwrap();
        assert!(matches!(
            store.add_finalized("third note:other.md"),
            AddOutcome::Added { .. }
        ));
        let before = fs::read_to_string(&path).unwrap();
        fs::create_dir(path.with_extension("tmp")).unwrap();
        assert!(matches!(
            store.delete_many(&[0, 1]),
            BulkDeleteOutcome::Error(_)
        ));
        assert_eq!(store.tasks().len(), 2);
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        assert_eq!(fs::read_to_string(&shared).unwrap(), content);
        assert_eq!(fs::read_to_string(&other).unwrap(), "另一正文");
        assert!(!fs::read_dir(dir.join("notes")).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".deleted-")
        }));
        fs::remove_dir(path.with_extension("tmp")).unwrap();

        assert!(matches!(
            store.delete_many(&[0, 1, 1, 999]),
            BulkDeleteOutcome::Done { deleted: 2 }
        ));
        assert!(!shared.exists() && !other.exists());
        assert!(matches!(store.undo(), UndoOutcome::Undone));
        assert_eq!(fs::read_to_string(&shared).unwrap(), content);
        assert_eq!(fs::read_to_string(&other).unwrap(), "另一正文");
        assert!(matches!(store.delete(0), DeleteOutcome::Deleted { .. }));
        fs::write(&shared, "手动恢复的新正文").unwrap();
        assert!(matches!(store.undo(), UndoOutcome::Undone));
        assert_eq!(fs::read_to_string(&shared).unwrap(), "手动恢复的新正文");

        let external = dir.join("external.md");
        fs::write(&external, "外部文件不得删除").unwrap();
        let index = store.tasks().len();
        store.add_finalized(&format!("legacy note:{}", external.display()));
        assert!(matches!(store.delete(index), DeleteOutcome::Deleted { .. }));
        assert_eq!(fs::read_to_string(&external).unwrap(), "外部文件不得删除");
        let literal = dir.join("notes/comment-only.md");
        fs::write(&literal, "非关联正文，不得误删").unwrap();
        let index = store.tasks().len();
        store.add_finalized(r#"legacy note:"keep note:comment-only.md literal""#);
        assert!(matches!(store.delete(index), DeleteOutcome::Deleted { .. }));
        assert!(
            literal.exists(),
            "quoted comments must not trigger body deletion"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
