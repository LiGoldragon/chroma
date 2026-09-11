//! [`Config`] — Chroma's schema-authored Datom configuration.

use core::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use datom_codec::{Actualizing, Budget, Potential};

use crate::brightness::{BrightnessAxis, BrightnessLevel, BrightnessSchedule, BrightnessWaypoint};
use crate::error::{Error, Result};
use crate::generated as data;
use crate::theme::{
    GhosttyConfigTemplates, PiThemeControl, PiThemeControlRegistryDirectory, ThemeAdapters, ThemeAxis, ThemeConcern,
    ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule, ThemeWaypoint,
};
use crate::time::{LocalHour, LocalMinute, RampDuration, RampTrigger, SignedMinutes};
use crate::warmth::{WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint};

/// The on-disk Chroma configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn from_default_locations() -> Result<Self> {
        if let Some(path) = std::env::var_os("CHROMA_CONFIG").map(PathBuf::from) {
            return Ok(Self { path });
        }
        if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
            return Ok(Self { path: path.join("chroma/config.datom") });
        }
        if let Some(path) = std::env::var_os("HOME").map(PathBuf::from) {
            return Ok(Self { path: path.join(".config/chroma/config.datom") });
        }
        Err(Error::Config { message: "neither CHROMA_CONFIG, XDG_CONFIG_HOME, nor HOME locates config.datom".into() })
    }

    pub fn theme_axis(&self) -> Result<ThemeAxis> {
        Ok(self.config()?.theme)
    }

    pub fn config(&self) -> Result<Config> {
        Self::decode_config(&std::fs::read_to_string(&self.path)?)
    }

    pub async fn config_async(&self) -> Result<Config> {
        Self::decode_config(&tokio::fs::read_to_string(&self.path).await?)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn decode_config(text: &str) -> Result<Config> {
        let mut potential = Potential::<data::Config>::from(text);
        let mut budget = Budget { remaining: 16_384, reader: protos::ReaderBudget { remaining: 16_384 }, depth: 0, maximum_depth: 16_384 };
        potential
            .actualize(&mut budget)
            .map_err(|error| Error::Config { message: format!("Datom config: {error:?}") })?
            .try_into()
    }
}

/// Top-level Chroma runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeAxis,
    pub warmth: WarmthAxis,
    pub brightness: BrightnessAxis,
}

impl Config {
    pub fn needs_geolocation(&self) -> bool {
        self.theme.schedule.needs_geolocation()
            || self.warmth.schedule.needs_geolocation()
            || self.brightness.schedule.needs_geolocation()
    }
}

impl TryFrom<data::Config> for Config {
    type Error = Error;

    fn try_from(value: data::Config) -> Result<Self> {
        let data::Config { theme_axis: theme, warmth_axis: warmth, brightness_axis: brightness } = value;
        Ok(Self {
            theme: theme_axis(theme)?,
            warmth: WarmthAxis { schedule: warmth_schedule(warmth)? },
            brightness: BrightnessAxis { schedule: brightness_schedule(brightness)? },
        })
    }
}

fn theme_axis(value: data::ThemeAxis) -> Result<ThemeAxis> {
    let data::ThemeAxis { theme_concern_vector: concerns, theme_palettes: palettes, string_option: dconf, integer_option: font_point_size, ghostty_config_templates_option: ghostty_config_templates, pi_theme_control_option: pi_theme_control_data, theme_schedule: schedule } = value;
    let concerns = concerns.into_iter().map(theme_concern).collect::<Vec<_>>();
    let ghostty_config_templates = ghostty_config_templates.map(|data::GhosttyConfigTemplates { first_string: dark, second_string: light }| {
        GhosttyConfigTemplates { dark: PathBuf::from(dark), light: PathBuf::from(light) }
    });
    if concerns.contains(&ThemeConcern::Ghostty) && ghostty_config_templates.is_none() {
        return Err(Error::Config {
            message: "Ghostty concern requires ghosttyConfigTemplates with dark and light paths".into(),
        });
    }
    Ok(ThemeAxis {
        concerns,
        palettes: theme_palettes(palettes),
        adapters: ThemeAdapters { dconf: dconf.map(PathBuf::from) },
        font_point_size: optional_positive_u8(font_point_size.map(i64::from), "fontPointSize", 12)?,
        ghostty_config_templates,
        pi_theme_control: pi_theme_control_data.map(pi_theme_control).transpose()?,
        schedule: theme_schedule(schedule)?,
    })
}

