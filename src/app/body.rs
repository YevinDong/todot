use super::{AddOutcome, App, DialogInputMode, DraftState, Mode};
use crate::core::Reconcile;
use crate::{note, todo};
use std::fs;

#[derive(Debug, Default, Clone)]
pub struct BodyState {
    pub draft: DraftState,
    pub focused: bool,
    original_task: Option<String>,
    rel: Option<String>,
    original: Option<String>,
}

impl App {
    pub fn begin_task_edit(&mut self, insert: bool) -> bool {
        let Some(abs) = self.cur_task_index_in_tasks() else {
            self.flash("归档任务请先恢复，再编辑");
            return false;
        };
        let Some(task) = self.tasks().get(abs).cloned() else {
            return false;
        };
        let rel = note::note_rel_from_raw(&task.raw);
        let original = match note::linked_path(&task.raw, self.notes_dir()) {
            Some(path) => match note::read_body(&path) {
                Ok(text) => text,
                Err(e) => {
                    self.flash(format!("读取正文失败：{e}"));
                    return false;
                }
            },
            None => None,
        };
        let mut body = BodyState {
            original_task: Some(task.raw.clone()),
            rel,
            original: original.clone(),
            ..BodyState::default()
        };
        body.draft
            .set(original.unwrap_or_default(), DialogInputMode::Insert);
        self.body = body;
        self.selection.enter_edit(abs);
        self.draft.set(
            note::without_link(&task.raw),
            if insert {
                DialogInputMode::Insert
            } else {
                DialogInputMode::Normal
            },
        );
        self.mode = Mode::Insert;
        true
    }

