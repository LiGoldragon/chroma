#![allow(dead_code, non_camel_case_types, non_snake_case)]
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Request {
    SetTheme(ThemeMode),
    GetTheme,
    SetWarmth(WarmthLevel),
    SetWarmthKelvin(i64),
    StartWarmthRamp(RequestStartWarmthRamp),
    StartWarmthRampKelvin(RequestStartWarmthRampKelvin),
    InterruptWarmth,
    GetWarmth,
    SetBrightness(BrightnessLevel),
    SetBrightnessPercent(i64),
    StartBrightnessRamp(RequestStartBrightnessRamp),
    StartBrightnessRampPercent(RequestStartBrightnessRampPercent),
    InterruptBrightness,
    GetBrightness,
    GetState,
    GetSolarClock,
}
#[rustfmt::skip]
pub type RequestSetTheme = ThemeMode;
#[rustfmt::skip]
pub type RequestSetWarmth = WarmthLevel;
#[rustfmt::skip]
pub type RequestSetWarmthKelvin = i64;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct RequestStartWarmthRamp {
    pub warmth_level: WarmthLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct RequestStartWarmthRampKelvin {
    pub integer: i64,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
pub type RequestSetBrightness = BrightnessLevel;
#[rustfmt::skip]
pub type RequestSetBrightnessPercent = i64;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct RequestStartBrightnessRamp {
    pub brightness_level: BrightnessLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct RequestStartBrightnessRampPercent {
    pub integer: i64,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Reply {
    Accepted,
    Theme(ThemeMode),
    Warmth(i64),
    Brightness(i64),
    State(ReplyState),
    SolarClock(ReplySolarClock),
    SolarClockUnavailable,
    Error(String),
}
#[rustfmt::skip]
pub type ReplyTheme = ThemeMode;
#[rustfmt::skip]
pub type ReplyWarmth = i64;
#[rustfmt::skip]
pub type ReplyBrightness = i64;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ReplyState {
    pub theme_mode: ThemeMode,
    pub first_integer: i64,
    pub second_integer: i64,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ReplySolarClock {
    pub first_integer: i64,
    pub second_integer: i64,
}
#[rustfmt::skip]
pub type ReplyError = String;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum ThemeMode {
    Dark,
    Light,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum WarmthLevel {
    Cold,
    Cool,
    Neutral,
    Warm,
    Warmer,
    Warmest,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum BrightnessLevel {
    Dim,
    Dimmer,
    Mid,
    Bright,
    Brighter,
    Brightest,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum RampDuration {
    Minutes(i64),
    Seconds(i64),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Config {
    pub theme_axis: ThemeAxis,
    pub warmth_axis: WarmthAxis,
    pub brightness_axis: BrightnessAxis,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ThemeAxis {
    pub theme_concern_vector: std::vec::Vec<ThemeConcern>,
    pub theme_palettes: ThemePalettes,
    pub string_option: Option<String>,
    pub integer_option: Option<i64>,
    pub ghostty_config_templates_option: Option<GhosttyConfigTemplates>,
    pub pi_theme_control_option: Option<PiThemeControl>,
    pub theme_schedule: ThemeSchedule,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum ThemeConcern {
    Terminal,
    Desktop,
    Ghostty,
    Pi,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ThemePalettes {
    pub first_theme_palette: ThemePalette,
    pub second_theme_palette: ThemePalette,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ThemePalette {
    pub first_string: String,
    pub second_string: String,
    pub third_string: String,
    pub fourth_string: String,
    pub fifth_string: String,
    pub sixth_string: String,
    pub seventh_string: String,
    pub eighth_string: String,
    pub ninth_string: String,
    pub tenth_string: String,
    pub position_11_string: String,
    pub position_12_string: String,
    pub position_13_string: String,
    pub position_14_string: String,
    pub position_15_string: String,
    pub position_16_string: String,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct GhosttyConfigTemplates {
    pub first_string: String,
    pub second_string: String,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct PiThemeControl {
    pub pi_theme_control_registry_directory: PiThemeControlRegistryDirectory,
    pub first_integer_option: Option<i64>,
    pub second_integer_option: Option<i64>,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum PiThemeControlRegistryDirectory {
    RuntimeRelative(String),
    Absolute(String),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum ThemeSchedule {
    Manual(ThemeMode),
    Scheduled(ThemeScheduleScheduled),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ThemeScheduleScheduled {
    pub theme_waypoint_vector: std::vec::Vec<ThemeWaypoint>,
    pub theme_mode: ThemeMode,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ThemeWaypoint {
    pub ramp_trigger: RampTrigger,
    pub theme_mode: ThemeMode,
}
#[rustfmt::skip]
pub type WarmthAxis = WarmthSchedule;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum WarmthSchedule {
    Manual(WarmthLevel),
    Scheduled(WarmthScheduleScheduled),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct WarmthScheduleScheduled {
    pub warmth_waypoint_vector: std::vec::Vec<WarmthWaypoint>,
    pub warmth_level: WarmthLevel,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct WarmthWaypoint {
    pub ramp_trigger: RampTrigger,
    pub warmth_level: WarmthLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
pub type BrightnessAxis = BrightnessSchedule;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum BrightnessSchedule {
    Manual(BrightnessLevel),
    Scheduled(BrightnessScheduleScheduled),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct BrightnessScheduleScheduled {
    pub brightness_waypoint_vector: std::vec::Vec<BrightnessWaypoint>,
    pub brightness_level: BrightnessLevel,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct BrightnessWaypoint {
    pub ramp_trigger: RampTrigger,
    pub brightness_level: BrightnessLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum RampTrigger {
    Sunrise(i64),
    Sunset(i64),
    CivilDawn(i64),
    CivilDusk(i64),
    TimeOfDay(RampTriggerTimeOfDay),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct RampTriggerTimeOfDay {
    pub first_integer: i64,
    pub second_integer: i64,
}