fn theme_palettes(value: data::ThemePalettes) -> ThemePalettes {
    let data::ThemePalettes { first_theme_palette: dark, second_theme_palette: light } = value;
    ThemePalettes { dark: palette(dark), light: palette(light) }
}

fn palette(value: data::ThemePalette) -> ThemePalette {
    let data::ThemePalette { first_string: base00, second_string: base01, third_string: base02, fourth_string: base03, fifth_string: base04, sixth_string: base05, seventh_string: base06, eighth_string: base07, ninth_string: base08, tenth_string: base09, position_11_string: base0a, position_12_string: base0b, position_13_string: base0c, position_14_string: base0d, position_15_string: base0e, position_16_string: base0f } = value;
    ThemePalette {
        base00, base01, base02, base03, base04, base05, base06, base07,
        base08, base09, base0a, base0b, base0c, base0d, base0e, base0f,
    }
}

fn pi_theme_control(value: data::PiThemeControl) -> Result<PiThemeControl> {
    let data::PiThemeControl { pi_theme_control_registry_directory: registry_directory, first_integer_option: connect_timeout_millis, second_integer_option: write_timeout_millis } = value;
    let registry_directory = match registry_directory {
        data::PiThemeControlRegistryDirectory::RuntimeRelative(path) => {
            PiThemeControlRegistryDirectory::runtime_relative(path)
        }
        data::PiThemeControlRegistryDirectory::Absolute(path) => {
            PiThemeControlRegistryDirectory::absolute(path)
        }
    };
    Ok(PiThemeControl {
        registry_directory,
        connect_timeout: Duration::from_millis(optional_positive_u64(
            connect_timeout_millis.map(i64::from),
            "connectTimeoutMillis",
            100,
        )?),
        write_timeout: Duration::from_millis(optional_positive_u64(
            write_timeout_millis.map(i64::from),
            "writeTimeoutMillis",
            100,
        )?),
    })
}

fn theme_concern(value: data::ThemeConcern) -> ThemeConcern {
    match value {
        data::ThemeConcern::Terminal => ThemeConcern::Terminal,
        data::ThemeConcern::Desktop => ThemeConcern::Desktop,
        data::ThemeConcern::Ghostty => ThemeConcern::Ghostty,
        data::ThemeConcern::Pi => ThemeConcern::Pi,
    }
}

fn theme_mode(value: data::ThemeMode) -> ThemeMode {
    match value {
        data::ThemeMode::Dark => ThemeMode::Dark,
        data::ThemeMode::Light => ThemeMode::Light,
    }
}

fn warmth_level(value: data::WarmthLevel) -> WarmthLevel {
    match value {
        data::WarmthLevel::Cold => WarmthLevel::Cold,
        data::WarmthLevel::Cool => WarmthLevel::Cool,
        data::WarmthLevel::Neutral => WarmthLevel::Neutral,
        data::WarmthLevel::Warm => WarmthLevel::Warm,
        data::WarmthLevel::Warmer => WarmthLevel::Warmer,
        data::WarmthLevel::Warmest => WarmthLevel::Warmest,
    }
}

fn brightness_level(value: data::BrightnessLevel) -> BrightnessLevel {
    match value {
        data::BrightnessLevel::Dim => BrightnessLevel::Dim,
        data::BrightnessLevel::Dimmer => BrightnessLevel::Dimmer,
        data::BrightnessLevel::Mid => BrightnessLevel::Mid,
        data::BrightnessLevel::Bright => BrightnessLevel::Bright,
        data::BrightnessLevel::Brighter => BrightnessLevel::Brighter,
        data::BrightnessLevel::Brightest => BrightnessLevel::Brightest,
    }
}

fn theme_schedule(value: data::ThemeSchedule) -> Result<ThemeSchedule> {
    match value {
        data::ThemeSchedule::Manual(mode) => Ok(ThemeSchedule::Manual(theme_mode(mode))),
        data::ThemeSchedule::Scheduled(value) => {
            let data::ThemeScheduleScheduled { theme_waypoint_vector: waypoints, theme_mode: default } = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::ThemeWaypoint { ramp_trigger: trigger_value, theme_mode: mode } = waypoint;
                    Ok(ThemeWaypoint { trigger: trigger(trigger_value)?, mode: theme_mode(mode) })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, default, "theme", |waypoints, default| ThemeSchedule::Scheduled {
                waypoints,
                default: theme_mode(default),
            })
        }
    }
}

