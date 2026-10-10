//! Full-frame snapshot tests for every major mode/view.
//!
//! Each scene renders the real `ui::draw` into a fixed-size `TestBackend` and
//! emits two snapshots:
//!
//! * `*_text` — the visible character grid. Catches layout, content, and
//!   widget-placement regressions.
//! * `*_styled` — the same grid with inline `{fg=#hex bg=#hex mod=…}` tags.
//!   Catches styling regressions (priority colors, due-date buckets, cursor
//!   highlight, dim, bold) that the plain-text view would miss.
//!
//! Run `cargo insta review` after intentional UI changes to accept new
//! snapshots, or `INSTA_UPDATE=auto cargo test --test snapshots` to bulk-accept
//! during local iteration.

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use tuxedo::app::{
    App, BuilderField, CalendarState, CalendarTarget, Density, DraftOverlay, Mode,
    PriorityChooserState, RecurrenceBuilderState, SlashMenuState, View,
};
use tuxedo::config::Config;
use tuxedo::recurrence::RecUnit;
use tuxedo::sample;
use tuxedo::ui;

const COLS: u16 = 100;
const ROWS: u16 = 32;

/// File path used in every fixture. Hard-coded (not `temp_dir()`) so the
/// header line that displays it stays byte-identical across runs and
/// machines. The file is never actually written; `App::new` only stores it.
const FIXTURE_PATH: &str = "/tmp/tuxedo-snapshot.txt";

/// Config-file path for the settings-overlay fixture. Hard-coded for the
/// same reason as `FIXTURE_PATH`: `Config::path()` resolves `$HOME` at
/// runtime, which would otherwise bake the author's home directory into
/// the snapshot and break on any other machine (CI included).
const FIXTURE_CONFIG_PATH: &str = "/tmp/tuxedo-snapshot.toml";

fn make_app() -> App {
    // Seed the fixture file on disk so any snapshot test that exercises a
    // mutation (which calls `check_external_changes` and compares disk vs
    // `last_disk`) sees a consistent state. The file contents match what
    // we hand `App::new` as the in-memory body, so the comparison passes
    // without forcing a reload.
    std::fs::write(FIXTURE_PATH, sample::TODO_RAW).expect("seed fixture file");
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        sample::TODO_RAW.to_string(),
        "2026-05-06".to_string(),
        Config::default(),
    );
    app.config_path = Some(PathBuf::from(FIXTURE_CONFIG_PATH));
    // Compact density keeps each scene dense and stable: blank-line counts
    // shift with density, which would churn snapshots without adding signal.
    app.prefs.density = Density::Compact;
    app
}

fn render(app: &App) -> Buffer {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).expect("terminal init");
    terminal.draw(|f| ui::draw(f, app)).expect("draw frame");
    terminal.backend().buffer().clone()
}

