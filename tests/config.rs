//! Chroma config is embodied by the generated Datom anatomy, then validated by runtime policy.

use chroma::{BrightnessLevel, ConfigFile, ThemeConcern, ThemeMode, WarmthLevel};
use datom_codec::Datomizable;
use protos::{Protosizable, Textualizable};

fn string(value: &str) -> String {
    value.to_owned()
}

fn palette() -> chroma::generated::ThemePalette {
    chroma::generated::ThemePalette { first_string: string("#000000"), second_string: string("#111111"), third_string: string("#222222"), fourth_string: string("#333333"), fifth_string: string("#444444"), sixth_string: string("#555555"), seventh_string: string("#666666"), eighth_string: string("#777777"), ninth_string: string("#888888"), tenth_string: string("#999999"), position_11_string: string("#aaaaaa"), position_12_string: string("#bbbbbb"), position_13_string: string("#cccccc"), position_14_string: string("#dddddd"), position_15_string: string("#eeeeee"), position_16_string: string("#ffffff") }
}

fn config() -> chroma::generated::Config {
    use chroma::generated as data;
    chroma::generated::Config { theme_axis: data::ThemeAxis { theme_concern_vector: vec![data::ThemeConcern::Terminal], theme_palettes: data::ThemePalettes { first_theme_palette: palette(), second_theme_palette: palette() }, string_option: None, integer_option: None, ghostty_config_templates_option: None, pi_theme_control_option: None, theme_schedule: data::ThemeSchedule::Manual(data::ThemeMode::Dark) }, warmth_axis: data::WarmthSchedule::Manual(data::WarmthLevel::Neutral), brightness_axis: data::BrightnessSchedule::Manual(data::BrightnessLevel::Bright) }
}

fn fixture() -> (tempfile::TempDir, ConfigFile) {
    let directory = tempfile::tempdir().expect("create config fixture");
    let path = directory.path().join("config.datom");
    std::fs::write(&path, config().datomize(vec![]).protosize().textualize()).expect("write Datom fixture");
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