fn warmth_schedule(value: data::WarmthSchedule) -> Result<WarmthSchedule> {
    match value {
        data::WarmthSchedule::Manual(level) => Ok(WarmthSchedule::Manual(warmth_level(level))),
        data::WarmthSchedule::Scheduled(value) => {
            let data::WarmthScheduleScheduled { warmth_waypoint_vector: waypoints, warmth_level: default } = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::WarmthWaypoint { ramp_trigger: trigger_value, warmth_level: target, ramp_duration } = waypoint;
                    Ok(WarmthWaypoint {
                        trigger: trigger(trigger_value)?,
                        target: warmth_level(target),
                        ramp_duration: duration(ramp_duration)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, default, "warmth", |waypoints, default| WarmthSchedule::Scheduled {
                waypoints,
                default: warmth_level(default),
            })
        }
    }
}

fn brightness_schedule(value: data::BrightnessSchedule) -> Result<BrightnessSchedule> {
    match value {
        data::BrightnessSchedule::Manual(level) => Ok(BrightnessSchedule::Manual(brightness_level(level))),
        data::BrightnessSchedule::Scheduled(value) => {
            let data::BrightnessScheduleScheduled { brightness_waypoint_vector: waypoints, brightness_level: default } = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::BrightnessWaypoint { ramp_trigger: trigger_value, brightness_level: target, ramp_duration } = waypoint;
                    Ok(BrightnessWaypoint {
                        trigger: trigger(trigger_value)?,
                        target: brightness_level(target),
                        ramp_duration: duration(ramp_duration)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, default, "brightness", |waypoints, default| BrightnessSchedule::Scheduled {
                waypoints,
                default: brightness_level(default),
            })
        }
    }
}

fn scheduled<Value, Default, Output>(
    waypoints: Vec<Value>,
    default: Default,
    name: &str,
    build: impl FnOnce(Vec<Value>, Default) -> Output,
) -> Result<Output> {
    if waypoints.is_empty() {
        return Err(Error::Config { message: format!("{name} scheduled config needs at least one waypoint") });
    }
    Ok(build(waypoints, default))
}

fn trigger(value: data::RampTrigger) -> Result<RampTrigger> {
    let offset = |value: i64| {
        let value = i64::from(value);
        i16::try_from(value)
            .map(SignedMinutes::new)
            .map_err(|_| Error::Config { message: format!("solar offset must fit signed 16-bit minutes, got {value}") })
    };
    Ok(match value {
        data::RampTrigger::Sunrise(value) => RampTrigger::Sunrise(offset(value)?),
        data::RampTrigger::Sunset(value) => RampTrigger::Sunset(offset(value)?),
        data::RampTrigger::CivilDawn(value) => RampTrigger::CivilDawn(offset(value)?),
        data::RampTrigger::CivilDusk(value) => RampTrigger::CivilDusk(offset(value)?),
        data::RampTrigger::TimeOfDay(data::RampTriggerTimeOfDay { first_integer: hour, second_integer: minute }) => RampTrigger::TimeOfDay(
            checked_time(i64::from(hour), 23, "TimeOfDay hour").map(LocalHour::new)?,
            checked_time(i64::from(minute), 59, "TimeOfDay minute").map(LocalMinute::new)?,
        ),
    })
}

fn duration(value: data::RampDuration) -> Result<RampDuration> {
    match value {
        data::RampDuration::Minutes(value) => u32::try_from(i64::from(value)).map(RampDuration::from_minutes),
        data::RampDuration::Seconds(value) => u64::try_from(i64::from(value)).map(RampDuration::from_seconds),
    }
    .map_err(|_| Error::Config { message: "ramp duration must be non-negative".into() })
}

fn checked_time(value: i64, maximum: u8, name: &str) -> Result<u8> {
    u8::try_from(value)
        .ok()
        .filter(|value| *value <= maximum)
        .ok_or_else(|| Error::Config { message: format!("{name} must be between 0 and {maximum}, got {value}") })
}

fn optional_positive_u64(value: Option<i64>, name: &str, default: u64) -> Result<u64> {
    match value {
        None => Ok(default),
        Some(value) if value > 0 => {
            u64::try_from(value).map_err(|_| Error::Config { message: format!("{name} must fit u64") })
        }
        Some(value) => Err(Error::Config { message: format!("{name} must be positive, got {value}") }),
    }
}

fn optional_positive_u8(value: Option<i64>, name: &str, default: u8) -> Result<u8> {
    match value {
        None => Ok(default),
        Some(value) if value > 0 => {
            u8::try_from(value).map_err(|_| Error::Config { message: format!("{name} must fit u8") })
        }
        Some(value) => Err(Error::Config { message: format!("{name} must be positive, got {value}") }),
    }
}

impl fmt::Display for ConfigFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.path.display())
    }
}

impl AsRef<Path> for ConfigFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
