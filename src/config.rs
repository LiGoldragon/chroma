//! [`Config`] — Chroma's schema-authored Datom configuration.

use core::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use datomic::{Text, TextEdge};

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
        Text::<data::Config>::from(text)
            .embody()
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
        Ok(Self {
            theme: theme_axis(value.theme)?,
            warmth: WarmthAxis { schedule: warmth_schedule(value.warmth.schedule)? },
            brightness: BrightnessAxis { schedule: brightness_schedule(value.brightness.schedule)? },
        })
    }
}

fn theme_axis(value: data::ThemeAxis) -> Result<ThemeAxis> {
    let concerns = value.concerns.into_iter().map(theme_concern).collect::<Vec<_>>();
    let ghostty_config_templates = value.ghostty_config_templates.map(|templates| GhosttyConfigTemplates {
        dark: PathBuf::from(templates.dark.as_ref()),
        light: PathBuf::from(templates.light.as_ref()),
    });
    if concerns.contains(&ThemeConcern::Ghostty) && ghostty_config_templates.is_none() {
        return Err(Error::Config {
            message: "Ghostty concern requires ghosttyConfigTemplates with dark and light paths".into(),
        });
    }
    Ok(ThemeAxis {
        concerns,
        palettes: ThemePalettes { dark: palette(value.palettes.dark), light: palette(value.palettes.light) },
        adapters: ThemeAdapters { dconf: value.dconf.map(|path| PathBuf::from(path.as_ref())) },
        font_point_size: optional_positive_u8(value.font_point_size, "fontPointSize", 12)?,
        ghostty_config_templates,
        pi_theme_control: value.pi_theme_control.map(pi_theme_control).transpose()?,
        schedule: theme_schedule(value.schedule)?,
    })
}

fn palette(value: data::ThemePalette) -> ThemePalette {
    ThemePalette {
        base00: value.base00.as_ref().into(),
        base01: value.base01.as_ref().into(),
        base02: value.base02.as_ref().into(),
        base03: value.base03.as_ref().into(),
        base04: value.base04.as_ref().into(),
        base05: value.base05.as_ref().into(),
        base06: value.base06.as_ref().into(),
        base07: value.base07.as_ref().into(),
        base08: value.base08.as_ref().into(),
        base09: value.base09.as_ref().into(),
        base0a: value.base0_a.as_ref().into(),
        base0b: value.base0_b.as_ref().into(),
        base0c: value.base0_c.as_ref().into(),
        base0d: value.base0_d.as_ref().into(),
        base0e: value.base0_e.as_ref().into(),
        base0f: value.base0_f.as_ref().into(),
    }
}

fn pi_theme_control(value: data::PiThemeControl) -> Result<PiThemeControl> {
    let registry_directory = match value.registry_directory {
        data::PiThemeControlRegistryDirectory::RuntimeRelative(path) => {
            PiThemeControlRegistryDirectory::runtime_relative(path.as_ref())
        }
        data::PiThemeControlRegistryDirectory::Absolute(path) => {
            PiThemeControlRegistryDirectory::absolute(path.as_ref())
        }
    };
    Ok(PiThemeControl {
        registry_directory,
        connect_timeout: Duration::from_millis(optional_positive_u64(
            value.connect_timeout_millis,
            "connectTimeoutMillis",
            100,
        )?),
        write_timeout: Duration::from_millis(optional_positive_u64(
            value.write_timeout_millis,
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
            let waypoints = value
                .waypoints
                .into_iter()
                .map(|waypoint| {
                    Ok(ThemeWaypoint { trigger: trigger(waypoint.trigger)?, mode: theme_mode(waypoint.mode) })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, value.default, "theme", |waypoints, default| ThemeSchedule::Scheduled {
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
            let waypoints = value
                .waypoints
                .into_iter()
                .map(|waypoint| {
                    Ok(WarmthWaypoint {
                        trigger: trigger(waypoint.trigger)?,
                        target: warmth_level(waypoint.target),
                        ramp_duration: duration(waypoint.ramp_duration)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, value.default, "warmth", |waypoints, default| WarmthSchedule::Scheduled {
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
            let waypoints = value
                .waypoints
                .into_iter()
                .map(|waypoint| {
                    Ok(BrightnessWaypoint {
                        trigger: trigger(waypoint.trigger)?,
                        target: brightness_level(waypoint.target),
                        ramp_duration: duration(waypoint.ramp_duration)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            scheduled(waypoints, value.default, "brightness", |waypoints, default| BrightnessSchedule::Scheduled {
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
        i16::try_from(value)
            .map(SignedMinutes::new)
            .map_err(|_| Error::Config { message: format!("solar offset must fit signed 16-bit minutes, got {value}") })
    };
    Ok(match value {
        data::RampTrigger::Sunrise(value) => RampTrigger::Sunrise(offset(value)?),
        data::RampTrigger::Sunset(value) => RampTrigger::Sunset(offset(value)?),
        data::RampTrigger::CivilDawn(value) => RampTrigger::CivilDawn(offset(value)?),
        data::RampTrigger::CivilDusk(value) => RampTrigger::CivilDusk(offset(value)?),
        data::RampTrigger::TimeOfDay(value) => RampTrigger::TimeOfDay(
            checked_time(value.hour, 23, "TimeOfDay hour").map(LocalHour::new)?,
            checked_time(value.minute, 59, "TimeOfDay minute").map(LocalMinute::new)?,
        ),
    })
}

fn duration(value: data::RampDuration) -> Result<RampDuration> {
    match value {
        data::RampDuration::Minutes(value) => u32::try_from(value).map(RampDuration::from_minutes),
        data::RampDuration::Seconds(value) => u64::try_from(value).map(RampDuration::from_seconds),
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
