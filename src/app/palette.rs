//! Command palette catalog and filter. Maps every `Action` variant to a
//! human-readable label and its current keybinding, then filters by fuzzy
//! subsequence match against the label. Reuses `search::subseq_match_ci` so
//! the matching semantics stay identical to the `/` task search.
use super::types::Mode;
use crate::action::Action;
use crate::search::subseq_match_ci;

#[derive(Debug, Clone, Copy)]
pub struct PaletteEntry {
    pub label: &'static str,
    pub keys: &'static str,
    pub action: Action,
}

/// Every action that's meaningful to invoke from the palette. `ArmF` is
/// omitted: it only exists as the leader of `fp` / `fc`, both of which appear
/// here under their full names.
pub const ENTRIES: &[PaletteEntry] = &[
    PaletteEntry {
        label: "新建任务",
        keys: "n",
        action: Action::BeginAdd,
    },
    PaletteEntry {
        label: "编辑当前任务（普通模式）",
        keys: "e",
        action: Action::BeginEdit,
    },
    PaletteEntry {
        label: "编辑当前任务（输入模式）",
        keys: "i",
        action: Action::BeginEditInsert,
    },
    PaletteEntry {
        label: "切换完成状态",
        keys: "x",
        action: Action::ToggleComplete,
    },
    PaletteEntry {
        label: "删除任务",
        keys: "dd",
        action: Action::Delete,
    },
    PaletteEntry {
        label: "切换优先级",
        keys: "p",
        action: Action::CyclePriority,
    },
    PaletteEntry {
        label: "任务下移",
        keys: "J",
        action: Action::MoveTaskDown,
    },
    PaletteEntry {
        label: "任务上移",
        keys: "K",
        action: Action::MoveTaskUp,
    },
    PaletteEntry {
        label: "给当前任务添加项目",
        keys: "+",
        action: Action::BeginPromptProject,
    },
    PaletteEntry {
        label: "给当前任务添加或移除场景",
        keys: "c",
        action: Action::BeginPromptContext,
    },
    PaletteEntry {
        label: "复制整行",
        keys: "yy",
        action: Action::CopyLine,
    },
    PaletteEntry {
        label: "复制正文",
        keys: "yb",
        action: Action::CopyBody,
    },
    PaletteEntry {
        label: "打开任务笔记",
        keys: "o",
        action: Action::OpenNote,
    },
    PaletteEntry {
        label: "创建或打开任务笔记",
        keys: "O",
        action: Action::CreateOrOpenNote,
    },
    PaletteEntry {
        label: "撤销",
        keys: "u",
        action: Action::Undo,
    },
    PaletteEntry {
        label: "光标下移",
        keys: "j / ↓",
        action: Action::CursorDown,
    },
    PaletteEntry {
        label: "光标上移",
        keys: "k / ↑",
        action: Action::CursorUp,
    },
    PaletteEntry {
        label: "跳到首项",
        keys: "gg",
        action: Action::CursorTop,
    },
    PaletteEntry {
        label: "跳到末项",
        keys: "G",
        action: Action::CursorBottom,
    },
    PaletteEntry {
        label: "向下翻页",
        keys: "Ctrl-d",
        action: Action::HalfPageDown,
    },
    PaletteEntry {
        label: "向上翻页",
        keys: "Ctrl-u",
        action: Action::HalfPageUp,
    },
    PaletteEntry {
        label: "模糊搜索",
        keys: "/",
        action: Action::BeginSearch,
    },
    PaletteEntry {
        label: "按项目筛选",
        keys: "fp",
        action: Action::PickProject,
    },
    PaletteEntry {
        label: "按场景筛选",
        keys: "fc",
        action: Action::PickContext,
    },
    PaletteEntry {
        label: "选择已保存筛选",
        keys: "ff",
        action: Action::PickSavedFilter,
    },
    PaletteEntry {
        label: "保存搜索为筛选",
        keys: "fs",
        action: Action::SaveCurrentFilter,
    },
    PaletteEntry {
        label: "切换排序",
        keys: "S",
        action: Action::CycleSort,
    },
    PaletteEntry {
        label: "切换多选模式",
        keys: "v",
        action: Action::ToggleVisual,
    },
    PaletteEntry {
        label: "切换当前行选择",
        keys: "Space",
        action: Action::ToggleSelected,
    },
    PaletteEntry {
        label: "待办列表",
        keys: "l",
        action: Action::GoList,
    },
    PaletteEntry {
        label: "切换归档视图",
        keys: "a",
        action: Action::ToggleArchiveView,
    },
    PaletteEntry {
        label: "归档已完成任务",
        keys: "A",
        action: Action::ArchiveCompleted,
    },
    PaletteEntry {
        label: "显示已完成任务",
        keys: "H",
        action: Action::ToggleShowDone,
    },
    PaletteEntry {
        label: "显示未来任务",
        keys: "F",
        action: Action::ToggleShowFuture,
    },
    PaletteEntry {
        label: "开关筛选栏",
        keys: "[",
        action: Action::ToggleLeftPane,
    },
    PaletteEntry {
        label: "开关详情栏",
        keys: "]",
        action: Action::ToggleRightPane,
    },
    PaletteEntry {
        label: "选择主题",
        keys: "T",
        action: Action::OpenThemePicker,
    },
    PaletteEntry {
        label: "切换主题",
        keys: "",
        action: Action::CycleTheme,
    },
    PaletteEntry {
        label: "切换行间距",
        keys: "D",
        action: Action::CycleDensity,
    },
    PaletteEntry {
        label: "开关行号",
        keys: "L",
        action: Action::ToggleLineNum,
    },
    PaletteEntry {
        label: "打开帮助",
        keys: "?",
        action: Action::OpenHelp,
    },
    PaletteEntry {
        label: "打开设置",
        keys: ",",
        action: Action::OpenSettings,
    },
    PaletteEntry {
        label: "打开命令菜单",
        keys: ": / Ctrl-P",
        action: Action::OpenCommandPalette,
    },
    PaletteEntry {
        label: "显示手机录入二维码",
        keys: "s",
        action: Action::OpenShare,
    },
    PaletteEntry {
        label: "返回 / 清除",
        keys: "Esc",
        action: Action::EscapeStack,
    },
    PaletteEntry {
        label: "退出",
        keys: "q",
        action: Action::Quit,
    },
    PaletteEntry {
        label: "修改截止日期",
        keys: "r",
        action: Action::Reschedule,
    },
    PaletteEntry {
        label: "切换每周起始日",
        keys: "W",
        action: Action::ChangeWeekStart,
    },
    PaletteEntry {
        label: "展开任务详情",
        keys: "Enter",
        action: Action::OpenDetail,
    },
];