/// Flatten a buffer to a plain character grid. Trailing whitespace per row is
/// preserved so width regressions show up as missing/extra padding columns.
fn buffer_to_text(buf: &Buffer) -> String {
    let cols = buf.area.width;
    let rows = buf.area.height;
    let mut out = String::with_capacity(usize::from(rows) * usize::from(cols + 1));
    for y in 0..rows {
        for x in 0..cols {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Flatten a buffer to text with inline style tags. Adjacent cells sharing
/// the same style are collapsed into one run; default styles are omitted.
///
/// Format: `{fg=#xxxxxx bg=#xxxxxx mod=bold,dim}…{/}`. Either attribute is
/// dropped when it's `Color::Reset`. The closing `{/}` only appears when the
/// run was non-default.
fn buffer_to_styled(buf: &Buffer) -> String {
    let cols = buf.area.width;
    let rows = buf.area.height;
    let mut out = String::new();

    for y in 0..rows {
        let mut x = 0u16;
        let mut current: Option<StyleKey> = None;
        while x < cols {
            let cell = &buf[(x, y)];
            let key = StyleKey::from_cell(cell);
            if Some(&key) != current.as_ref() {
                if current.as_ref().is_some_and(|k| !k.is_default()) {
                    out.push_str("{/}");
                }
                if !key.is_default() {
                    push_open_tag(&mut out, &key);
                }
                current = Some(key);
            }
            out.push_str(escape(cell.symbol()).as_str());
            x += 1;
        }
        if current.as_ref().is_some_and(|k| !k.is_default()) {
            out.push_str("{/}");
        }
        out.push('\n');
    }
    out
}

#[derive(Clone, PartialEq, Eq)]
struct StyleKey {
    fg: Color,
    bg: Color,
    modifier: Modifier,
}

impl StyleKey {
    fn from_cell(cell: &ratatui::buffer::Cell) -> Self {
        Self {
            fg: cell.fg,
            bg: cell.bg,
            modifier: cell.modifier,
        }
    }

    fn is_default(&self) -> bool {
        matches!(self.fg, Color::Reset)
            && matches!(self.bg, Color::Reset)
            && self.modifier.is_empty()
    }
}

fn push_open_tag(out: &mut String, key: &StyleKey) {
    out.push('{');
    let mut first = true;
    if !matches!(key.fg, Color::Reset) {
        out.push_str("fg=");
        out.push_str(&color_repr(key.fg));
        first = false;
    }
    if !matches!(key.bg, Color::Reset) {
        if !first {
            out.push(' ');
        }
        out.push_str("bg=");
        out.push_str(&color_repr(key.bg));
        first = false;
    }
    if !key.modifier.is_empty() {
        if !first {
            out.push(' ');
        }
        out.push_str("mod=");
        out.push_str(&modifier_repr(key.modifier));
    }
    out.push('}');
}

fn color_repr(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        Color::Reset => "reset".into(),
        // Themes are RGB-only today; keep a fallback so a future ANSI color
        // still produces a stable, readable token instead of `Debug` noise.
        other => format!("{:?}", other).to_lowercase(),
    }
}

fn modifier_repr(m: Modifier) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if m.contains(Modifier::BOLD) {
        parts.push("bold");
    }
    if m.contains(Modifier::DIM) {
        parts.push("dim");
    }
    if m.contains(Modifier::ITALIC) {
        parts.push("italic");
    }
    if m.contains(Modifier::UNDERLINED) {
        parts.push("underlined");
    }
    if m.contains(Modifier::REVERSED) {
        parts.push("reversed");
    }
    if m.contains(Modifier::SLOW_BLINK) {
        parts.push("slow_blink");
    }
    if m.contains(Modifier::RAPID_BLINK) {
        parts.push("rapid_blink");
    }
    if m.contains(Modifier::CROSSED_OUT) {
        parts.push("crossed_out");
    }
    if m.contains(Modifier::HIDDEN) {
        parts.push("hidden");
    }
    parts.join(",")
}

/// Escape brace literals so they don't collide with our `{tag}` syntax.
fn escape(s: &str) -> String {
    s.replace('{', "{{").replace('}', "}}")
}

/// Snapshot both the text grid and the styled grid for the given scene.
/// Uses two separate insta calls so a layout-only change doesn't force a
/// styling review (and vice versa).
fn snapshot_app(name: &str, app: &App) {
    let buf = render(app);
    insta::assert_snapshot!(format!("{name}_text"), buffer_to_text(&buf));
    insta::assert_snapshot!(format!("{name}_styled"), buffer_to_styled(&buf));
}

// ---------------------------------------------------------------------------
// Scenes
// ---------------------------------------------------------------------------

#[test]
fn mobile_layout_snapshots() {
    let app = make_app();
    for (name, cols, rows) in [
        ("list_mobile_portrait", 40, 60),
        ("list_narrow", 60, 30),
        ("list_mobile_keyboard", 40, 16),
    ] {
        insta::assert_snapshot!(name, render_text(&app, cols, rows));
    }
}

#[test]
fn mobile_panels_stack_and_keep_list_visible_after_resize() {
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        many_tasks_body(50),
        "2026-05-06".into(),
        Config::default(),
    );
    app.prefs.density = Density::Compact;
    while app.prefs.sort != tuxedo::app::Sort::File {
        app.cycle_sort();
    }
    app.cursor = 40;
    for (cols, rows) in [(40, 60), (60, 30), (100, 32), (40, 16), (40, 8), (40, 60)] {
        let text = render_text(&app, cols, rows);
        let lines = text.lines().collect::<Vec<_>>();
        assert!(
            text.contains("row-040"),
            "cursor vanished at {cols}x{rows}:\n{text}"
        );
        if cols < 100 && rows >= 16 {
            let filters = lines
                .iter()
                .position(|line| line.replace(' ', "").starts_with("项目"))
                .unwrap();
            let detail = lines
                .iter()
                .position(|line| line.replace(' ', "").starts_with("详情"))
                .unwrap();
            assert!(filters > 2 && detail > filters);
            assert!(detail - filters <= 3);
        }
        assert!(app.prefs.layout.left && app.prefs.layout.right);
    }
    app.prefs.layout.left = false;
    app.prefs.layout.right = false;
    let text = render_text(&app, 40, 30);
    assert!(
        !text
            .lines()
            .any(|line| line.replace(' ', "").starts_with("详情"))
    );
    assert!(!text.replace(' ', "").contains("项目"));
    assert!(text.contains("row-040"));
}

