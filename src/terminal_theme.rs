//! Non-blocking terminal queries, using Crossterm's single input reader.
use std::io::Write;
use std::time::{Duration, Instant};

use crossterm::event::Event;
use todot::app::App;
use todot::theme::ColorMode;

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct ThemeProbe {
    // false: semantic mode; true: OSC 11 background fallback.
    pending: Option<(Instant, bool)>,
    manual: bool,
}

impl ThemeProbe {
    pub fn deadline(&self) -> Option<Instant> {
        self.pending.map(|(deadline, _)| deadline)
    }

    pub fn start(&mut self, app: &mut App, out: &mut impl Write, now: Instant, manual: bool) {
        self.manual = manual;
        app.theme_detection_status = "检测中";
        self.pending = Some((now + RESPONSE_TIMEOUT, false));
        #[cfg(unix)]
        if out
            .write_all(b"\x1b[?996n")
            .and_then(|_| out.flush())
            .is_err()
        {
            self.fail(app);
        }
        #[cfg(not(unix))]
        {
            let _ = out;
            self.fail(app);
        }
    }

    pub fn tick(&mut self, app: &mut App, out: &mut impl Write, now: Instant) -> bool {
        let Some((deadline, background)) = self.pending else {
            return false;
        };
        if now < deadline {
            return false;
        }
        if background {
            self.fail(app);
        } else {
            self.pending = Some((now + RESPONSE_TIMEOUT, true));
            if out
                .write_all(b"\x1b]11;?\x1b\\")
                .and_then(|_| out.flush())
                .is_err()
            {
                self.fail(app);
            }
        }
        true
    }

    pub fn handle(&mut self, app: &mut App, event: &Event) -> bool {
        match *event {
            Event::ColorSchemeChanged(dark) => {
                // Late semantic replies still outrank a background estimate.
                self.pending = None;
                app.set_terminal_mode(
                    if dark {
                        ColorMode::Dark
                    } else {
                        ColorMode::Light
                    },
                    false,
                );
                true
            }
            Event::TerminalBackgroundColor(r, g, b) => {
                if matches!(self.pending, Some((_, true))) {
                    self.pending = None;
                    app.set_terminal_mode(ColorMode::from_background(r, g, b), true);
                }
                true
            }
            _ => false,
        }
    }

    fn fail(&mut self, app: &mut App) {
        self.pending = None;
        app.theme_detection_status = "检测失败（保留当前主题）";
        if self.manual {
            app.flash("无法检测终端深浅模式，已保留当前主题");
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use todot::config::Config;

    #[test]
    fn queries_are_bounded_preserve_theme_and_prefer_semantic_replies() {
        let mut app = App::new(
            "unused-theme-probe.txt".into(),
            "".into(),
            "2026-10-10".into(),
            Config {
                theme_auto: Some(true),
                theme_dark: Some("Nord".into()),
                theme_light: Some("Dawn".into()),
                ..Config::default()
            },
        );
        app.draft_set_insert("未保存输入".into());
        let mut probe = ThemeProbe::default();
        let mut output = Vec::new();
        let now = Instant::now();
        probe.start(&mut app, &mut output, now, true);
        assert_eq!(output, b"\x1b[?996n");
        assert!(!probe.tick(&mut app, &mut output, now));
        assert!(probe.tick(&mut app, &mut output, now + RESPONSE_TIMEOUT));
        assert!(output.ends_with(b"\x1b]11;?\x1b\\"));
        assert!(probe.handle(
            &mut app,
            &Event::TerminalBackgroundColor(65535, 65535, 65535)
        ));
        assert_eq!(app.theme().name, "Dawn");
        assert!(probe.handle(&mut app, &Event::ColorSchemeChanged(true)));
        assert_eq!(app.theme().name, "Nord");
        probe.handle(
            &mut app,
            &Event::TerminalBackgroundColor(65535, 65535, 65535),
        );
        assert_eq!(app.theme().name, "Nord");
        probe.start(&mut app, &mut output, now, true);
        probe.tick(&mut app, &mut output, now + RESPONSE_TIMEOUT);
        probe.tick(&mut app, &mut output, now + RESPONSE_TIMEOUT * 2);
        assert!(probe.deadline().is_none());
        assert_eq!(app.theme().name, "Nord");
        assert_eq!(app.draft.text(), "未保存输入");
        assert!(app.theme_detection_status.contains("检测失败"));
    }
}
