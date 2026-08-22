//! Durable recovery contracts for Chroma visual state.

use chroma::{
    BrightnessPercent, KelvinTemperature, ReadStoredState, RecordTheme, RecordWarmth, StateStore, StoredThemeState,
    StoredVisualState, StoredWarmthState, ThemeMode,
};
use kameo::actor::Spawn;
use redb::{Database, TableDefinition};

fn kelvin(value: u16) -> KelvinTemperature {
    KelvinTemperature::new(value)
}

fn fallback_state() -> StoredVisualState {
    StoredVisualState {
        theme: ThemeMode::Dark,
        theme_state: StoredThemeState::new(ThemeMode::Dark, 0),
        warmth: Some(StoredWarmthState::settled(kelvin(4_500))),
        percent: BrightnessPercent::new(75),
    }
}

#[test]
fn mid_ramp_restart_keeps_intent_and_reprojects_before_relay_recovery() {
    let target = kelvin(2_700);
    let persisted = StoredWarmthState::settled(kelvin(4_500)).project_transition(kelvin(3_600), target);
    let restarted = StoredWarmthState::from_archive(&persisted.archive().expect("archive warmth state"))
        .expect("restore warmth state");

    assert_eq!(restarted.desired_kelvin(), target);
    assert_eq!(restarted.applied_kelvin(), Some(kelvin(4_500)));
    assert_eq!(restarted.projected_kelvin(), kelvin(3_600));
    assert!(restarted.is_transitioning());

    let recovered = restarted.project_transition(kelvin(3_200), target);
    assert_eq!(recovered.applied_kelvin(), Some(kelvin(4_500)), "projection is not a false relay acknowledgement");
    assert_eq!(recovered.projected_kelvin(), kelvin(3_200));
    assert!(recovered.is_transitioning());
}

#[test]
fn failed_relay_application_does_not_replace_last_confirmed_warmth() {
    let applied = kelvin(4_500);
    let target = kelvin(2_700);
    let state_after_failed_write = StoredWarmthState::settled(applied).request_set(target);

    assert_eq!(state_after_failed_write.desired_kelvin(), target);
    assert_eq!(state_after_failed_write.projected_kelvin(), target);
    assert_eq!(state_after_failed_write.applied_kelvin(), Some(applied));
    assert!(state_after_failed_write.requires_settle_at(target));

    let acknowledged = state_after_failed_write.record_applied(target, true);
    assert_eq!(acknowledged.applied_kelvin(), Some(target));
    assert!(!acknowledged.requires_settle_at(target));
}

#[test]
fn target_equality_never_skips_an_unfinished_scheduled_transition() {
    let target = kelvin(2_700);
    let unfinished = StoredWarmthState::settled(kelvin(4_500)).project_transition(kelvin(3_600), target);

    assert_eq!(unfinished.desired_kelvin(), target);
    assert!(unfinished.requires_transition_to(target));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn warmth_transition_state_survives_state_store_restart() {
    let directory = tempfile::tempdir().expect("create temporary state directory");
    let database_path = directory.path().join("state.redb");
    let expected = StoredWarmthState::settled(kelvin(4_500)).project_transition(kelvin(3_600), kelvin(2_700));

    let store = StateStore::spawn_in_thread(StateStore::open(&database_path).expect("open state store"));
    store.wait_for_startup().await;
    store.ask(RecordWarmth { state: expected }).await.expect("persist warmth transition");
    let _ = store.stop_gracefully().await;
    store.wait_for_shutdown().await;

    let restarted = StateStore::spawn_in_thread(StateStore::open(&database_path).expect("reopen state store"));
    restarted.wait_for_startup().await;
    let restored =
        restarted.ask(ReadStoredState { fallback: fallback_state() }).await.expect("read persisted visual state");
    assert_eq!(restored.warmth, Some(expected));
    let _ = restarted.stop_gracefully().await;
    restarted.wait_for_shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn absent_or_legacy_warmth_state_never_replays_a_stale_target() {
    const LEGACY_WARMTH_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("warmth");
    const CURRENT_KEY: &str = "current";

    let directory = tempfile::tempdir().expect("create temporary state directory");
    let database_path = directory.path().join("state.redb");
    let legacy_database = Database::create(&database_path).expect("create legacy state database");
    let legacy_target = kelvin(2_700).archive().expect("archive old single warmth value");
    let transaction = legacy_database.begin_write().expect("open legacy state transaction");
    {
        let mut table = transaction.open_table(LEGACY_WARMTH_TABLE).expect("open old warmth table");
        table.insert(CURRENT_KEY, legacy_target.as_slice()).expect("write old target");
    }
    transaction.commit().expect("commit old warmth target");
    drop(legacy_database);

    let store = StateStore::spawn_in_thread(StateStore::open(&database_path).expect("open current state store"));
    store.wait_for_startup().await;
    let restored = store
        .ask(ReadStoredState {
            fallback: StoredVisualState {
                theme: ThemeMode::Dark,
                theme_state: StoredThemeState::new(ThemeMode::Dark, 0),
                warmth: None,
                percent: BrightnessPercent::new(75),
            },
        })
        .await
        .expect("read current state without old warmth interpretation");

    assert_eq!(restored.warmth, None, "the old target is not a current physical warmth state");
    let _ = store.stop_gracefully().await;
    store.wait_for_shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn theme_snapshot_migrates_a_theme_only_archive_once_at_revision_zero() {
    const THEME_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("theme");
    const CURRENT_KEY: &str = "current";
    let directory = tempfile::tempdir().expect("create temporary state directory");
    let database_path = directory.path().join("state.redb");
    let database = Database::create(&database_path).expect("create old database");
    let old_theme = ThemeMode::Light.archive().expect("archive old theme");
    let transaction = database.begin_write().expect("open transaction");
    {
        let mut table = transaction.open_table(THEME_TABLE).expect("open theme table");
        table.insert(CURRENT_KEY, old_theme.as_slice()).expect("persist old theme only");
    }
    transaction.commit().expect("commit old state");
    drop(database);

    let store = StateStore::spawn_in_thread(StateStore::open(&database_path).expect("open current state store"));
    store.wait_for_startup().await;
    let restored =
        store.ask(ReadStoredState { fallback: fallback_state() }).await.expect("migrate and read theme state");
    assert_eq!(restored.theme_state, StoredThemeState::new(ThemeMode::Light, 0));
    store
        .ask(RecordTheme { state: StoredThemeState::new(ThemeMode::Dark, 1) })
        .await
        .expect("atomically replace snapshot");
    let reloaded = store.ask(ReadStoredState { fallback: fallback_state() }).await.expect("read replacement");
    assert_eq!(reloaded.theme_state, StoredThemeState::new(ThemeMode::Dark, 1));
    let _ = store.stop_gracefully().await;
    store.wait_for_shutdown().await;
}
