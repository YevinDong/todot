use std::io;

use super::types::{Density, Sort};
use crate::app::WeekStart;
use crate::config::Config;
use crate::theme::{self, ColorMode, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSlot {
    Fixed,
    Dark,
    Light,
}

impl ThemeSlot {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fixed => "固定／回退",
            Self::Dark => "深色",
            Self::Light => "浅色",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePrefs {
    pub auto: bool,
    pub fixed: usize,
    pub dark: usize,
    pub light: usize,
}

impl ThemePrefs {
    pub fn index(self, slot: ThemeSlot) -> usize {
        match slot {
            ThemeSlot::Fixed => self.fixed,
            ThemeSlot::Dark => self.dark,
            ThemeSlot::Light => self.light,
        }
    }

    pub fn active_slot(self, mode: Option<ColorMode>) -> ThemeSlot {
        match (self.auto, mode) {
            (true, Some(ColorMode::Dark)) => ThemeSlot::Dark,
            (true, Some(ColorMode::Light)) => ThemeSlot::Light,
            _ => ThemeSlot::Fixed,
        }
    }

    pub fn mode_label(self) -> &'static str {
        if self.auto { "跟随终端" } else { "固定" }
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub left: bool,
    pub right: bool,
    pub line_num: bool,
    pub status_bar: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            left: true,
            right: true,
            line_num: true,
            status_bar: true,
        }
    }
}

/// User-tunable preferences persisted to `Config`. Cycle/toggle methods return
/// the flash message for the caller to display, sidestepping any `&mut prefs`
/// + `&mut flash_state` borrow tangle on `App`.
#[derive(Debug, Clone)]
pub struct Prefs {
    theme_idx: usize,
    pub themes: ThemePrefs,
    pub density: Density,
    pub sort: Sort,
    pub layout: Layout,
    pub show_done: bool,
    pub show_future: bool,
    /// Metadata keys whose `key:value` tokens are hidden from task rows.
    /// Config-only (no in-app toggle); see `Config::hidden_keys`.
    pub hidden_keys: Vec<String>,
    pub week_start: WeekStart,
    /// Whether the create/edit dialog opens the recurrence builder overlay
    /// for `rec:` / `/rec`. Config-only (no in-app toggle); see
    /// `Config::recurrence_builder`.
    pub recurrence_builder: bool,
}

impl Prefs {
    pub fn from_config(cfg: Config) -> Self {
        let theme_idx = cfg
            .theme
            .as_deref()
            .and_then(|name| theme::all().iter().position(|t| t.name == name))
            .unwrap_or(0);
        let resolve = |name: &Option<String>, default: usize| {
            name.as_deref()
                .map(|name| {
                    theme::all()
                        .iter()
                        .position(|t| t.name == name)
                        .unwrap_or(theme_idx)
                })
                .unwrap_or(default)
        };
        Self {
            theme_idx,
            themes: ThemePrefs {
                auto: cfg.theme_auto.unwrap_or(false),
                fixed: theme_idx,
                dark: resolve(&cfg.theme_dark, 0),
                light: resolve(&cfg.theme_light, 1),
            },
            density: cfg.density.unwrap_or(Density::Comfortable),
            sort: cfg.sort.unwrap_or(Sort::Priority),
            layout: Layout {
                left: cfg.show_left.unwrap_or(true),
                right: cfg.show_right.unwrap_or(true),
                line_num: cfg.show_line_num.unwrap_or(true),
                status_bar: cfg.show_status_bar.unwrap_or(true),
            },
            show_done: cfg.show_done.unwrap_or(false),
            show_future: cfg.show_future.unwrap_or(false),
            hidden_keys: cfg.hidden_keys,
            week_start: cfg.week_start.unwrap_or(WeekStart::Sunday),
            recurrence_builder: cfg.recurrence_builder.unwrap_or(true),
        }
    }

