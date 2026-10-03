#![allow(dead_code, non_camel_case_types, non_snake_case)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
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
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestStartWarmthRamp {
    pub warmth_level: WarmthLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestStartWarmthRampKelvin {
    pub integer: i64,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
pub type RequestSetBrightness = BrightnessLevel;
#[rustfmt::skip]
pub type RequestSetBrightnessPercent = i64;
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestStartBrightnessRamp {
    pub brightness_level: BrightnessLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestStartBrightnessRampPercent {
    pub integer: i64,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
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
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ReplyState {
    pub theme_mode: ThemeMode,
    pub first_integer: i64,
    pub second_integer: i64,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ReplySolarClock {
    pub first_integer: i64,
    pub second_integer: i64,
}
#[rustfmt::skip]
pub type ReplyError = String;
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum ThemeMode {
    Dark,
    Light,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum WarmthLevel {
    Cold,
    Cool,
    Neutral,
    Warm,
    Warmer,
    Warmest,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum BrightnessLevel {
    Dim,
    Dimmer,
    Mid,
    Bright,
    Brighter,
    Brightest,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum RampDuration {
    Minutes(i64),
    Seconds(i64),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Config {
    pub theme_axis: ThemeAxis,
    pub warmth_axis: WarmthAxis,
    pub brightness_axis: BrightnessAxis,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ThemeAxis {
    pub theme_concern_vector: std::vec::Vec<ThemeConcern>,
    pub theme_palettes: ThemePalettes,
    pub string_option: std::option::Option<String>,
    pub integer_option: std::option::Option<i64>,
    pub ghostty_config_templates_option: std::option::Option<GhosttyConfigTemplates>,
    pub pi_theme_control_option: std::option::Option<PiThemeControl>,
    pub theme_schedule: ThemeSchedule,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum ThemeConcern {
    Terminal,
    Desktop,
    Ghostty,
    Pi,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ThemePalettes {
    pub first_theme_palette: ThemePalette,
    pub second_theme_palette: ThemePalette,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
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
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct GhosttyConfigTemplates {
    pub first_string: String,
    pub second_string: String,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct PiThemeControl {
    pub pi_theme_control_registry_directory: PiThemeControlRegistryDirectory,
    pub first_integer_option: std::option::Option<i64>,
    pub second_integer_option: std::option::Option<i64>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum PiThemeControlRegistryDirectory {
    RuntimeRelative(String),
    Absolute(String),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum ThemeSchedule {
    Manual(ThemeMode),
    Scheduled(ThemeScheduleScheduled),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ThemeScheduleScheduled {
    pub theme_waypoint_vector: std::vec::Vec<ThemeWaypoint>,
    pub theme_mode: ThemeMode,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ThemeWaypoint {
    pub ramp_trigger: RampTrigger,
    pub theme_mode: ThemeMode,
}
#[rustfmt::skip]
pub type WarmthAxis = WarmthSchedule;
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum WarmthSchedule {
    Manual(WarmthLevel),
    Scheduled(WarmthScheduleScheduled),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct WarmthScheduleScheduled {
    pub warmth_waypoint_vector: std::vec::Vec<WarmthWaypoint>,
    pub warmth_level: WarmthLevel,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct WarmthWaypoint {
    pub ramp_trigger: RampTrigger,
    pub warmth_level: WarmthLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
pub type BrightnessAxis = BrightnessSchedule;
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum BrightnessSchedule {
    Manual(BrightnessLevel),
    Scheduled(BrightnessScheduleScheduled),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct BrightnessScheduleScheduled {
    pub brightness_waypoint_vector: std::vec::Vec<BrightnessWaypoint>,
    pub brightness_level: BrightnessLevel,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct BrightnessWaypoint {
    pub ramp_trigger: RampTrigger,
    pub brightness_level: BrightnessLevel,
    pub ramp_duration: RampDuration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum RampTrigger {
    Sunrise(i64),
    Sunset(i64),
    CivilDawn(i64),
    CivilDusk(i64),
    TimeOfDay(RampTriggerTimeOfDay),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RampTriggerTimeOfDay {
    pub first_integer: i64,
    pub second_integer: i64,
}
