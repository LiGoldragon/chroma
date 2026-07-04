use chroma::{
    ApplyTheme, PiThemeControl, ThemeAdapters, ThemeApplier, ThemeAxis, ThemeConcern, ThemeMode, ThemePalette,
    ThemePalettes, ThemeSchedule,
};
use kameo::actor::ActorRef;
use nota_next::{NotaDecode, NotaEncode, NotaSource};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::time::{Duration, timeout};

fn round_trip_nota<T>(value: &T) -> T
where
    T: NotaEncode + NotaDecode + Clone,
{
    let text = value.to_nota();
    NotaSource::new(&text).parse().expect("decode")
}

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

#[test]
fn nota_round_trip_dark() {
    assert_eq!(round_trip_nota(&ThemeMode::Dark), ThemeMode::Dark);
}

#[test]
fn nota_round_trip_light() {
    assert_eq!(round_trip_nota(&ThemeMode::Light), ThemeMode::Light);
}

#[test]
fn nota_encodes_as_pascal_variant_name() {
    let text = ThemeMode::Dark.to_nota();
    assert_eq!(text, "Dark");

    let text = ThemeMode::Light.to_nota();
    assert_eq!(text, "Light");
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
