//! [`Config`] — Chroma's schema-authored Datom configuration.

use core::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use datom_codec::{Actualizable, IncorporationBudget, Potential};

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
        Potential::<data::Config>::from(text)
            .actualize(IncorporationBudget::try_from(16_384).expect("positive config budget"))
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
        let data::Config(theme, warmth, brightness) = value;
        let data::WarmthAxis(warmth_data) = warmth;
        let data::BrightnessAxis(brightness_data) = brightness;
        Ok(Self {
            theme: theme_axis(theme)?,
            warmth: WarmthAxis { schedule: warmth_schedule(warmth_data)? },
            brightness: BrightnessAxis { schedule: brightness_schedule(brightness_data)? },
        })
    }
}

fn theme_axis(value: data::ThemeAxis) -> Result<ThemeAxis> {
    let data::ThemeAxis(
        concerns,
        palettes,
        dconf,
        font_point_size,
        ghostty_config_templates,
        pi_theme_control_data,
        schedule,
    ) = value;
    let concerns = concerns.into_iter().map(theme_concern).collect::<Vec<_>>();
    let ghostty_config_templates = ghostty_config_templates.map(|data::GhosttyConfigTemplates(dark, light)| {
        GhosttyConfigTemplates { dark: PathBuf::from(dark.as_ref()), light: PathBuf::from(light.as_ref()) }
    });
    if concerns.contains(&ThemeConcern::Ghostty) && ghostty_config_templates.is_none() {
        return Err(Error::Config {
            message: "Ghostty concern requires ghosttyConfigTemplates with dark and light paths".into(),
        });
    }
    Ok(ThemeAxis {
        concerns,
        palettes: theme_palettes(palettes),
        adapters: ThemeAdapters { dconf: dconf.map(|path| PathBuf::from(path.as_ref())) },
        font_point_size: optional_positive_u8(font_point_size.map(i64::from), "fontPointSize", 12)?,
        ghostty_config_templates,
        pi_theme_control: pi_theme_control_data.map(pi_theme_control).transpose()?,
        schedule: theme_schedule(schedule)?,
    })
}

fn theme_palettes(value: data::ThemePalettes) -> ThemePalettes {
    let data::ThemePalettes(dark, light) = value;
    ThemePalettes { dark: palette(dark), light: palette(light) }
}

fn palette(value: data::ThemePalette) -> ThemePalette {
    let data::ThemePalette(
        base00,
        base01,
        base02,
        base03,
        base04,
        base05,
        base06,
        base07,
        base08,
        base09,
        base0a,
        base0b,
        base0c,
        base0d,
        base0e,
        base0f,
    ) = value;
    ThemePalette {
        base00: base00.as_ref().into(),
        base01: base01.as_ref().into(),
        base02: base02.as_ref().into(),
        base03: base03.as_ref().into(),
        base04: base04.as_ref().into(),
        base05: base05.as_ref().into(),
        base06: base06.as_ref().into(),
        base07: base07.as_ref().into(),
        base08: base08.as_ref().into(),
        base09: base09.as_ref().into(),
        base0a: base0a.as_ref().into(),
        base0b: base0b.as_ref().into(),
        base0c: base0c.as_ref().into(),
        base0d: base0d.as_ref().into(),
        base0e: base0e.as_ref().into(),
        base0f: base0f.as_ref().into(),
    }
}

fn pi_theme_control(value: data::PiThemeControl) -> Result<PiThemeControl> {
    let data::PiThemeControl(registry_directory, connect_timeout_millis, write_timeout_millis) = value;
    let registry_directory = match registry_directory {
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
            let data::ThemeScheduleScheduled(waypoints, default) = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::ThemeWaypoint(trigger_value, mode) = waypoint;
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
            let data::WarmthScheduleScheduled(waypoints, default) = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::WarmthWaypoint(trigger_value, target, ramp_duration) = waypoint;
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
            let data::BrightnessScheduleScheduled(waypoints, default) = value;
            let waypoints = waypoints
                .into_iter()
                .map(|waypoint| {
                    let data::BrightnessWaypoint(trigger_value, target, ramp_duration) = waypoint;
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
    let offset = |value: protos::Integer| {
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
        data::RampTrigger::TimeOfDay(data::RampTriggerTimeOfDay(hour, minute)) => RampTrigger::TimeOfDay(
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
