# Crossterm 0.29.0 terminal-theme patch

Source: crates.io `crossterm` 0.29.0, MIT (see LICENSE). Source files are retained; example targets omitted from the manifest.

Local changes:
- `src/event.rs`: expose color-scheme and default-background replies as events.
- `src/event/sys/unix/parse.rs`: parse CSI ? 997 and OSC 11 (BEL/ST, 1–4 hex digits per channel), reject completed unknown private CSI reports instead of swallowing later input.

All replies pass through Crossterm's existing reader, preserving keys and bracketed paste. Unix terminals support this parser; Windows keeps manual/fixed-theme fallback.

Regression coverage: `tests/theme_terminal.py` runs the real binary in a PTY, including fragmented/late/malformed replies and typing while querying.

Remove this patch when a released, Ratatui-compatible Crossterm provides equivalent response events. Do not replace it with a second stdin reader.
