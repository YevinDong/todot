use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, ThemeSlot};
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let all = theme::all();
    let bg = Style::default().bg(theme.panel).fg(theme.fg);
    let selected = Style::default()
        .bg(theme.cursor)
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border).bg(theme.panel))
        .title(" todot · 主题 ")
        .style(bg);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [summary, list_area, footer] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(inner);
    let mut lines = vec![Line::raw(format!(
        " 模式：{}（m 切换）",
        app.prefs.themes.mode_label()
    ))];
    for slot in [ThemeSlot::Fixed, ThemeSlot::Dark, ThemeSlot::Light] {
        let active = slot == app.theme_pick_slot;
        lines.push(Line::styled(
            format!(
                " {}{}：{}",
                if active { "▶ " } else { "  " },
                slot.label(),
                all[app.prefs.themes.index(slot)].name
            ),
            if active { selected } else { bg },
        ));
    }
    lines.push(Line::raw(format!(" 检测：{}", app.theme_detection_status)));
    frame.render_widget(Paragraph::new(lines).style(bg), summary);

    let cur = app.prefs.themes.index(app.theme_pick_slot);
    let lines: Vec<_> = all
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Line::from(vec![Span::styled(
                format!(" {}{}", if i == cur { "▶ " } else { "  " }, t.name),
                if i == cur { selected } else { bg },
            )])
        })
        .collect();
    let scroll = cur.saturating_sub(usize::from(list_area.height.saturating_sub(1)));
    frame.render_widget(
        Paragraph::new(lines).style(bg).scroll((scroll as u16, 0)),
        list_area,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(" Tab 槽位 · m 模式 · j/k/T 预览"),
            Line::raw(" Enter 保存 · Esc 取消 · Ctrl+L 重测"),
        ])
        .style(bg.fg(theme.dim)),
        footer,
    );
}
