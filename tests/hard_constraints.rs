#[test]
fn hc_chroma_005_terminal_concern_does_not_broadcast_to_pty_inventory() {
    let theme_source = include_str!("../src/theme.rs");

    assert!(!theme_source.contains("/dev/pts"));
    assert!(!theme_source.contains("broadcast_terminal_colors"));
}

#[test]
fn hc_chroma_006_daemon_does_not_trigger_global_terminal_reload_files() {
    let theme_source = include_str!("../src/theme.rs");

    assert!(!theme_source.contains("wezterm-reload"));
}

#[test]
fn hc_chroma_007_cli_does_not_emit_live_terminal_palette_sequences() {
    let cli_source = include_str!("../src/bin/chroma.rs");
    let theme_source = include_str!("../src/theme.rs");

    assert!(!cli_source.contains("terminal_osc_sequence"));
    assert!(!cli_source.contains("IsTerminal"));
    assert!(!cli_source.contains("write_all"));
    assert!(!theme_source.contains("terminal_osc_sequence"));
    assert!(!theme_source.contains("\\x1b]4;"));
}
