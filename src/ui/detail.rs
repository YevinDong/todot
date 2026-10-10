use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;
use crate::theme::Theme;
use crate::todo::Task;
use crate::ui::task_row::{due_label, due_token_style, is_url_token, url_token_style};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));

    let task = app.cur_task();
    // Wrap to the actual pane width minus 1-char left padding and 1-char
    // safety margin on the right. Floor at 16 so a tiny pane still wraps.
    let wrap_w = (area.width as usize).saturating_sub(2).max(16);
    let lines = build_lines(theme, task, app.today(), wrap_w, app.task_body());
    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(theme.panel).fg(theme.fg));
    frame.render_widget(para, area);
}

pub fn render_compact(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));
    let [title, body, meta, tags, notes] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2.min(area.height.saturating_sub(3))),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    let style = Style::default().fg(theme.fg).bg(theme.panel);
    frame.render_widget(
        Paragraph::new(" 详情 · Enter 展开").style(style.fg(theme.dim)),
        title,
    );
    let Some(task) = app.cur_task() else {
        frame.render_widget(Paragraph::new(" （无任务）").style(style), body);
        return;
    };
    frame.render_widget(
        Paragraph::new(format!(" {}", crate::todo::body_only(&task.raw)))
            .wrap(Wrap { trim: false })
            .style(style),
        body,
    );
    let mut metadata = Vec::new();
    if let Some(p) = task.priority {
        metadata.push(Span::styled(
            format!(" ({p}) "),
            Style::default().fg(theme.priority_color(p)),
        ));
    }
    if let Some(due) = task.due.as_deref() {
        metadata.push(Span::styled(
            format!("截止 {due} {}", due_label(due, app.today())),
            due_token_style(task.done, due, app.today(), theme),
        ));
    }
    if task.done {
        metadata.push(Span::raw(format!(
            " 完成 {}",
            task.done_date.as_deref().unwrap_or("—")
        )));
    }
    frame.render_widget(Paragraph::new(Line::from(metadata)).style(style), meta);
    let labels = Line::from(vec![
        Span::styled(
            format!(
                " {}",
                task.projects
                    .iter()
                    .map(|p| format!("+{p}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Style::default().fg(theme.project),
        ),
        Span::styled(
            format!(
                " {}",
                task.contexts
                    .iter()
                    .map(|c| format!("@{c}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Style::default().fg(theme.context),
        ),
    ]);
    frame.render_widget(Paragraph::new(labels).style(style), tags);
    let content = match app.task_body() {
        Ok(Some(text)) => crate::note::display_text(&text),
        Ok(None) => task
            .notes
            .iter()
            .map(|n| format!(" 备注 {n}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => format!(" 正文读取失败：{e}"),
    };
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .style(style),
        notes,
    );
}

pub fn render_full(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(" 详情 · Esc 返回 ")
        .style(Style::default().fg(theme.fg).bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let para = Paragraph::new(build_lines(
        theme,
        app.cur_task(),
        app.today(),
        usize::from(inner.width.saturating_sub(2)).max(1),
        app.task_body(),
    ))
    .wrap(Wrap { trim: false })
    .style(Style::default().fg(theme.fg).bg(theme.panel));
    let max_scroll = para
        .line_count(inner.width)
        .saturating_sub(usize::from(inner.height))
        .min(usize::from(u16::MAX)) as u16;
    let scroll = app.detail_scroll.get().min(max_scroll);
    app.detail_scroll.set(scroll);
    app.detail_page_height.set(inner.height);
    frame.render_widget(para.scroll((scroll, 0)), inner);
}

fn build_lines<'a>(
    theme: &Theme,
    task: Option<&'a Task>,
    today: &'a str,
    wrap_w: usize,
    body: std::io::Result<Option<String>>,
) -> Vec<Line<'a>> {
    let mut rows: Vec<Line> = Vec::new();
    rows.push(line_panel(
        theme,
        vec![Span::styled(
            " 详情",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    let Some(t) = task else {
        rows.push(line_panel(
            theme,
            vec![Span::styled(" （无任务）", Style::default().fg(theme.dim))],
        ));
        return rows;
    };

    if matches!(&body, Ok(Some(_))) {
        rows.push(line_panel(
            theme,
            vec![Span::styled(
                format!(" {}", crate::todo::body_only(&t.raw)),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )],
        ));
    }
    let priority_value = if let Some(p) = t.priority {
        Span::styled(
            format!("({p})"),
            Style::default()
                .fg(theme.priority_color(p))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" 优先级    ", Style::default().fg(theme.dim)),
            priority_value,
        ],
    ));
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" 创建日期  ", Style::default().fg(theme.dim)),
            Span::styled(
                t.created_date.as_deref().unwrap_or("—"),
                Style::default().fg(theme.fg),
            ),
        ],
    ));
    if let Some(due) = &t.due {
        rows.push(line_panel(
            theme,
            vec![
                Span::styled(" 截止日期  ", Style::default().fg(theme.dim)),
                Span::styled(due.as_str(), Style::default().fg(theme.fg)),
                Span::raw("  "),
                Span::styled(due_label(due, today), Style::default().fg(theme.overdue)),
            ],
        ));
    }
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" 项目      ", Style::default().fg(theme.dim)),
            Span::styled(
                t.projects
                    .iter()
                    .map(|p| format!("+{p}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Style::default().fg(theme.project),
            ),
        ],
    ));
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" 场景      ", Style::default().fg(theme.dim)),
            Span::styled(
                t.contexts
                    .iter()
                    .map(|c| format!("@{c}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Style::default().fg(theme.context),
            ),
        ],
    ));

    // Rendering notes line by line
    if !t.notes.is_empty() {
        rows.push(line_panel(
            theme,
            vec![Span::styled(" 备注", Style::default().fg(theme.dim))],
        ));
        for note in &t.notes {
            let chunks = wrap_words(note, wrap_w.saturating_sub(4), false);
            for (i, chunk) in chunks.into_iter().enumerate() {
                let prefix = if i == 0 { "   - " } else { "     " };
                rows.push(line_panel(
                    theme,
                    vec![Span::styled(
                        format!("{prefix}{}", chunk.join(" ")),
                        Style::default().fg(theme.fg),
                    )],
                ))
            }
        }
    }

    if t.done {
        rows.push(line_panel(
            theme,
            vec![
                Span::styled(" 完成日期  ", Style::default().fg(theme.dim)),
                Span::styled(
                    t.done_date.as_deref().unwrap_or(""),
                    Style::default().fg(theme.done),
                ),
            ],
        ));
    }
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    rows.push(line_panel(
        theme,
        vec![Span::styled(
            " 原始记录",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    let mut state = RawWalk::default();
    for chunk in wrap_words(&t.raw, wrap_w, true) {
        let mut spans: Vec<Span> = vec![Span::raw(" ")];
        let mut words = chunk.into_iter();
        if let Some(first) = words.next() {
            spans.push(style_raw_token(first, t, today, theme, &mut state));
        }
        for w in words {
            spans.push(Span::raw(" "));
            spans.push(style_raw_token(w, t, today, theme, &mut state));
        }
        rows.push(line_panel(theme, spans));
    }
    match body {
        Ok(Some(text)) => {
            rows.push(line_panel(theme, vec![Span::raw(" ")]));
            rows.push(line_panel(
                theme,
                vec![Span::styled(
                    " 正文",
                    Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
                )],
            ));
            rows.push(line_panel(theme, vec![Span::raw(" ")]));
            for line in crate::note::display_text(&text).split('\n') {
                rows.push(line_panel(theme, vec![Span::raw(format!(" {line}"))]));
            }
        }
        Ok(None) => {}
        Err(e) => rows.push(line_panel(
            theme,
            vec![Span::styled(
                format!(" 正文读取失败：{e}"),
                Style::default().fg(theme.overdue),
            )],
        )),
    }
    rows
}

#[derive(Default)]
struct RawWalk {
    done_marker_consumed: bool,
    priority_consumed: bool,
}

fn style_raw_token<'a>(
    token: &'a str,
    task: &Task,
    today: &str,
    theme: &Theme,
    state: &mut RawWalk,
) -> Span<'a> {
    if task.done && !state.done_marker_consumed {
        state.done_marker_consumed = true;
        if token == "x" {
            return Span::styled(token, Style::default().fg(theme.done));
        }
    }
    if !state.priority_consumed
        && let Some(p) = task.priority
        && token.len() == 3
        && token.as_bytes()[0] == b'('
        && token.as_bytes()[1] == p as u8
        && token.as_bytes()[2] == b')'
    {
        state.priority_consumed = true;
        return Span::styled(
            token,
            Style::default()
                .fg(theme.priority_color(p))
                .add_modifier(Modifier::BOLD),
        );
    }
    if let Some(rest) = token.strip_prefix("due:") {
        return Span::styled(token, due_token_style(task.done, rest, today, theme));
    }
    if is_url_token(token) {
        return Span::styled(token, url_token_style(task.done, theme));
    }
    if token.len() > 1 && token.starts_with('+') {
        return Span::styled(token, Style::default().fg(theme.project));
    }
    if token.len() > 1 && token.starts_with('@') {
        return Span::styled(token, Style::default().fg(theme.context));
    }
    Span::styled(token, Style::default().fg(theme.fg))
}

fn line_panel<'a>(theme: &Theme, spans: Vec<Span<'a>>) -> Line<'a> {
    Line::from(spans).style(Style::default().bg(theme.panel))
}

/// Wrap `s` to roughly `width` graphemes, returning each output line as a
/// vector of borrowed words. Borrowing avoids the per-frame `String` alloc
/// that the previous `Vec<String>` form forced on every render.
fn wrap_words(s: &str, width: usize, hide_link: bool) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = Vec::new();
    let mut acc: Vec<&str> = Vec::new();
    let mut acc_len = 0;
    for word in crate::note::task_tokens(s)
        .filter_map(|(word, linked)| (!hide_link || !linked).then_some(word))
    {
        let wlen = Span::raw(word).width();
        let extra = if acc.is_empty() { 0 } else { 1 };
        if acc_len + wlen + extra > width && !acc.is_empty() {
            out.push(std::mem::take(&mut acc));
            acc_len = 0;
        }
        if !acc.is_empty() {
            acc_len += 1;
        }
        acc.push(word);
        acc_len += wlen;
    }
    if !acc.is_empty() {
        out.push(acc);
    }
    out
}
