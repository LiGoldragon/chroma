use chroma::{
    ApplyTheme, PiThemeControl, ThemeAdapters, ThemeApplier, ThemeAxis, ThemeConcern, ThemeMode, ThemePalette,
    ThemePalettes, ThemeSchedule,
};
use kameo::actor::ActorRef;
use nota_next::{NotaDecode, NotaEncode, NotaSource};
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

    fn socket_path(&self) -> std::path::PathBuf {
        self.temporary_directory.path().join("pi-live-theme.sock")
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
            pi_theme_control: Some(PiThemeControl::from_socket_path(self.socket_path())),
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
async fn pi_theme_control_sends_dark_and_light_line_events() {
    let fixture = ThemeApplierFixture::new();
    let listener = UnixListener::bind(fixture.socket_path()).expect("bind Pi theme control listener");
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Dark).await;
    fixture.apply_theme(&applier, ThemeMode::Light).await;

    let mut received = Vec::new();
    for _ in 0..2 {
        let (mut stream, _) = timeout(Duration::from_secs(1), listener.accept())
            .await
            .expect("listener receives connection")
            .expect("accept connection");
        let mut message = String::new();
        timeout(Duration::from_secs(1), stream.read_to_string(&mut message))
            .await
            .expect("read Pi theme control message")
            .expect("read connection");
        received.push(message);
    }

    assert_eq!(received, vec!["dark\n", "light\n"]);
    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
}

#[tokio::test]
async fn missing_pi_theme_control_socket_is_non_fatal_for_theme_apply() {
    let fixture = ThemeApplierFixture::new();
    let applier = ThemeApplier::start(fixture.pi_axis()).await;

    fixture.apply_theme(&applier, ThemeMode::Dark).await;

    let _ = applier.stop_gracefully().await;
    applier.wait_for_shutdown().await;
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
