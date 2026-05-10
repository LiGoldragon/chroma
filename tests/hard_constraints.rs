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

#[test]
fn hc_chroma_008_runtime_uses_kameo_not_hand_rolled_task_actors() {
    let daemon_source = include_str!("../src/daemon.rs");
    let theme_source = include_str!("../src/theme.rs");

    assert!(daemon_source.contains("impl Actor for ChromaRoot"));
    assert!(theme_source.contains("impl Actor for ThemeApplier"));
    assert!(!daemon_source.contains("tokio::spawn"));
    assert!(!theme_source.contains("tokio::spawn"));
    assert!(!theme_source.contains("unbounded_channel"));
    assert!(!daemon_source.contains("AbortHandle"));
    assert!(!theme_source.contains("AbortHandle"));
}

#[test]
fn hc_chroma_009_runtime_has_no_shared_mutex_between_actors() {
    let daemon_source = include_str!("../src/daemon.rs");
    let theme_source = include_str!("../src/theme.rs");
    let state_source = include_str!("../src/state.rs");

    for source in [daemon_source, theme_source, state_source] {
        assert!(!source.contains("Arc<Mutex"));
        assert!(!source.contains("std::sync::Mutex"));
        assert!(!source.contains("tokio::sync::Mutex"));
    }
}

#[test]
fn hc_chroma_010_ghostty_concern_uses_native_config_and_gtk_action_reload() {
    let theme_source = include_str!("../src/theme.rs");

    assert!(theme_source.contains("config.ghostty"));
    assert!(theme_source.contains("GhosttyConfigTemplates"));
    assert!(theme_source.contains("template_for"));
    assert!(theme_source.contains("read_to_string"));
    assert!(theme_source.contains("tokio::fs::write(directory.join(\"config.ghostty\")"));
    assert!(theme_source.contains("com.mitchellh.ghostty"));
    assert!(theme_source.contains("org.gtk.Actions"));
    assert!(theme_source.contains("reload-config"));
    assert!(!theme_source.contains("systemctl"));
    assert!(!theme_source.contains("ReloadUnit"));
    assert!(!theme_source.contains("app-com.mitchellh.ghostty.service"));
    assert!(!theme_source.contains("ghostty-reload"));
}

#[test]
fn hc_chroma_011_config_reload_uses_push_watcher_not_polling_loop() {
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(daemon_source.contains("struct ConfigWatcher"));
    assert!(daemon_source.contains("recommended_watcher"));
    assert!(daemon_source.contains("RecursiveMode::NonRecursive"));
    assert!(!daemon_source.contains("notify::PollWatcher"));
}
