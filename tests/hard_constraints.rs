#[test]
fn hc_chroma_005_terminal_concern_does_not_broadcast_to_pty_inventory() {
    let theme_source = include_str!("../src/theme.rs");

    assert!(!theme_source.contains("/dev/pts"));
    assert!(!theme_source.contains("broadcast_terminal_colors"));
}

#[test]
fn hc_chroma_006_daemon_does_not_trigger_global_terminal_reload_files_or_pi_mode_sidecar() {
    let theme_source = include_str!("../src/theme.rs");

    assert!(!theme_source.contains("wezterm-reload"));
    assert!(!theme_source.contains("current-mode"));
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

#[test]
fn hc_chroma_012_geoclue_uses_system_bus_and_waits_for_location_updated() {
    let daemon_source = include_str!("../src/daemon.rs");
    let geoclue_source = include_str!("../src/geoclue.rs");

    assert!(daemon_source.contains("zbus::Connection::system().await?"));
    assert!(!daemon_source.contains("zbus::Connection::session().await?"));
    assert!(daemon_source.contains("receive_signal(\"LocationUpdated\")"));
    assert!(!daemon_source.contains("client.get_property(\"Location\")"));
    assert!(geoclue_source.contains("GeoclueLocationUpdateAwaiter"));
    assert!(geoclue_source.contains("GeoclueRootLocation"));
    assert!(geoclue_source.contains("GeoclueLocationStale"));
}

#[test]
fn hc_chroma_013_resume_reconciles_schedule_from_login1_prepare_for_sleep() {
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(daemon_source.contains("struct SleepTransitionWatcher"));
    assert!(daemon_source.contains("org.freedesktop.login1"));
    assert!(daemon_source.contains("org.freedesktop.login1.Manager"));
    assert!(daemon_source.contains("PrepareForSleep"));
    assert!(daemon_source.contains("deserialize::<bool>()"));
    assert!(daemon_source.contains("struct ResumeFromSleep"));
    assert!(daemon_source.contains("tell(ResumeSchedule)"));
}

#[test]
fn hc_chroma_014_schedule_reconcile_cancels_stale_deadline_chains() {
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(daemon_source.contains("schedule_generation"));
    assert!(daemon_source.contains("struct ScheduledScheduleEvaluation"));
    assert!(daemon_source.contains("generation: u64"));
    assert!(daemon_source.contains("self.schedule_generation = self.schedule_generation.saturating_add(1)"));
    assert!(daemon_source.contains("if message.generation == self.schedule_generation"));
    assert!(!daemon_source.contains("tell(EvaluateSchedule).send_after"));
}

#[test]
fn hc_chroma_015_resume_location_refresh_is_separate_from_time_reconcile() {
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(daemon_source.contains("struct ResumeSchedule"));
    assert!(daemon_source.contains("self.reconcile(generation, context).await;"));
    assert!(daemon_source.contains("POST_RESUME_LOCATION_REFRESH_DELAY"));
    assert!(daemon_source.contains("struct LocationRefreshDue"));
    assert!(daemon_source.contains("struct LocationRefreshCompleted"));
    assert!(daemon_source.contains("context.spawn(async move"));
    assert!(daemon_source.contains("let location = ScheduleEngine::current_location().await;"));
    assert!(daemon_source.contains("chroma-daemon accepted fresh GeoClue location"));
    assert!(daemon_source.contains("chroma-daemon recomputed solar schedule from fresh GeoClue location"));
    assert!(daemon_source.contains("Some(fresh_location) =>"));
    assert!(daemon_source.contains("fresh_location.refresh_delay_at"));
    assert!(daemon_source.contains("self.request_location_refresh(context, Duration::ZERO);"));
}

#[test]
fn hc_chroma_016_startup_reapplies_state_and_civil_axes_wait_for_location() {
    let daemon_source = include_str!("../src/daemon.rs");
    let schedule_source = include_str!("../src/schedule.rs");
    let state_source = include_str!("../src/state.rs");

    assert!(daemon_source.contains("struct ReapplyCurrentState"));
    assert!(daemon_source.contains("root.ask(ReapplyCurrentState)"));
    assert!(schedule_source.contains("theme: Option<ThemeMode>"));
    assert!(schedule_source.contains("warmth: Option<ScheduledWarmth>"));
    assert!(schedule_source.contains("brightness: Option<ScheduledBrightness>"));
    assert!(schedule_source.contains("fn waiting() -> Self"));
    assert!(schedule_source.contains("self.schedule.needs_geolocation() && self.location.is_none()"));
    assert!(state_source.contains("struct StoredLocation"));
    assert!(state_source.contains("const LOCATION_TABLE"));
    assert!(state_source.contains("struct RecordLocation"));
    assert!(state_source.contains("struct ReadStoredLocation"));
}

#[test]
fn hc_chroma_017_schedule_reconciliation_applies_only_changed_values() {
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(daemon_source.contains("theme != self.theme"));
    assert!(daemon_source.contains("kelvin != self.warmth"));
    assert!(daemon_source.contains("target_kelvin != self.warmth"));
    assert!(daemon_source.contains("percent != self.brightness"));
    assert!(daemon_source.contains("target_percent != self.brightness"));
}

#[test]
fn hc_chroma_018_schedule_projects_elapsed_ramp_time() {
    let schedule_source = include_str!("../src/schedule.rs");
    let daemon_source = include_str!("../src/daemon.rs");

    assert!(schedule_source.contains("Wall-clock schedule projection"));
    assert!(schedule_source.contains("RampedScheduleState::Transition"));
    assert!(schedule_source.contains("remaining_duration_at"));
    assert!(schedule_source.contains("interpolate_toward"));
    assert!(daemon_source.contains("WarmthApplication::RampFrom"));
    assert!(daemon_source.contains("BrightnessApplication::RampFrom"));
}

#[test]
fn hc_chroma_018_stale_location_refresh_retries_with_a_held_location() {
    let daemon_source = include_str!("../src/daemon.rs");
    let geoclue_source = include_str!("../src/geoclue.rs");

    assert!(daemon_source.contains("const LOCATION_RENEWAL_RETRY_DELAY: Duration = Duration::from_secs(10);"));
    assert!(daemon_source.contains("self.location_lease.has_held()"));
    assert!(daemon_source.contains("self.location_lease.renew(fresh_location);"));
    assert!(geoclue_source.contains("pub struct FreshLocationLease"));
    assert!(geoclue_source.contains("self.held.filter(|location| location.is_current_at(now))"));
}
