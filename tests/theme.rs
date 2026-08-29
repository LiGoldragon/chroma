use chroma::{
    ApplyTheme, GhosttyConfigChange, GhosttyConfigFile, PiThemeControl, ThemeAdapters, ThemeApplier, ThemeAxis,
    ThemeConcern, ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule,
};
use kameo::actor::ActorRef;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::time::{Duration, timeout};

struct ThemeApplierFixture {
    temporary_directory: tempfile::TempDir,
}

impl ThemeApplierFixture {
    fn new() -> Self {
        Self { temporary_directory: tempfile::tempdir().expect("create tempdir") }
    }

    fn registry_directory(&self) -> PathBuf {
        self.temporary_directory.path().join("pi-live-theme.d")
    }

    fn socket_path(&self, name: &str) -> PathBuf {
        self.registry_directory().join(format!("{name}.sock"))
    }

    fn registry_entry_path(&self, name: &str) -> PathBuf {
        self.registry_directory().join(format!("{name}.path"))
    }

    fn register_socket(&self, name: &str, socket_path: &Path) -> PathBuf {
        let registry_entry_path = self.registry_entry_path(name);
        fs::create_dir_all(self.registry_directory()).expect("create Pi theme control registry directory");
        fs::write(&registry_entry_path, format!("{}\n", socket_path.display()))
            .expect("write Pi theme control registry entry");
        registry_entry_path
    }

    fn pi_axis(&self) -> ThemeAxis {
        let palette = ThemePalette::from_base16_slots([
            "#000000", "#111111", "#222222", "#333333", "#444444", "#555555", "#666666", "#777777", "#888888",
            "#999999", "#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd", "#eeeeee", "#ffffff",
        ]);
        ThemeAxis {
            concerns: vec![ThemeConcern::Pi],
            palettes: ThemePalettes { dark: palette.clone(), light: palette },
            adapters: ThemeAdapters::default(),
            font_point_size: 12,
            ghostty_config_templates: None,
            pi_theme_control: Some(PiThemeControl::from_registry_directory(self.registry_directory())),
            schedule: ThemeSchedule::Manual(ThemeMode::Dark),
        }
    }

    async fn apply_theme(&self, applier: &ActorRef<ThemeApplier>, mode: ThemeMode) {
        applier.ask(ApplyTheme { mode }).await.expect("theme enqueue succeeds");
    }
}

#[test]
fn dark_renders_as_dark() {
    assert_eq!(ThemeMode::Dark.as_str(), "dark");
    assert_eq!(format!("{}", ThemeMode::Dark), "dark");
}

#[test]
fn light_renders_as_light() {
    assert_eq!(ThemeMode::Light.as_str(), "light");
    assert_eq!(format!("{}", ThemeMode::Light), "light");
}

#[test]
fn toggled_inverts() {
    assert_eq!(ThemeMode::Dark.toggled(), ThemeMode::Light);
    assert_eq!(ThemeMode::Light.toggled(), ThemeMode::Dark);
}

#[test]
fn toggled_twice_is_identity() {
    assert_eq!(ThemeMode::Dark.toggled().toggled(), ThemeMode::Dark);
    assert_eq!(ThemeMode::Light.toggled().toggled(), ThemeMode::Light);
}

#[tokio::test]
async fn pi_theme_control_sends_theme_line_event_to_each_registered_socket() {
    let fixture = ThemeApplierFixture::new();
    fs::create_dir_all(fixture.registry_directory()).expect("create Pi theme control registry directory");
    let first_socket_path = fixture.socket_path("first");
    let second_socket_path = fixture.socket_path("second");
    let first_listener = UnixListener::bind(&first_socket_path).expect("bind first Pi theme control listener");
    let second_listener = UnixListener::bind(&second_socket_path).expect("bind second Pi theme control listener");
    fixture.register_socket("first", &first_socket_path);
    fixture.register_socket("second", &second_socket_path);
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Dark).await;

    let received = timeout(Duration::from_secs(1), async {
        tokio::join!(read_single_theme_message(first_listener), read_single_theme_message(second_listener))
    })
    .await
    .expect("both Pi sessions receive theme control messages");

    assert_eq!(received, ("dark\n".to_string(), "dark\n".to_string()));
    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
}

