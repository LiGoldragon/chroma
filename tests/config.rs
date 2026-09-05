//! Chroma config is embodied by the generated Datom anatomy, then validated by runtime policy.

use chroma::{BrightnessLevel, ConfigFile, ThemeConcern, ThemeMode, WarmthLevel};
use datom_codec::Textualizable;

fn string(value: &str) -> protos::Text {
    value.to_owned().try_into().expect("representable Datom string")
}

fn palette() -> chroma::generated::ThemePalette {
    chroma::generated::ThemePalette(
        string("#000000"),
        string("#111111"),
        string("#222222"),
        string("#333333"),
        string("#444444"),
        string("#555555"),
        string("#666666"),
        string("#777777"),
        string("#888888"),
        string("#999999"),
        string("#aaaaaa"),
        string("#bbbbbb"),
        string("#cccccc"),
        string("#dddddd"),
        string("#eeeeee"),
        string("#ffffff"),
    )
}

fn config() -> chroma::generated::Config {
    use chroma::generated as data;
    chroma::generated::Config(
        data::ThemeAxis(
            vec![data::ThemeConcern::Terminal],
            data::ThemePalettes(palette(), palette()),
            None,
            None,
            None,
            None,
            data::ThemeSchedule::Manual(data::ThemeMode::Dark),
        ),
        data::WarmthAxis(data::WarmthSchedule::Manual(data::WarmthLevel::Neutral)),
        data::BrightnessAxis(data::BrightnessSchedule::Manual(data::BrightnessLevel::Bright)),
    )
}

fn fixture() -> (tempfile::TempDir, ConfigFile) {
    let directory = tempfile::tempdir().expect("create config fixture");
    let path = directory.path().join("config.datom");
    std::fs::write(&path, config().textualize()).expect("write Datom fixture");
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