#[derive(Debug, Default, Clone)]
pub struct CommandPaletteState {
    /// Highlighted row in the *filtered* list. Reset to 0 whenever the user
    /// edits the search text so the highlight doesn't get stranded past the
    /// new result count.
    pub cursor: usize,
    /// Mode the user was in when they opened the palette. Restored on close
    /// so that opening the palette from Visual mode (with a selection) and
    /// cancelling — or running a visual-aware action like ToggleComplete —
    /// keeps the selection meaningful instead of silently dropping into
    /// Normal.
    prior_mode: Option<Mode>,
    /// Cached filter inputs and outputs. `refresh` recomputes only when the
    /// needle changes, so multiple call sites (Enter, Up/Down, render) can
    /// read `hits()` per frame without re-running the match each time.
    cached_needle: String,
    cached_hits: Vec<PaletteHit>,
}

impl CommandPaletteState {
    /// Snapshot the mode the palette is being opened from, reset the
    /// highlight, and seed the cache with the unfiltered list so the first
    /// frame doesn't have to fall through `refresh`.
    pub fn open(&mut self, prior: Mode) {
        self.cursor = 0;
        self.prior_mode = Some(prior);
        self.cached_needle.clear();
        self.cached_hits = filtered("");
    }

    /// Consume the snapshot taken in `open`. Defaults to `Normal` if the
    /// palette was somehow closed without a matching open — keeps the close
    /// path total instead of panicking.
    pub fn take_prior(&mut self) -> Mode {
        self.prior_mode.take().unwrap_or(Mode::Normal)
    }

    /// Read the snapshot without consuming it. Renderers use this to keep
    /// the underlying UI looking the same while the palette overlay is open.
    pub fn prior(&self) -> Option<Mode> {
        self.prior_mode
    }

    /// Currently visible hits, in rank order. Computed at most once per
    /// `refresh` call.
    pub fn hits(&self) -> &[PaletteHit] {
        &self.cached_hits
    }

    /// Recompute `hits` if `needle` differs from the cached needle and snap
    /// the highlight back to the top. Cheap no-op when the needle is the
    /// same (e.g., a cursor-move keystroke that doesn't change the draft).
    pub fn refresh(&mut self, needle: &str) {
        if self.cached_needle == needle {
            return;
        }
        self.cached_needle.clear();
        self.cached_needle.push_str(needle);
        self.cached_hits = filtered(needle);
        self.cursor = 0;
    }

    /// Move the highlight by `dir` rows with wrap-around. No-op when the
    /// filtered list is empty.
    pub fn step(&mut self, dir: i32) {
        let len = self.cached_hits.len();
        if len == 0 {
            return;
        }
        let cur = self.cursor.min(len - 1) as i32;
        let next = (cur + dir).rem_euclid(len as i32) as usize;
        self.cursor = next;
    }

    /// Action under the highlight, if any. Returns `None` when the filter
    /// produced no matches.
    pub fn current_action(&self) -> Option<Action> {
        let hit = self.cached_hits.get(self.cursor)?;
        ENTRIES.get(hit.entry_idx).map(|e| e.action)
    }
}

/// One filtered hit: the index into `ENTRIES`, plus the matched byte offsets
/// inside that entry's label (for highlighting).
#[derive(Debug, Clone)]
pub struct PaletteHit {
    pub entry_idx: usize,
    pub match_positions: Vec<usize>,
}