#[tokio::test]
async fn pi_registration_receives_current_mode_after_chroma_changed_with_no_peers() {
    let fixture = ThemeApplierFixture::new();
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Dark).await;

    fs::create_dir_all(fixture.registry_directory()).expect("create Pi theme control registry directory");
    let socket_path = fixture.socket_path("late-pi");
    let listener = UnixListener::bind(&socket_path).expect("bind late Pi listener");
    fixture.register_socket("late-pi", &socket_path);

    let received = timeout(Duration::from_secs(1), read_single_theme_message(listener))
        .await
        .expect("late Pi registration receives the current Chroma mode");

    assert_eq!(received, "dark\n");
    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
}

#[tokio::test]
async fn stale_pi_theme_control_registry_entry_is_cleaned_without_failing_theme_apply() {
    let fixture = ThemeApplierFixture::new();
    let missing_socket_path = fixture.socket_path("missing");
    let stale_entry_path = fixture.register_socket("missing", &missing_socket_path);
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Light).await;

    timeout(Duration::from_secs(1), async {
        loop {
            if !stale_entry_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stale Pi theme control registry entry is removed");

    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
}

#[tokio::test]
async fn missing_pi_theme_control_registry_directory_is_non_fatal_for_theme_apply() {
    let fixture = ThemeApplierFixture::new();
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Dark).await;

    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
}

async fn read_single_theme_message(listener: UnixListener) -> String {
    let (mut stream, _) = listener.accept().await.expect("accept connection");
    let mut message = String::new();
    stream.read_to_string(&mut message).await.expect("read Pi theme control message");
    message
}

#[tokio::test]
async fn ghostty_config_file_skips_equal_content_without_writing() {
    let fixture = ThemeApplierFixture::new();
    let directory = fixture.temporary_directory.path().join("ghostty");
    let config_path = directory.join("config.ghostty");
    fs::create_dir_all(&directory).expect("create Ghostty config directory");
    fs::write(&config_path, "background = #000000\n").expect("write current Ghostty config");

    let original_permissions = fs::metadata(&directory).expect("read Ghostty config directory metadata").permissions();
    let mut read_only_permissions = original_permissions.clone();
    read_only_permissions.set_mode(0o500);
    fs::set_permissions(&directory, read_only_permissions).expect("make Ghostty config directory unwritable");
    let change = GhosttyConfigFile::at(&config_path).replace_if_changed("background = #000000\n".into()).await;
    fs::set_permissions(&directory, original_permissions).expect("restore Ghostty config directory permissions");

    assert_eq!(change.expect("equal Ghostty config does not write"), GhosttyConfigChange::Unchanged);
    assert_eq!(fs::read_to_string(&config_path).expect("read unchanged Ghostty config"), "background = #000000\n");
}

#[tokio::test]
async fn ghostty_config_file_atomically_replaces_different_content() {
    let fixture = ThemeApplierFixture::new();
    let directory = fixture.temporary_directory.path().join("ghostty");
    let config_path = directory.join("config.ghostty");
    fs::create_dir_all(&directory).expect("create Ghostty config directory");
    fs::write(&config_path, "background = #000000\n").expect("write current Ghostty config");

    let change = GhosttyConfigFile::at(&config_path)
        .replace_if_changed("background = #faf5f0\n".into())
        .await
        .expect("replace different Ghostty config");

    assert_eq!(change, GhosttyConfigChange::Replaced);
    assert_eq!(fs::read_to_string(&config_path).expect("read replaced Ghostty config"), "background = #faf5f0\n");
}

#[test]
fn ghostty_palette_lines_emit_sixteen_indexed_entries() {
    let palette = ThemePalette::from_base16_slots([
        "#000000", "#111111", "#222222", "#333333", "#444444", "#555555", "#666666", "#777777", "#888888", "#999999",
        "#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd", "#eeeeee", "#ffffff",
    ]);

    let lines = palette.ghostty_palette_lines();

    assert!(lines.contains("palette = 0=#000000"));
    assert!(lines.contains("palette = 1=#888888"));
    assert!(lines.contains("palette = 15=#777777"));
    assert_eq!(lines.lines().count(), 16);
}
