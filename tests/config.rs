//! Chroma config is embodied by the generated Datom anatomy, then validated by runtime policy.

use chroma::{BrightnessLevel, ConfigFile, ThemeConcern, ThemeMode, WarmthLevel};
use datomic::Datomic;

fn string(value: &str) -> datomic::DatomicString {
    value.to_owned().try_into().expect("representable Datom string")
}

fn palette() -> chroma::generated::ThemePalette {
    chroma::generated::ThemePalette {
        base00: string("#000000"),
        base01: string("#111111"),
        base02: string("#222222"),
        base03: string("#333333"),
        base04: string("#444444"),
        base05: string("#555555"),
        base06: string("#666666"),
        base07: string("#777777"),
        base08: string("#888888"),
        base09: string("#999999"),
        base0_a: string("#aaaaaa"),
        base0_b: string("#bbbbbb"),
        base0_c: string("#cccccc"),
        base0_d: string("#dddddd"),
        base0_e: string("#eeeeee"),
        base0_f: string("#ffffff"),
    }
}

fn config() -> chroma::generated::Config {
    use chroma::generated as data;
    chroma::generated::Config {
        theme: data::ThemeAxis {
            concerns: vec![data::ThemeConcern::Terminal],
            palettes: data::ThemePalettes { dark: palette(), light: palette() },
            dconf: None,
            font_point_size: None,
            ghostty_config_templates: None,
            pi_theme_control: None,
            schedule: data::ThemeSchedule::Manual(data::ThemeMode::Dark),
        },
        warmth: data::WarmthAxis { schedule: data::WarmthSchedule::Manual(data::WarmthLevel::Neutral) },
        brightness: data::BrightnessAxis { schedule: data::BrightnessSchedule::Manual(data::BrightnessLevel::Bright) },
    }
}

fn fixture() -> (tempfile::TempDir, ConfigFile) {
    let directory = tempfile::tempdir().expect("create config fixture");
    let path = directory.path().join("config.datom");
    std::fs::write(&path, config().textualize().as_ref()).expect("write Datom fixture");
    (directory, ConfigFile::from_path(path))
}

#[test]
fn generated_datom_config_becomes_runtime_axes() {
    let (_directory, file) = fixture();
    let config = file.config().expect("embody configuration");
    assert_eq!(config.theme.concerns, vec![ThemeConcern::Terminal]);
    assert_eq!(config.theme.schedule, chroma::ThemeSchedule::Manual(ThemeMode::Dark));
    assert_eq!(config.warmth.schedule, chroma::WarmthSchedule::Manual(WarmthLevel::Neutral));
    assert_eq!(config.brightness.schedule, chroma::BrightnessSchedule::Manual(BrightnessLevel::Bright));
}

#[test]
fn config_path_is_datom() {
    let directory = tempfile::tempdir().expect("create config home");
    unsafe { std::env::set_var("XDG_CONFIG_HOME", directory.path()) };
    let file = ConfigFile::from_default_locations().expect("locate config");
    assert!(file.path().ends_with("chroma/config.datom"));
}

#[test]
fn legacy_and_yaml_are_not_chroma_config_anatomies() {
    let directory = tempfile::tempdir().expect("create config fixture");
    let path = directory.path().join("config.datom");
    std::fs::write(&path, "Config.(Theme.(Dark))").expect("write legacy fixture");
    assert!(ConfigFile::from_path(&path).config().is_err());
    std::fs::write(&path, "theme: dark\n").expect("write yaml fixture");
    assert!(ConfigFile::from_path(path).config().is_err());
}
