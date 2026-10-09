use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, DialogInputMode, Mode, View};
use crate::ui::dialog::draft_cursor_spans;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let mut mode_label: std::borrow::Cow<'static, str> = match app.mode {
        Mode::Normal => "普通".into(),
        Mode::Insert => match app.draft.input_mode() {
            DialogInputMode::Normal => "普通",
            DialogInputMode::Insert => "输入",
        }
        .into(),
        Mode::Search => "搜索".into(),
        Mode::Visual => "多选".into(),
        Mode::Help => "帮助".into(),
        Mode::Detail => "详情".into(),
        Mode::Settings => "设置".into(),
        Mode::PromptProject => "项目".into(),
        Mode::PromptContext => "场景".into(),
        Mode::PickProject => "选择项目".into(),
        Mode::PickContext => "选择场景".into(),
        Mode::PromptRenameProject => "重命名项目".into(),
        Mode::PromptRenameContext => "重命名场景".into(),
        Mode::PickSavedFilter => "选择筛选".into(),
        Mode::PromptSaveFilter => "保存筛选".into(),
        Mode::CommandPalette => "命令".into(),
        Mode::Share => "手机录入".into(),
        Mode::PickTheme => "选择主题".into(),
        Mode::Welcome => "欢迎".into(),
    };
    if matches!(app.view, View::Archive) && app.mode != Mode::Detail {
        mode_label = "归档".into();
    }

    let hint = match app.mode {
        Mode::Insert => match app.draft.input_mode() {
            DialogInputMode::Normal => "h/l 移动 · w/b/e 跳词 · i/a 输入 · Enter 保存 · Esc 取消",
            DialogInputMode::Insert => "Enter 保存 · Esc 普通模式",
        },
        Mode::Visual => "空格 选择 · x 完成 · dd 删除 · Esc 取消",
        Mode::Help => "? 关闭帮助",
        Mode::Detail => "j/k 滚动 · PgUp/PgDn 翻页 · Esc 返回",
        Mode::Settings => "Esc 返回",
        Mode::PromptProject => "输入项目名称 · Enter 保存 · Esc 取消",
        Mode::PromptContext => "输入场景名称 · Enter 切换 · Esc 取消",
        Mode::PickProject => "j/k 选择项目 · r 重命名 · Enter 确认 · Esc 清除",
        Mode::PickContext => "j/k 选择场景 · r 重命名 · Enter 确认 · Esc 清除",
        Mode::PickSavedFilter => "j/k 选择筛选 · Enter 确认 · Esc 还原",
        Mode::PromptSaveFilter => "输入筛选名称 · Enter 保存 · Esc 取消",
        Mode::CommandPalette => "输入以筛选 · Enter 执行 · Esc 取消",
        Mode::Share => "扫描二维码 · 任意键关闭",
        Mode::Welcome => "c 创建 todo.txt · s 打开示例 · q 退出",
        _ => "j/k 移动 · n 新建 · x 完成 · / 搜索 · ? 帮助 · u 撤销 · q 退出",
    };

    if super::is_compact(frame.area()) {
        let hint = match app.mode {
            Mode::Detail => "j/k滚动 Esc返回",
            Mode::PickProject | Mode::PickContext | Mode::PickSavedFilter => "j/k选择 Enter确认",
            Mode::Insert => match app.draft.input_mode() {
                DialogInputMode::Insert => "Enter保存 Esc普通",
                DialogInputMode::Normal => "Enter保存 Esc取消",
            },
            Mode::Visual => "空格选择 x完成",
            Mode::Normal => "Enter详情 /搜索",
            _ => hint,
        };
        let text = if let Some(flash) = app.flash_active() {
            format!(" {mode_label} · {flash}")
        } else {
            let chord = app
                .chord
                .active()
                .map(|c| format!(" {c}…"))
                .unwrap_or_default();
            format!(
                " {mode_label}{chord} · {}项 · {hint}",
                app.visible_indices().len()
            )
        };
        frame.render_widget(
            Paragraph::new(super::ellipsize(&text, usize::from(area.width)))
                .style(Style::default().fg(theme.status_fg).bg(theme.statusbar)),
            area,
        );
        return;
    }

    if let Some(f) = app.flash_active() {
        mode_label = format!("{mode_label} · {f}").into();
    }
    let mut right_parts = Vec::new();
    if matches!(app.view, View::Archive) {
        right_parts.push(format!("{} 项归档", app.archive().len()));
    } else {
        right_parts.push(format!("{} 项待办", app.visible_indices().len()));
    }
    if !app.selection.is_empty() {
        right_parts.push(format!("{} 项已选", app.selection.len()));
    }
    right_parts.push(app.today().to_string());
    right_parts.push(concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")).to_string());
    // Track where the update suffix would slot in so we can paint it in the
    // accent color (the rest of the right text is dim).
    let update_suffix = app
        .update_available()
        .map(|tag| format!(" · ↑ {tag} (tuxedo update)"));
    let right_text = right_parts.join(" · ");

    // Append a chord indicator (e.g. " g…") so two-key sequences like gg/dd/fp
    // give visible feedback on the first press. Only shown while armed.
    let chord_suffix = app
        .chord
        .active()
        .map(|c| format!(" {c}…"))
        .unwrap_or_default();
    // Layout: mode chip on left, hint in middle, right text right-aligned.
    let chip_text = format!(" {mode_label}{chord_suffix} ");
    let chip_w = Span::raw(chip_text.as_str()).width() as u16;
    let update_w = update_suffix
        .as_deref()
        .map(|s| Span::raw(s).width() as u16)
        .unwrap_or(0);
    let right_w = Span::raw(right_text.as_str()).width() as u16 + update_w + 1;
    let middle_w = area.width.saturating_sub(chip_w).saturating_sub(right_w);

    let [chip_area, mid_area, right_area] = Layout::horizontal([
        Constraint::Length(chip_w),
        Constraint::Length(middle_w),
        Constraint::Length(right_w),
    ])
    .areas(area);

    let chip = Paragraph::new(Span::styled(
        chip_text,
        Style::default()
            .bg(theme.mode_bg)
            .fg(theme.mode_fg)
            .add_modifier(Modifier::BOLD),
    ))
    .style(Style::default().bg(theme.statusbar));
    frame.render_widget(chip, chip_area);

    let mid_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(hint, Style::default().fg(theme.status_fg)),
    ])
    .style(Style::default().bg(theme.statusbar));
    frame.render_widget(
        Paragraph::new(mid_line).style(Style::default().bg(theme.statusbar)),
        mid_area,
    );

    let right_line = if let Some(suffix) = update_suffix {
        Line::from(vec![
            Span::styled(right_text, Style::default().fg(theme.dim)),
            Span::styled(
                suffix,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().fg(theme.dim)),
        ])
        .style(Style::default().bg(theme.statusbar))
    } else {
        Line::from(Span::styled(
            format!("{right_text} "),
            Style::default().fg(theme.dim),
        ))
        .style(Style::default().bg(theme.statusbar))
    };
    frame.render_widget(
        Paragraph::new(right_line)
            .style(Style::default().bg(theme.statusbar))
            .right_aligned(),
        right_area,
    );
}

pub fn render_command_line(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let visible_count = app.visible_indices().len();
    let suggestion = format!("  {visible_count} 项匹配 · Enter 确认 · Esc 取消");
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            "/",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    spans.extend(draft_cursor_spans(
        app.draft.text(),
        app.draft.cursor(),
        theme.fg,
        theme.bg,
    ));
    spans.push(Span::styled(suggestion, Style::default().fg(theme.dim)));
    let line = Line::from(spans).style(Style::default().bg(theme.bg));
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.bg)),
        area,
    );
}