    pub fn theme(&self) -> &'static Theme {
        let all = theme::all();
        all[self.theme_idx % all.len()]
    }

    pub fn theme_idx(&self) -> usize {
        self.theme_idx
    }

    /// Jump directly to a specific theme by index. Used by the screenshot
    /// example to render every theme; production code should call
    /// `cycle_theme` instead so the change persists with a flash message.
    pub fn set_theme_idx(&mut self, idx: usize) {
        self.theme_idx = idx % theme::all().len();
    }

    pub fn sort_label(&self) -> &'static str {
        match self.sort {
            Sort::Priority => "优先级",
            Sort::Due => "截止日期",
            Sort::File => "文件顺序",
        }
    }

    pub fn apply_terminal_mode(&mut self, mode: Option<ColorMode>) {
        self.set_theme_idx(self.themes.index(self.themes.active_slot(mode)));
    }

    pub fn set_slot_theme(&mut self, slot: ThemeSlot, idx: usize) {
        let idx = idx % theme::all().len();
        match slot {
            ThemeSlot::Fixed => self.themes.fixed = idx,
            ThemeSlot::Dark => self.themes.dark = idx,
            ThemeSlot::Light => self.themes.light = idx,
        }
        self.set_theme_idx(idx);
    }

    pub fn cycle_theme(&mut self, mode: Option<ColorMode>) -> String {
        let slot = self.themes.active_slot(mode);
        self.set_slot_theme(slot, self.themes.index(slot) + 1);
        format!("{}主题：{}", slot.label(), self.theme().name)
    }

    pub fn cycle_density(&mut self) -> String {
        self.density = match self.density {
            Density::Compact => Density::Comfortable,
            Density::Comfortable => Density::Cozy,
            Density::Cozy => Density::Compact,
        };
        format!(
            "行间距：{}",
            match self.density {
                Density::Compact => "紧凑",
                Density::Comfortable => "适中",
                Density::Cozy => "宽松",
            }
        )
    }

    pub fn cycle_sort(&mut self) -> String {
        self.sort = match self.sort {
            Sort::Priority => Sort::Due,
            Sort::Due => Sort::File,
            Sort::File => Sort::Priority,
        };
        format!("排序：{}", self.sort_label())
    }

    pub fn toggle_left(&mut self) {
        self.layout.left = !self.layout.left;
    }

    pub fn toggle_right(&mut self) {
        self.layout.right = !self.layout.right;
    }

    pub fn toggle_line_num(&mut self) {
        self.layout.line_num = !self.layout.line_num;
    }

    pub fn toggle_show_done(&mut self) {
        self.show_done = !self.show_done;
    }

    pub fn toggle_show_future(&mut self) {
        self.show_future = !self.show_future;
    }

    pub fn cycle_week_start(&mut self) -> String {
        self.week_start = match self.week_start {
            WeekStart::Sunday => WeekStart::Monday,
            WeekStart::Monday => WeekStart::Sunday,
        };
        format!(
            "每周起始日：{}",
            match self.week_start {
                WeekStart::Sunday => "周日",
                WeekStart::Monday => "周一",
            }
        )
    }

    /// Serialize configured slots, never the active or previewed theme.
    pub fn write_theme_config(&self, cfg: &mut Config) {
        let all = theme::all();
        cfg.theme = Some(all[self.themes.fixed].name.to_string());
        cfg.theme_auto = Some(self.themes.auto);
        cfg.theme_dark = Some(all[self.themes.dark].name.to_string());
        cfg.theme_light = Some(all[self.themes.light].name.to_string());
    }

    /// Load first so non-pref fields, including the capture token, survive.
    /// The caller reports errors inside the TUI without corrupting its output.
    pub fn save(&self) -> io::Result<()> {
        let mut cfg = Config::load();
        self.write_theme_config(&mut cfg);
        cfg.density = Some(self.density);
        cfg.sort = Some(self.sort);
        cfg.show_left = Some(self.layout.left);
        cfg.show_right = Some(self.layout.right);
        cfg.show_line_num = Some(self.layout.line_num);
        cfg.show_status_bar = Some(self.layout.status_bar);
        cfg.show_done = Some(self.show_done);
        cfg.show_future = Some(self.show_future);
        cfg.hidden_keys = self.hidden_keys.clone();
        cfg.save()
    }
}
