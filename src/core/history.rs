use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::Store;
use super::outcome::{Reconcile, UndoOutcome};
use crate::app::UNDO_LIMIT;
use crate::todo::Task;

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    pub tasks: Vec<Task>,
    pub bodies: Vec<(PathBuf, Arc<[u8]>)>,
}

#[derive(Debug, Default, Clone)]
pub struct History {
    stack: VecDeque<Snapshot>,
}

impl History {
    pub fn push(&mut self, tasks: Vec<Task>) {
        self.push_with_bodies(tasks, Vec::new());
    }

    pub(crate) fn push_with_bodies(&mut self, tasks: Vec<Task>, bodies: Vec<(PathBuf, Arc<[u8]>)>) {
        if self.stack.len() >= UNDO_LIMIT {
            self.stack.pop_front();
        }
        self.stack.push_back(Snapshot { tasks, bodies });
    }

    pub(crate) fn remember_bodies(&mut self, bodies: &[(PathBuf, Arc<[u8]>)], notes_dir: &Path) {
        for snapshot in &mut self.stack {
            for (path, bytes) in bodies {
                if snapshot.bodies.iter().any(|(saved, _)| saved == path) {
                    continue;
                }
                let referenced = snapshot.tasks.iter().any(|task| {
                    let Some(linked) = crate::note::linked_path(&task.raw, notes_dir) else {
                        return false;
                    };
                    let resolved = std::fs::canonicalize(&linked).or_else(|_| {
                        std::fs::canonicalize(linked.parent().unwrap_or(Path::new(".")))
                            .map(|parent| parent.join(linked.file_name().unwrap_or_default()))
                    });
                    resolved.as_ref().is_ok_and(|resolved| resolved == path)
                });
                if referenced {
                    snapshot.bodies.push((path.clone(), bytes.clone()));
                }
            }
        }
    }

    pub fn pop(&mut self) -> Option<Vec<Task>> {
        self.stack.pop_back().map(|snapshot| snapshot.tasks)
    }

    pub(crate) fn pop_snapshot(&mut self) -> Option<Snapshot> {
        self.stack.pop_back()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

impl Store {
    pub(crate) fn push_history(&mut self) {
        self.history.push(self.tasks.clone());
    }

    pub fn undo(&mut self) -> UndoOutcome {
        match self.reconcile() {
            Reconcile::Unchanged => {}
            other => return UndoOutcome::Aborted(other),
        }
        match self.history.pop_snapshot() {
            Some(snapshot) => {
                let _lock = match self.restore_deleted_bodies(&snapshot.bodies) {
                    Ok(lock) => lock,
                    Err(e) => {
                        self.history
                            .push_with_bodies(snapshot.tasks, snapshot.bodies);
                        return UndoOutcome::Error(e);
                    }
                };
                let current = std::mem::replace(&mut self.tasks, snapshot.tasks);
                match self.persist() {
                    Ok(()) => UndoOutcome::Undone,
                    Err(e) => {
                        let prev = std::mem::replace(&mut self.tasks, current);
                        self.history.push_with_bodies(prev, snapshot.bodies);
                        UndoOutcome::Error(e)
                    }
                }
            }
            None => UndoOutcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::build_store;

    #[test]
    fn history_evicts_fifo_at_undo_limit() {
        let mut store = build_store("a\n");
        for _ in 0..(UNDO_LIMIT + 5) {
            store.push_history();
        }
        assert_eq!(store.history.len(), UNDO_LIMIT);
    }

    #[test]
    fn failed_undo_restores_tasks_and_history() {
        let mut store = build_store("first\nsecond\n");
        assert!(matches!(
            store.move_tasks(&[(0, 1)]),
            crate::core::MoveOutcome::Moved
        ));
        let tmp_path = store.file_path.with_extension("tmp");
        let _ = std::fs::remove_file(&tmp_path);
        let _ = std::fs::remove_dir_all(&tmp_path);
        std::fs::create_dir(&tmp_path).expect("create blocking temp directory");

        assert!(matches!(store.undo(), UndoOutcome::Error(_)));
        assert_eq!(
            store
                .tasks()
                .iter()
                .map(|task| task.raw.as_str())
                .collect::<Vec<_>>(),
            ["second", "first"]
        );
        assert_eq!(store.history.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&store.file_path).expect("read todo.txt"),
            "second\nfirst\n"
        );

        std::fs::remove_dir(&tmp_path).expect("remove blocking temp directory");
    }
}