#[test]
fn mobile_filter_picker_keeps_active_chip_visible() {
    let raw = (0..15)
        .map(|i| format!("task +project-{i:02} @context-{i:02}\n"))
        .collect::<String>();
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        raw,
        "2026-05-06".into(),
        Config::default(),
    );
    app.set_project_filter(Some("project-14".into()));
    app.mode = Mode::PickProject;
    let text = render_text(&app, 40, 30);
    let line = text
        .lines()
        .find(|line| line.replace(' ', "").starts_with("项目"))
        .unwrap();
    assert!(line.contains("▸+project-14"), "{line}");
    assert!(line.contains('…'));
    app.set_context_filter(Some("context-14".into()));
    app.mode = Mode::PickContext;
    let text = render_text(&app, 40, 10); // Only one filter row fits.
    let line = text
        .lines()
        .find(|line| line.replace(' ', "").starts_with("场景"))
        .unwrap();
    assert!(line.contains("▸@context-14"), "{line}");

    app.saved_filters = vec![
        tuxedo::app::SavedFilter {
            name: "first".into(),
            query: "task".into(),
        },
        tuxedo::app::SavedFilter {
            name: "second".into(),
            query: "task".into(),
        },
    ];
    app.enter_pick_saved();
    app.pick_step(true);
    let text = render_text(&app, 40, 10);
    let line = text
        .lines()
        .find(|line| line.replace(' ', "").starts_with("已存"))
        .unwrap();
    assert!(line.contains("▸second"), "{line}");
    assert!(!line.contains("▸first"));
}

#[test]
fn mobile_empty_filters_do_not_reserve_rows_and_status_stays_short() {
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        "task without tags\n".into(),
        "2026-05-06".into(),
        Config::default(),
    );
    let text = render_text(&app, 40, 30);
    let detail = text
        .lines()
        .position(|line| line.replace(' ', "").starts_with("详情"))
        .unwrap();
    assert_eq!(detail, 20); // 29 body rows - 9 preview rows, no filter rows.
    let status = text.lines().last().unwrap().replace(' ', "");
    assert!(
        status.contains("1项") && status.contains("Enter详情"),
        "{status}"
    );
    assert!(!status.contains("tuxedo") && !status.contains("2026"));
    app.flash("short flash");
    assert_eq!(
        render_text(&app, 40, 30)
            .lines()
            .last()
            .unwrap()
            .matches("short flash")
            .count(),
        1
    );
    for (cols, rows) in [(0, 0), (1, 1), (20, 5), (40, 8)] {
        render_text(&app, cols, rows); // Tiny viewports must not panic.
    }
    app.prefs.layout.status_bar = false;
    assert!(!render_text(&app, 40, 30).replace(' ', "").contains("普通"));
    app.mode = Mode::Settings;
    assert!(
        render_text(&app, 40, 30)
            .lines()
            .next()
            .unwrap()
            .replace(' ', "")
            .contains("设置")
    );
}

