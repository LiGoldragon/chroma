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
