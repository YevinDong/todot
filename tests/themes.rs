use todot::app::{App, Mode, Prefs, ThemeSlot};
use todot::config::Config;
use todot::theme::ColorMode;

#[test]
fn theme_slots_preserve_legacy_config_preview_cancel_and_runtime_detection() {
    let config = Config {
        theme: Some("Matrix".into()),
        theme_dark: Some("Nord".into()),
        theme_light: Some("Dawn".into()),
        ..Config::default()
    };
    let mut prefs = Prefs::from_config(config.clone());
    prefs.apply_terminal_mode(Some(ColorMode::Light));
    assert_eq!(prefs.theme().name, "Matrix", "legacy config stays fixed");
    prefs.themes.auto = true;
    prefs.apply_terminal_mode(None);
    assert_eq!(
        prefs.theme().name,
        "Matrix",
        "unknown mode uses fixed fallback"
    );
    prefs.apply_terminal_mode(Some(ColorMode::Dark));
    assert_eq!(prefs.theme().name, "Nord");
    prefs.apply_terminal_mode(Some(ColorMode::Light));
    assert_eq!(prefs.theme().name, "Dawn");
    let mut saved = config.clone();
    prefs.write_theme_config(&mut saved);
    assert_eq!(
        saved.theme.as_deref(),
        Some("Matrix"),
        "active light theme must not overwrite fixed theme"
    );
    assert_eq!(saved.theme_dark.as_deref(), Some("Nord"));
    assert_eq!(saved.theme_light.as_deref(), Some("Dawn"));

    saved.theme_dark = Some("No such theme".into());
    let mut missing = Prefs::from_config(saved);
    missing.apply_terminal_mode(Some(ColorMode::Dark));
    assert_eq!(missing.theme().name, "Matrix");
    missing.set_slot_theme(ThemeSlot::Light, missing.themes.dark);
    assert_eq!(
        missing.themes.light, missing.themes.dark,
        "same theme in both slots is allowed"
    );

    let mut app = App::new(
        "unused-theme-slots.txt".into(),
        "".into(),
        "2026-10-10".into(),
        config.clone(),
    );
    app.prefs.themes.auto = true;
    app.set_terminal_mode(ColorMode::Dark, false);
    app.mode = Mode::Settings;
    let original = app.prefs.themes;
    app.enter_pick_theme();
    assert_eq!(app.theme_pick_slot, ThemeSlot::Dark);
    app.pick_theme_step(true);
    assert_eq!(app.theme().name, "Matrix");
    app.set_terminal_mode(ColorMode::Light, true);
    assert_eq!(
        app.theme().name,
        "Matrix",
        "detection must not overwrite preview"
    );
    app.pick_theme_slot_step(true);
    assert_eq!(app.theme_pick_slot, ThemeSlot::Light);
    app.pick_theme_step(true);
    app.pick_theme_toggle_auto();
    app.pick_theme_cancel();
    assert_eq!(app.mode, Mode::Settings);
    assert_eq!(
        app.prefs.themes, original,
        "Esc restores every slot and mode"
    );
    assert_eq!(
        app.theme().name,
        "Dawn",
        "latest detection applies after cancel"
    );
    assert_eq!(app.terminal_mode, Some(ColorMode::Light));
    let mut changed = config;
    changed.theme_auto = Some(true);
    changed.theme_light = Some("Nord".into());
    app.reload_config(changed);
    assert_eq!(app.theme().name, "Nord", "reload preserves detected mode");
    assert_eq!(app.terminal_mode, Some(ColorMode::Light));

    assert_eq!(ColorMode::from_background(0, 0, 0), ColorMode::Dark);
    assert_eq!(
        ColorMode::from_background(65535, 65535, 65535),
        ColorMode::Light
    );
    assert_eq!(
        ColorMode::from_background(32896, 32896, 32896),
        ColorMode::Light
    );
}