#[test]
fn detail_popup_is_centered_opaque_and_keeps_background_outside() {
    let mut app = make_app();
    for (cols, rows) in [(100, 32), (160, 50), (40, 16)] {
        let mut terminal = Terminal::new(TestBackend::new(cols, rows)).unwrap();
        app.mode = Mode::Normal;
        let background = terminal.draw(|f| ui::draw(f, &app)).unwrap().buffer.clone();
        app.mode = Mode::Detail;
        app.detail_scroll.set(0);
        let actual = terminal.draw(|f| ui::draw(f, &app)).unwrap().buffer;
        let w = (cols - 4).min(100);
        let h = (rows - 5).min(32);
        let x = (cols - w) / 2;
        let y = (rows - 1 - h) / 2;
        let popup = ratatui::layout::Rect::new(x, y, w, h);
        assert_eq!(actual[(x, y)].symbol(), "┌");
        assert_eq!(actual[(x + w - 1, y + h - 1)].symbol(), "┘");
        assert_eq!(app.detail_page_height.get(), h - 2);
        assert_eq!(actual[(x + w - 1, y + h - 1)].bg, app.theme().panel);

        let mut isolated = Terminal::new(TestBackend::new(cols, rows)).unwrap();
        let expected = isolated
            .draw(|f| ui::detail::render_full(f, popup, &app))
            .unwrap()
            .buffer;
        for row in 0..rows - 1 {
            for col in 0..cols {
                if popup.contains((col, row).into()) {
                    assert_eq!(actual[(col, row)], expected[(col, row)]);
                } else {
                    assert_eq!(actual[(col, row)], background[(col, row)]);
                }
            }
        }
    }
    for (cols, rows) in [(0, 0), (1, 1), (4, 4), (20, 5)] {
        render_text(&app, cols, rows); // Tiny viewports must not panic.
    }
}

#[test]
fn full_detail_scroll_clamps_and_reaches_raw_record_after_resize() {
    let notes = (0..60)
        .map(|i| format!(" note:\"note-{i:02} 中文备注 long content\""))
        .collect::<String>();
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        format!("(A) Full task +work @home{notes}\n"),
        "2026-05-06".into(),
        Config::default(),
    );
    app.mode = Mode::Detail;
    let initial = render_text(&app, 40, 16);
    assert!(initial.contains("note-00"));
    app.detail_scroll.set(10);
    let scrolled = render_text(&app, 40, 16);
    assert_ne!(initial, scrolled);
    app.detail_scroll.set(u16::MAX);
    let last = render_text(&app, 40, 16);
    assert!(last.contains("note-59"));
    let offset = app.detail_scroll.get();
    assert!(offset > 10 && offset < u16::MAX);
    assert_eq!(app.detail_page_height.get(), 9);
    let wider = render_text(&app, 100, 32);
    assert_eq!(app.detail_page_height.get(), 25);
    assert!(wider.contains("note-59"));
    assert!(app.detail_scroll.get() < offset);
    app.detail_scroll.set(0);
    assert_eq!(initial, render_text(&app, 40, 16));
    app.prefs.layout.right = false;
    assert!(render_text(&app, 40, 16).contains("note-00"));
}

#[test]
fn list_default() {
    snapshot_app("list_default", &make_app());
}

#[test]
fn list_with_search() {
    let mut app = make_app();
    app.set_search("work".to_string());
    snapshot_app("list_with_search", &app);
}

#[test]
fn list_with_project_filter() {
    let mut app = make_app();
    app.set_project_filter(Some("work".to_string()));
    snapshot_app("list_with_project_filter", &app);
}

#[test]
fn list_grouped_by_due() {
    let mut app = make_app();
    // Default sort is Priority (groups by priority bucket); cycle once to
    // exercise the Due grouping path which has different bucket logic.
    app.cycle_sort();
    snapshot_app("list_grouped_by_due", &app);
}