    pub fn task_body(&self) -> std::io::Result<Option<String>> {
        // ponytail: read on redraw; cache by selected path/mtime if large notes cause lag.
        let Some(task) = self.cur_task() else {
            return Ok(None);
        };
        let Some(path) = note::linked_path(&task.raw, self.notes_dir()) else {
            return Ok(None);
        };
        note::read_body(&path)?
            .map(Some)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "正文文件不存在"))
    }

    /// Save the body first, then the task. A failed task write rolls the body back.
    /// ponytail: per-file atomic writes + rollback; add a journal if crash-atomic two-file saves are required.
    /// The draft stays intact on any error; no cross-file atomicity is claimed.
    pub fn submit_task_draft(&mut self) -> AddOutcome {
        match self.save_task_draft() {
            Ok(outcome) => outcome,
            Err(error) => {
                self.flash(error);
                AddOutcome::Invalid
            }
        }
    }

    fn save_task_draft(&mut self) -> Result<AddOutcome, String> {
        if self.draft.text().contains(['\n', '\r']) {
            return Err("标题只能一行，多行内容请放在正文中".into());
        }
        let empty = self.draft.text().trim().is_empty()
            || self.draft.text().trim() == self.filter().tag_seed().trim_end();
        if empty {
            return if self.body.draft.text().is_empty() {
                Ok(AddOutcome::Empty)
            } else {
                Err("请先填写标题，正文输入已保留".into())
            };
        }
        let reconcile = self.store.reconcile();
        if reconcile != Reconcile::Unchanged {
            self.handle_reconcile_abort(reconcile);
            return Err("任务文件发生变化，输入已保留；请取消后重新编辑".into());
        }
        if let Some(original) = &self.body.original_task
            && self
                .selection
                .editing()
                .and_then(|i| self.task_raw(i))
                .as_ref()
                != Some(original)
        {
            return Err("原任务发生变化，输入已保留；请取消后重新编辑".into());
        }
        if self.selection.editing().is_none() && self.normalize_add_draft() {
            return Ok(AddOutcome::Parsed);
        }
        if note::note_rel_from_raw(self.draft.text()).is_some() {
            return Err("正文路径由程序管理，请在正文框中输入内容".into());
        }
        let body = self.body.draft.text().to_owned();
        let changed = body != self.body.original.as_deref().unwrap_or("");
        let rel = match &self.body.rel {
            Some(rel) => Some(rel.clone()),
            None if !body.is_empty() => Some(note::new_body_rel().map_err(|e| e.to_string())?),
            None => None,
        };
        let path = rel.as_ref().map(|rel| {
            note::linked_path(&format!("note:{rel}"), self.notes_dir()).expect("nonempty note path")
        });
        let mut lock = None;
        if changed && let Some(path) = &path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建正文目录失败：{e}"))?;
            }
            let file =
                note::lock_dir(self.notes_dir()).map_err(|e| format!("锁定正文失败：{e}"))?;
            lock = Some(file);
            if note::read_body(path).map_err(|e| format!("读取正文失败：{e}"))?
                != self.body.original
            {
                return Err("正文已被外部修改，未覆盖；输入已保留".into());
            }
            todo::write_atomic(path, &body).map_err(|e| format!("保存正文失败：{e}"))?;
        }
        let draft = self.draft.clone();
        if let Some(rel) = &rel {
            self.draft.set(
                format!("{} note:{rel}", draft.text().trim()),
                draft.input_mode(),
            );
        }
        let outcome = if self.selection.editing().is_some() {
            self.save_edit()
        } else {
            self.add_from_draft()
        };
        self.draft = draft;
        if outcome != AddOutcome::Saved
            && changed
            && let Some(path) = &path
        {
            // Do not overwrite an uncooperative external editor's concurrent write.
            let rollback = match note::read_body(path) {
                Ok(Some(current)) if current == body => match &self.body.original {
                    Some(original) => todo::write_atomic(path, original),
                    None => fs::remove_file(path),
                },
                _ => Err(std::io::Error::other("正文又被其他程序修改")),
            };
            if let Err(e) = rollback {
                return Err(format!("标题未保存，正文回滚失败：{e}；正文和输入均保留"));
            }
        }
        drop(lock);
        Ok(outcome)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn body_roundtrips_without_parsing_and_failed_saves_preserve_both_files() {
        let dir = std::env::temp_dir().join(format!("todot-body-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("todo.txt");
        fs::write(&path, "").unwrap();
        let mut app = App::new(
            path.clone(),
            "".into(),
            "2026-10-10".into(),
            Config {
                notes_dir: Some(dir.join("notes").to_string_lossy().into_owned()),
                ..Config::default()
            },
        );
        let body =
            "# 正文\n\n  缩进 +不是标签 @不是场景 due:2030-01-01\n```json\n{\"正文\": 1}\n```\n";
        app.draft_set_insert("中文标题 +work".into());
        app.body.draft.set(body.into(), DialogInputMode::Insert);
        app.mode = Mode::Insert;
        assert_eq!(app.submit_task_draft(), AddOutcome::Saved);
        assert_eq!(app.tasks().len(), 1);
        assert_eq!(app.tasks()[0].projects, ["work"]);
        assert!(app.tasks()[0].due.is_none());
        let raw = app.tasks()[0].raw.clone();
        let note_path = note::linked_path(&raw, app.notes_dir()).unwrap();
        assert_eq!(app.task_body().unwrap().as_deref(), Some(body));
        assert!(app.begin_task_edit(true));
        assert!(!app.draft.text().contains("note:"));
        app.draft_set_insert("改名 +work".into());
        app.body.draft.insert_char('尾');
        let edited = app.body.draft.text().to_owned();
        fs::create_dir(path.with_extension("tmp")).unwrap();
        assert_eq!(app.submit_task_draft(), AddOutcome::Invalid);
        assert_eq!(app.draft.text(), "改名 +work");
        assert_eq!(app.body.draft.text(), edited);
        assert_eq!(app.tasks()[0].raw, raw);
        assert_eq!(fs::read_to_string(&note_path).unwrap(), body);
        fs::remove_dir(path.with_extension("tmp")).unwrap();
        assert_eq!(app.submit_task_draft(), AddOutcome::Saved);
        assert_eq!(
            note::linked_path(&app.tasks()[0].raw, app.notes_dir()).unwrap(),
            note_path
        );
        assert!(app.begin_task_edit(true));
        app.body.draft.insert_char('新');
        fs::write(&note_path, "外部编辑内容").unwrap();
        assert_eq!(app.submit_task_draft(), AddOutcome::Invalid);
        assert_eq!(fs::read_to_string(&note_path).unwrap(), "外部编辑内容");
        assert!(app.body.draft.text().ends_with('新'));
        fs::remove_dir_all(&dir).unwrap();
    }
}
