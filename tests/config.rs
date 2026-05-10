use chroma::{ConfigFile, RampTrigger, ThemeConcern, ThemeMode, ThemeSchedule};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct Fixture {
    temporary_directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self { temporary_directory: tempfile::tempdir().expect("create tempdir") }
    }

    fn write_config(&self, text: &str) -> PathBuf {
        let config = self.temporary_directory.path().join("config.nota");
        fs::write(&config, text).expect("write config");
        config
    }
}

#[test]
fn config_file_extracts_native_theme_axis_from_nota_config() {
    let fixture = Fixture::new();
    let config = fixture.write_config(NATIVE_CONFIG);

    let theme = ConfigFile::from_path(config).theme_axis().expect("theme axis decodes");

    assert_eq!(theme.concerns, vec![ThemeConcern::Terminal, ThemeConcern::Desktop, ThemeConcern::Ghostty]);
    assert_eq!(
        theme.adapters.dconf.as_deref().map(|path| path.to_string_lossy().to_string()),
        Some("/bin/dconf".into())
    );
    assert_eq!(theme.font_point_size, 14);
    assert_eq!(theme.palettes.dark.base00, "#000000");
    assert_eq!(theme.palettes.light.base05, "#3d3530");
    let ThemeSchedule::Scheduled { waypoints, default } = theme.schedule else {
        panic!("expected scheduled theme");
    };
    assert_eq!(default, ThemeMode::Dark);
    assert_eq!(waypoints.len(), 2);
    assert_eq!(waypoints[0].mode, ThemeMode::Light);
    assert!(matches!(waypoints[0].trigger, RampTrigger::CivilDawn(_)));
}

#[test]
fn config_file_decodes_manual_theme_schedule_from_nota_config() {
    let fixture = Fixture::new();
    let config = fixture.write_config(&NATIVE_CONFIG.replace(
        "(Schedule
      (Waypoint (CivilDawn (SignedMinutes 0)) Light)
      (Waypoint (CivilDusk (SignedMinutes 0)) Dark)
      (Default Dark))",
        "(Schedule (Manual Light))",
    ));

    let theme = ConfigFile::from_path(config).theme_axis().expect("theme axis decodes");

    assert_eq!(theme.schedule, ThemeSchedule::Manual(ThemeMode::Light));
}

#[test]
fn hc_chroma_001_apply_command_records_are_rejected_not_interpreted() {
    let fixture = Fixture::new();
    let config = fixture.write_config("(Config (Theme (ApplyCommand \"/tmp/apply-theme\")))");

    let error = ConfigFile::from_path(config).theme_axis().expect_err("apply command must fail");

    assert!(error.to_string().contains("removed shell-apply architecture"));
}

#[test]
fn hc_chroma_002_apply_targets_records_are_rejected_not_migrated() {
    let fixture = Fixture::new();
    let config = fixture.write_config("(Config (Theme (ApplyTargets (Target Terminal \"/tmp/apply-terminal\"))))");

    let error = ConfigFile::from_path(config).theme_axis().expect_err("apply targets must fail");

    assert!(error.to_string().contains("removed shell-apply architecture"));
}

#[test]
fn hc_chroma_003_legacy_theme_concern_is_rejected_not_retained() {
    let fixture = Fixture::new();
    let config =
        fixture.write_config(&NATIVE_CONFIG.replace("(Concerns Terminal Desktop Ghostty)", "(Concerns Legacy)"));

    let error = ConfigFile::from_path(config).theme_axis().expect_err("legacy concern must fail");

    assert!(error.to_string().contains("removed shell-apply architecture"));
}

#[test]
fn hc_chroma_004_yaml_data_inputs_are_rejected_in_favor_of_nota() {
    let fixture = Fixture::new();
    let config = fixture.write_config(&NATIVE_CONFIG.replace("(FontPointSize 14)", "(PaletteFile \"ignis.yaml\")"));

    let error = ConfigFile::from_path(config).theme_axis().expect_err("yaml input must fail");

    assert!(error.to_string().contains("YAML inputs are forbidden"));
}

const NATIVE_CONFIG: &str = r##"
(Config
  (Theme
    (Concerns Terminal Desktop Ghostty)
    (Palettes
      (Dark
        (Base00 "#000000")
        (Base01 "#1a1a1a")
        (Base02 "#2d2d2d")
        (Base03 "#505050")
        (Base04 "#b0b0b0")
        (Base05 "#d0d0d0")
        (Base06 "#e0e0e0")
        (Base07 "#ffffff")
        (Base08 "#ff0066")
        (Base09 "#ff8800")
        (Base0A "#f5c000")
        (Base0B "#00cc44")
        (Base0C "#cc44ff")
        (Base0D "#e040a0")
        (Base0E "#bb44ee")
        (Base0F "#ff5577"))
      (Light
        (Base00 "#faf5f0")
        (Base01 "#efe8e2")
        (Base02 "#ddd5ce")
        (Base03 "#887a70")
        (Base04 "#6a5e55")
        (Base05 "#3d3530")
        (Base06 "#2a2420")
        (Base07 "#1a1510")
        (Base08 "#cc0044")
        (Base09 "#d06600")
        (Base0A "#b89000")
        (Base0B "#1a8a30")
        (Base0C "#9930cc")
        (Base0D "#b03080")
        (Base0E "#8822bb")
        (Base0F "#cc3355")))
    (Adapters
      (Dconf "/bin/dconf")
      (Emacsclient "/bin/emacsclient"))
    (FontPointSize 14)
    (Schedule
      (Waypoint (CivilDawn (SignedMinutes 0)) Light)
      (Waypoint (CivilDusk (SignedMinutes 0)) Dark)
      (Default Dark)))
  (Warmth (Schedule (Manual Neutral)))
  (Brightness (Schedule (Manual Bright))))
"##;
