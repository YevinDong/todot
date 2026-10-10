use super::App;
use super::types::Mode;
use crate::core::{BulkCompleteOutcome, BulkDeleteOutcome};

impl App {
    /// Bulk-complete every task in the selection that isn't already done.
    /// Clears the selection and exits Visual mode on success. Recurring tasks
    /// also spawn their next instance (handled by the store).
    pub fn complete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let indices: Vec<usize> = self.selection.iter().collect();
        match self.store.complete_many(&indices) {
            BulkCompleteOutcome::Done { completed, spawned } => {
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(if spawned > 0 {
                    format!("完成 {completed} 项（新增 {spawned} 项重复任务）")
                } else {
                    format!("完成 {completed} 项")
                });
                self.recompute_visible();
                self.clamp_cursor();
            }
            BulkCompleteOutcome::NothingToComplete => {
                self.flash("没有可完成的任务");
                self.selection.clear();
                self.mode = Mode::Normal;
            }
            BulkCompleteOutcome::Aborted(r) => self.handle_reconcile_abort(r),
            BulkCompleteOutcome::Error(e) => self.flash(format!("完成任务失败：{e}")),
        }
    }

    /// Bulk-delete every task in the selection.
    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let indices: Vec<usize> = self.selection.iter().collect();
        match self.store.delete_many(&indices) {
            BulkDeleteOutcome::Done { deleted } => {
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(format!("删除 {deleted} 项"));
                self.recompute_visible();
                self.clamp_cursor();
            }
            BulkDeleteOutcome::CleanupFailed { error, .. } => {
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(error.to_string());
                self.recompute_visible();
                self.clamp_cursor();
            }
            BulkDeleteOutcome::Nothing => {
                self.selection.clear();
                self.mode = Mode::Normal;
            }
            BulkDeleteOutcome::Aborted(r) => self.handle_reconcile_abort(r),
            BulkDeleteOutcome::Error(e) => self.flash(format!("写入失败：{e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::build_app;

    #[test]
    fn complete_selected_clears_selection_and_flashes() {
        let mut app = build_app("a\nb\nc\n");
        app.selection.toggle(0);
        app.selection.toggle(2);
        app.mode = Mode::Visual;
        app.complete_selected();
        assert!(app.tasks()[0].done);
        assert!(!app.tasks()[1].done);
        assert!(app.tasks()[2].done);
        assert!(app.selection.is_empty());
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.flash_active(), Some("完成 2 项"));
    }

    #[test]
    fn complete_selected_reports_recurring_spawns() {
        let mut app = build_app("a\nPay rent due:2026-04-15 rec:+1m\nb\nWater plants rec:1w\n");
        app.refresh_today("2026-05-09".into());
        app.selection.toggle(1);
        app.selection.toggle(3);
        app.mode = Mode::Visual;
        app.complete_selected();
        assert_eq!(app.tasks().len(), 6);
        assert_eq!(app.flash_active(), Some("完成 2 项（新增 2 项重复任务）"));
    }

    #[test]
    fn delete_selected_removes_all_in_selection() {
        let mut app = build_app("a\nb\nc\nd\n");
        app.selection.toggle(1);
        app.selection.toggle(3);
        app.mode = Mode::Visual;
        app.delete_selected();
        assert_eq!(app.tasks().len(), 2);
        assert_eq!(app.tasks()[0].raw, "a");
        assert_eq!(app.tasks()[1].raw, "c");
        assert!(app.selection.is_empty());
        assert_eq!(app.mode, Mode::Normal);
    }
}