#[test]
fn list_no_sidebars() {
    let mut app = make_app();
    app.prefs.layout.left = false;
    app.prefs.layout.right = false;
    snapshot_app("list_no_sidebars", &app);
}

#[test]
fn list_sidebar_empty_hints() {
    // Tasks present but none carry +project / @context tags — the sidebar
    // should fall back to the "tag with +project" / "tag with @context" hints
    // instead of leaving the PROJECTS / CONTEXTS sections blank.
    let body = "(A) Buy milk\n(B) Call mom\nWrite up notes\n";
    std::fs::write(FIXTURE_PATH, body).expect("seed fixture file");
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        body.to_string(),
        "2026-05-06".to_string(),
        Config::default(),
    );
    app.config_path = Some(PathBuf::from(FIXTURE_CONFIG_PATH));
    app.prefs.density = Density::Compact;
    snapshot_app("list_sidebar_empty_hints", &app);
}

#[test]
fn archive_view() {
    let mut app = make_app();
    app.set_view(View::Archive);
    snapshot_app("archive_view", &app);
}

#[test]
fn help_overlay() {
    let mut app = make_app();
    app.mode = Mode::Help;
    snapshot_app("help_overlay", &app);
}

#[test]
fn settings_overlay() {
    let mut app = make_app();
    app.mode = Mode::Settings;
    snapshot_app("settings_overlay", &app);
}

#[test]
fn command_palette_unfiltered() {
    let mut app = make_app();
    app.command_palette.open(Mode::Normal);
    app.mode = Mode::CommandPalette;
    snapshot_app("command_palette_unfiltered", &app);
}

#[test]
fn command_palette_filtered() {
    let mut app = make_app();
    app.command_palette.open(Mode::Normal);
    app.mode = Mode::CommandPalette;
    app.draft_set("归档".to_string());
    app.command_palette.refresh("归档");
    snapshot_app("command_palette_filtered", &app);
}

#[test]
fn command_palette_preserves_visual_selection() {
    // Open the palette mid-Visual with two rows ticked: the underlying list
    // must keep its checkboxes visible while the overlay is shown.
    let mut app = make_app();
    app.mode = Mode::Visual;
    app.selection.toggle(0);
    app.selection.toggle(1);
    app.command_palette.open(Mode::Visual);
    app.mode = Mode::CommandPalette;
    snapshot_app("command_palette_preserves_visual_selection", &app);
}

#[test]
fn insert_dialog() {
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert("(A) Buy milk +groceries @errands due:2026-05-10".to_string());
    snapshot_app("insert_dialog", &app);
}

#[test]
fn insert_dialog_after_nl_parse() {
    // Type a prose buffer, press Enter once: the NL pre-pass rewrites the
    // draft to canonical todo.txt and surfaces a flash asking the user to
    // confirm. Mode stays in Insert so the user can review/edit.
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert(
        "Pay rent monthly on the first of the month, show the todo 3 days before the due date. \
         It's part of project home and context bank"
            .to_string(),
    );
    let outcome = app.add_from_draft();
    // The snapshot captures the visible buffer + flash; checking it first
    // surfaces *what* changed if the rewrite drifts. The outcome assertion
    // runs after as a contract check on AddOutcome::Parsed — a regression
    // either way will fail the test.
    snapshot_app("insert_dialog_after_nl_parse", &app);
    assert_eq!(outcome, tuxedo::app::AddOutcome::Parsed);
}

#[test]
fn insert_slash_menu() {
    // Mirrors mockup 1: slash menu open after the user has typed text plus
    // tags and is now picking metadata via `/`.
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert("Schedule team offsite +work @phone /".to_string());
    // The `/` lives at the last byte; install the overlay state that
    // `maybe_open_slash_menu` would normally produce.
    let anchor = app.draft.text().len() - 1;
    app.draft
        .set_overlay(Some(DraftOverlay::SlashMenu(SlashMenuState {
            anchor,
            selected: 0,
        })));
    snapshot_app("insert_slash_menu", &app);
}

