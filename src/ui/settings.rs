use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::ui::header;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.bg));

    let [header_area, _sp, body_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);

    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some("设置"),
            // title: None,
            // file: "settings",
            count: app.tasks().len(),
            sort: app.sort_label(),
            filter: None,
        },
    );

    let mut lines: Vec<Line> = Vec::new();
    let density = match app.prefs.density {
        crate::app::Density::Compact => "紧凑",
        crate::app::Density::Comfortable => "适中",
        crate::app::Density::Cozy => "宽松",
    };
    let on = |b: bool| if b { "开启" } else { "关闭" };

    let config_path = app
        .config_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "（不可用）".into());

    let items: &[(&str, Option<String>)] = &[
        ("文件", None),
        ("  待办文件", Some(app.file_path.display().to_string())),
        ("  配置文件", Some(config_path)),
        ("", Some("".into())),
        ("显示", None),
        ("  主题", Some(format!("{} ▾  （T 切换）", theme.name))),
        ("  行间距", Some(format!("{} ▾  （D 切换）", density))),
        (
            "  行号",
            Some(format!("{}  （L 开关）", on(app.prefs.layout.line_num))),
        ),
        ("  状态栏", Some(on(app.prefs.layout.status_bar).into())),
        (
            "  筛选侧栏",
            Some(format!("{}  （[ 开关）", on(app.prefs.layout.left))),
        ),
        (
            "  详情侧栏",
            Some(format!("{}  （] 开关）", on(app.prefs.layout.right))),
        ),
        (
            "  显示已完成任务",
            Some(format!("{}  （H 开关）", on(app.prefs.show_done))),
        ),
        (
            "  显示未来任务",
            Some(format!("{}  （F 开关）", on(app.prefs.show_future))),
        ),
        ("", Some("".into())),
        ("行为", None),
        (
            "  默认排序",
            Some(format!("{} （S 切换）", app.sort_label())),
        ),
        ("", Some("".into())),
        ("快捷键", None),
        ("  ", Some("按 ? 查看全部快捷键".into())),
    ];

    for (k, v) in items {
        match v {
            None => {
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        k.to_string(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            Some(val) if k.is_empty() => {
                lines.push(Line::raw(" "));
                let _ = val;
            }
            Some(val) => {
                let mut padded = k.to_string();
                let len = Span::raw(padded.as_str()).width();
                if len < 30 {
                    padded.push_str(&" ".repeat(30 - len));
                }
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(padded, Style::default().fg(theme.fg)),
                    Span::styled(val.clone(), Style::default().fg(theme.dim)),
                ]));
            }
        }
    }

    let para = Paragraph::new(lines).style(Style::default().bg(theme.bg).fg(theme.fg));
    frame.render_widget(para, body_area);
}
