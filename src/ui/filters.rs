use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Filter, ordered_unique};
use crate::core::filter;
use crate::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));

    let projects = ordered_unique(app.tasks(), |t| &t.projects);
    let contexts = ordered_unique(app.tasks(), |t| &t.contexts);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            " 筛选",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    lines.push(line_pad(theme, vec![Span::raw(" ")]));
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            " 项目",
            Style::default()
                .fg(theme.project)
                .add_modifier(Modifier::BOLD),
        )],
    ));
    if projects.is_empty() {
        lines.push(hint_row(theme, "+project", theme.project));
    } else {
        for (name, count) in &projects {
            let active = app.filter.project.as_deref() == Some(name.as_str());
            lines.push(filter_row(theme, "+", name, *count, active, theme.project));
        }
    }
    lines.push(line_pad(theme, vec![Span::raw(" ")]));
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            " 场景",
            Style::default()
                .fg(theme.context)
                .add_modifier(Modifier::BOLD),
        )],
    ));
    if contexts.is_empty() {
        lines.push(hint_row(theme, "@context", theme.context));
    } else {
        for (name, count) in &contexts {
            let active = app.filter.context.as_deref() == Some(name.as_str());
            lines.push(filter_row(theme, "@", name, *count, active, theme.context));
        }
    }

    let saved = app.saved_filters();
    if !saved.is_empty() {
        lines.push(line_pad(theme, vec![Span::raw(" ")]));
        lines.push(line_pad(
            theme,
            vec![Span::styled(
                " 已保存筛选",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )],
        ));
        let today = app.today();
        let default_filter = Filter::default();
        for f in saved {
            let active = app.filter().search == f.query;
            // Same predicate the list view uses, so a saved `due:` query
            // counts by date range instead of its own literal text.
            let needle = filter::resolve_needle(&f.query, today);
            let count = app
                .tasks()
                .iter()
                .filter(|t| {
                    !t.done && filter::passes_user_filter(t, &default_filter, Some(&needle))
                })
                .count();
            lines.push(filter_row(theme, "", &f.name, count, active, theme.accent));
        }
    }

    let para = Paragraph::new(lines).style(Style::default().bg(theme.panel).fg(theme.fg));
    frame.render_widget(para, area);
}

pub fn compact_height(app: &App) -> u16 {
    u16::from(
        app.tasks()
            .iter()
            .any(|t| !t.done && !t.projects.is_empty()),
    ) + u16::from(
        app.tasks()
            .iter()
            .any(|t| !t.done && !t.contexts.is_empty()),
    ) + u16::from(!app.saved_filters().is_empty())
}

pub fn render_compact(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));
    let projects = ordered_unique(app.tasks(), |t| &t.projects);
    let contexts = ordered_unique(app.tasks(), |t| &t.contexts);
    let mut lines = Vec::new();
    for (label, sigil, entries, active, color) in [
        (
            "项目",
            "+",
            &projects,
            app.filter().project.as_deref(),
            theme.project,
        ),
        (
            "场景",
            "@",
            &contexts,
            app.filter().context.as_deref(),
            theme.context,
        ),
    ] {
        if !entries.is_empty() {
            let chips = entries
                .iter()
                .map(|(name, count)| {
                    (
                        format!("{sigil}{name}({count})"),
                        active == Some(name.as_str()),
                    )
                })
                .collect::<Vec<_>>();
            lines.push(compact_row(theme, label, &chips, area.width, color));
        }
    }
    if !app.saved_filters().is_empty() {
        let chips = app
            .saved_filters()
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let active = if app.mode == crate::app::Mode::PickSavedFilter {
                    i == app.saved_pick_idx
                } else {
                    app.filter().search == f.query
                };
                (f.name.clone(), active)
            })
            .collect::<Vec<_>>();
        lines.push(compact_row(theme, "已存", &chips, area.width, theme.accent));
    }
    // On very short screens, keep the picker category visible first.
    let selected_row = match app.mode {
        crate::app::Mode::PickContext => usize::from(!projects.is_empty()),
        crate::app::Mode::PickSavedFilter => lines.len().saturating_sub(1),
        _ => 0,
    };
    if usize::from(area.height) < lines.len() && selected_row < lines.len() {
        lines.swap(0, selected_row);
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme.fg).bg(theme.panel)),
        area,
    );
}

fn compact_row(
    theme: &Theme,
    label: &str,
    chips: &[(String, bool)],
    width: u16,
    color: ratatui::style::Color,
) -> Line<'static> {
    let prefix = format!(" {label} ");
    let mut remaining = usize::from(width).saturating_sub(Span::raw(&prefix).width());
    let mut spans = vec![Span::styled(prefix, Style::default().fg(theme.dim))];
    // Start at the active chip (or one neighbor before it if both fit).
    let active = chips.iter().position(|(_, active)| *active).unwrap_or(0);
    let mut start = active;
    if active > 0
        && Span::raw(&chips[active - 1].0).width() + Span::raw(&chips[active].0).width() + 6
            <= remaining
    {
        start -= 1;
    }
    if start > 0 && remaining > 2 {
        spans.push(Span::raw("… "));
        remaining -= 2;
    }
    for (i, (text, active)) in chips.iter().enumerate().skip(start) {
        let tail = if i + 1 < chips.len() { 2 } else { 0 };
        let chip = format!("{}{text}", if *active { "▸" } else { "" });
        let chip_width = Span::raw(&chip).width();
        if i > start && chip_width > remaining.saturating_sub(tail) {
            if remaining > 0 {
                spans.push(Span::raw("…"));
            }
            break;
        }
        let clipped = super::ellipsize(&chip, remaining.saturating_sub(tail));
        let used = Span::raw(&clipped).width();
        spans.push(Span::styled(
            clipped,
            Style::default()
                .fg(color)
                .bg(if *active { theme.selected } else { theme.panel }),
        ));
        remaining = remaining.saturating_sub(used);
        if used < chip_width {
            break;
        }
        if remaining <= tail {
            if tail > 0 && remaining >= 2 {
                spans.push(Span::raw(" …"));
            }
            break;
        }
        spans.push(Span::raw(" "));
        remaining = remaining.saturating_sub(1);
    }
    Line::from(spans)
}

fn filter_row<'a>(
    theme: &Theme,
    sigil: &str,
    name: &'a str,
    count: usize,
    active: bool,
    sigil_color: ratatui::style::Color,
) -> Line<'a> {
    let bg = if active { theme.selected } else { theme.panel };
    let prefix = if active { "▸ " } else { "  " };
    let mut padded = format!("{}{}", sigil, name);
    let width = Span::raw(padded.as_str()).width();
    if width < 16 {
        let pad = 16 - width;
        padded.push_str(&" ".repeat(pad));
    }
    Line::from(vec![
        Span::raw(" "),
        Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
        Span::styled(padded, Style::default().fg(sigil_color)),
        Span::styled(format!("{:>3}", count), Style::default().fg(theme.dim)),
    ])
    .style(Style::default().bg(bg))
}

fn hint_row<'a>(theme: &Theme, token: &'a str, token_color: ratatui::style::Color) -> Line<'a> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled("添加标签 ", Style::default().fg(theme.dim)),
        Span::styled(token, Style::default().fg(token_color)),
    ])
    .style(Style::default().bg(theme.panel))
}

fn line_pad<'a>(theme: &Theme, spans: Vec<Span<'a>>) -> Line<'a> {
    Line::from(spans).style(Style::default().bg(theme.panel))
}