#[test]
fn insert_calendar_for_due() {
    // Mirrors mockup 2: calendar picker open after the user chose /due. The
    // focused date is one ahead of today so the focus/today highlights are
    // distinguishable in the snapshot.
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert("(A) Renew passport before summer trip +travel @errands".to_string());
    app.draft
        .set_overlay(Some(DraftOverlay::Calendar(CalendarState {
            target: CalendarTarget::Due,
            focused: chrono::NaiveDate::from_ymd_opt(2026, 5, 7).expect("static date"),
            anchor: None,
        })));
    snapshot_app("insert_calendar_for_due", &app);
}

#[test]
fn insert_recurrence_builder() {
    // Mirrors mockup 3: recurrence builder open after /rec.
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert("Water the plants +home".to_string());
    app.draft.set_overlay(Some(DraftOverlay::RecurrenceBuilder(
        RecurrenceBuilderState {
            interval: 1,
            unit: RecUnit::Week,
            strict: true,
            field: BuilderField::Interval,
            anchor: None,
        },
    )));
    snapshot_app("insert_recurrence_builder", &app);
}

#[test]
fn insert_priority_chooser() {
    let mut app = make_app();
    app.mode = Mode::Insert;
    app.draft_set_insert("Buy milk +groceries".to_string());
    app.draft
        .set_overlay(Some(DraftOverlay::PriorityChooser(PriorityChooserState {
            selected: 0,
        })));
    snapshot_app("insert_priority_chooser", &app);
}

#[test]
fn empty_state() {
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        String::new(),
        "2026-05-06".to_string(),
        Config::default(),
    );
    app.prefs.density = Density::Compact;
    app.prefs.layout.left = false;
    app.prefs.layout.right = false;
    snapshot_app("empty_state", &app);
}

#[test]
fn welcome_overlay() {
    // First-run prompt: empty backdrop, welcome box centered on top.
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        String::new(),
        "2026-05-06".to_string(),
        Config::default(),
    );
    app.prefs.density = Density::Compact;
    app.mode = Mode::Welcome;
    snapshot_app("welcome_overlay", &app);
}

/// Build a synthetic todo body with N rows so the list overflows any
/// reasonable viewport. Each row gets a unique label we can search for in the
/// rendered buffer.
fn many_tasks_body(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!(
            "2026-05-04 row-{:03} task body for scrolling +work @laptop\n",
            i
        ));
    }
    s
}

/// Render `app` into a fixed-size buffer and return its plain-text grid.
fn render_text(app: &App, cols: u16, rows: u16) -> String {
    let backend = TestBackend::new(cols, rows);
    let mut terminal = Terminal::new(backend).expect("terminal init");
    terminal.draw(|f| ui::draw(f, app)).expect("draw frame");
    buffer_to_text(terminal.backend().buffer())
}

#[test]
fn list_scrolls_to_keep_cursor_visible_when_below_fold() {
    // 50 rows of tasks rendered into a viewport that only fits a handful.
    // Without scrolling, advancing the cursor past the fold would leave the
    // active row off-screen even as the right-pane detail updated. With the
    // fix, the cursor's row text must appear in the rendered buffer.
    let mut app = App::new(
        PathBuf::from(FIXTURE_PATH),
        many_tasks_body(50),
        "2026-05-06".to_string(),
        Config::default(),
    );
    app.prefs.density = Density::Compact;
    app.prefs.layout.left = false;
    app.prefs.layout.right = false;
    // Switch to file-order sort so rows render flat (no priority/due groups
    // injecting extra header lines into the line-index math).
    while app.prefs.sort != tuxedo::app::Sort::File {
        app.cycle_sort();
    }

    let cursor_target = 40usize;
    app.cursor = cursor_target;
    let label = format!("row-{:03}", cursor_target);

    // Tiny viewport: with 12 rows total the body is well under 40 lines.
    let text = render_text(&app, 80, 12);
    assert!(
        text.contains(&label),
        "cursor row {label:?} should be visible in the scrolled viewport:\n{text}"
    );
}