/// Filter `ENTRIES` against `needle`. Empty needle returns every entry in
/// declaration order. Non-empty needle returns only entries whose label
/// contains the needle as a case-insensitive subsequence, ranked by:
///   1. where the first match sits — byte 0 beats a word-boundary match beats
///      a mid-word match. ("arch" → "archive…" before "toggle archive…"
///      before "fuzzy search".)
///   2. tightness of the run (smaller span first).
///   3. declaration order, via the stable sort.
pub fn filtered(needle: &str) -> Vec<PaletteHit> {
    if needle.is_empty() {
        return ENTRIES
            .iter()
            .enumerate()
            .map(|(i, _)| PaletteHit {
                entry_idx: i,
                match_positions: Vec::new(),
            })
            .collect();
    }
    let mut hits: Vec<((u8, usize), PaletteHit)> = ENTRIES
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            subseq_match_ci(e.label, needle).map(|positions| {
                let rank = rank_hit(e.label, &positions);
                (
                    rank,
                    PaletteHit {
                        entry_idx: i,
                        match_positions: positions,
                    },
                )
            })
        })
        .collect();
    // Stable sort preserves declaration order within a rank tier.
    hits.sort_by_key(|(rank, _)| *rank);
    hits.into_iter().map(|(_, h)| h).collect()
}

fn rank_hit(label: &str, positions: &[usize]) -> (u8, usize) {
    // `positions` is non-empty here: subseq_match_ci returns Some only for
    // a fully-matched needle, and the empty-needle case returns earlier.
    let first = positions[0];
    let last = positions[positions.len() - 1];
    let span = last - first;
    let start_tier = if first == 0 {
        0
    } else if is_word_boundary(label, first) {
        1
    } else {
        2
    };
    (start_tier, span)
}

/// True when `byte` is the start of a word: position 0, or the previous
/// char is non-alphanumeric (whitespace, punctuation, …). `byte` is assumed
/// to fall on a char boundary, which it does for every position returned by
/// `subseq_match_ci`.
fn is_word_boundary(s: &str, byte: usize) -> bool {
    if byte == 0 {
        return true;
    }
    s[..byte]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_needle_returns_all_entries() {
        let hits = filtered("");
        assert_eq!(hits.len(), ENTRIES.len());
    }

    #[test]
    fn matches_subsequence_in_label() {
        let hits = filtered("归档");
        assert!(!hits.is_empty());
        for hit in hits {
            assert!(subseq_match_ci(ENTRIES[hit.entry_idx].label, "归档").is_some());
        }
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(filtered("zzzqqq").is_empty());
    }

    #[test]
    fn start_of_label_ranks_above_mid_label() {
        let hits = filtered("归档");
        assert_eq!(ENTRIES[hits[0].entry_idx].label, "归档已完成任务");
    }

    #[test]
    fn word_boundary_ranks_above_mid_word() {
        let boundary = "toggle archive view";
        let middle = "fuzzy search";
        assert!(
            rank_hit(boundary, &subseq_match_ci(boundary, "arch").unwrap())
                < rank_hit(middle, &subseq_match_ci(middle, "arch").unwrap())
        );
    }

    #[test]
    fn tighter_match_ranks_above_gappier_within_tier() {
        assert!(rank_hit("abcd", &[0, 3]) < rank_hit("a---d", &[0, 4]));
    }

    #[test]
    fn case_insensitive_match() {
        assert_eq!(
            subseq_match_ci("Archive", "ARCH"),
            subseq_match_ci("Archive", "arch")
        );
    }

    #[test]
    fn entries_cover_every_meaningful_action() {
        // `ArmF` is intentionally omitted (it's only a chord leader, not a
        // user-facing action). Every other Action variant must be reachable
        // from the palette.
        let actions: Vec<Action> = ENTRIES.iter().map(|e| e.action).collect();
        let required = [
            Action::Quit,
            Action::CursorDown,
            Action::CursorUp,
            Action::CursorTop,
            Action::CursorBottom,
            Action::HalfPageDown,
            Action::HalfPageUp,
            Action::BeginAdd,
            Action::BeginEdit,
            Action::BeginEditInsert,
            Action::ToggleComplete,
            Action::Delete,
            Action::CyclePriority,
            Action::MoveTaskDown,
            Action::MoveTaskUp,
            Action::BeginSearch,
            Action::OpenHelp,
            Action::OpenDetail,
            Action::OpenSettings,
            Action::OpenCommandPalette,
            Action::Undo,
            Action::ToggleVisual,
            Action::ToggleSelected,
            Action::GoList,
            Action::ToggleArchiveView,
            Action::ArchiveCompleted,
            Action::PickProject,
            Action::PickContext,
            Action::CycleSort,
            Action::BeginPromptProject,
            Action::BeginPromptContext,
            Action::ToggleLeftPane,
            Action::ToggleRightPane,
            Action::CycleTheme,
            Action::CycleDensity,
            Action::ToggleLineNum,
            Action::ToggleShowDone,
            Action::ToggleShowFuture,
            Action::CopyLine,
            Action::CopyBody,
            Action::EscapeStack,
            Action::OpenShare,
            Action::OpenThemePicker,
            Action::Reschedule,
        ];
        for a in required {
            assert!(actions.contains(&a), "missing palette entry for {a:?}");
        }
    }
}
