//! [`Request`] — Chroma's internal rkyv request frame.
//!
//! The CLI embodies the generated Datom [`crate::generated::Request`]
//! before this runtime-only representation crosses the Unix socket.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::{BrightnessLevel, BrightnessPercent};
use crate::error::{Error, Result};
use crate::generated;
use crate::theme::ThemeMode;
use crate::time::RampDuration;
use crate::warmth::{KelvinTemperature, WarmthLevel};

/// What the CLI sends to the daemon over the rkyv socket frame.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq)]
pub enum Request {
    SetTheme { mode: ThemeMode },
    GetTheme,
    SetWarmth { level: WarmthLevel },
    SetWarmthKelvin { kelvin: KelvinTemperature },
    GetWarmth,
    StartWarmthRamp { target: WarmthLevel, duration: RampDuration },
    StartWarmthRampKelvin { target: KelvinTemperature, duration: RampDuration },
    InterruptWarmth,
    SetBrightness { level: BrightnessLevel },
    SetBrightnessPercent { percent: BrightnessPercent },
    GetBrightness,
    StartBrightnessRamp { target: BrightnessLevel, duration: RampDuration },
    StartBrightnessRampPercent { target: BrightnessPercent, duration: RampDuration },
    InterruptBrightness,
    GetState,
    GetSolarClock,
}

impl Request {
    /// Archive into rkyv bytes for the local socket frame.
    pub fn archive(&self) -> Result<Vec<u8>> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(|err| Error::RkyvCodec(err.to_string()))
    }

    /// Reconstruct from an rkyv archive coming off the local socket.
    pub fn from_archive(bytes: &[u8]) -> Result<Self> {
        rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes).map_err(|err| Error::RkyvCodec(err.to_string()))
    }
}

impl TryFrom<generated::Request> for Request {
    type Error = Error;

    fn try_from(request: generated::Request) -> Result<Self> {
        use generated::WarmthLevel as DataWarmth;
        use generated::{BrightnessLevel as DataBrightness, RampDuration as DataDuration, ThemeMode as DataTheme};

        let theme = |value: DataTheme| match value {
            DataTheme::Dark => ThemeMode::Dark,
            DataTheme::Light => ThemeMode::Light,
        };
        let warmth = |value: DataWarmth| match value {
            DataWarmth::Cold => WarmthLevel::Cold,
            DataWarmth::Cool => WarmthLevel::Cool,
            DataWarmth::Neutral => WarmthLevel::Neutral,
            DataWarmth::Warm => WarmthLevel::Warm,
            DataWarmth::Warmer => WarmthLevel::Warmer,
            DataWarmth::Warmest => WarmthLevel::Warmest,
        };
        let brightness = |value: DataBrightness| match value {
            DataBrightness::Dim => BrightnessLevel::Dim,
            DataBrightness::Dimmer => BrightnessLevel::Dimmer,
            DataBrightness::Mid => BrightnessLevel::Mid,
            DataBrightness::Bright => BrightnessLevel::Bright,
            DataBrightness::Brighter => BrightnessLevel::Brighter,
            DataBrightness::Brightest => BrightnessLevel::Brightest,
        };
        let kelvin = |value: i64| {
            u16::try_from(value).map(KelvinTemperature::new).map_err(|_| Error::Config {
                message: format!("kelvin must be a non-negative 16-bit integer, got {value}"),
            })
        };
        let percent = |value: i64| {
            u8::try_from(value).map(BrightnessPercent::new).map_err(|_| Error::Config {
                message: format!("brightness percent must be a non-negative 8-bit integer, got {value}"),
            })
        };
        let duration = |value: DataDuration| {
            match value {
                DataDuration::Minutes(value) => u32::try_from(value).map(crate::time::RampDuration::from_minutes),
                DataDuration::Seconds(value) => u64::try_from(value).map(crate::time::RampDuration::from_seconds),
            }
            .map_err(|_| Error::Config { message: "ramp duration must be non-negative".into() })
        };

        Ok(match request {
            generated::Request::SetTheme(value) => Self::SetTheme { mode: theme(value.mode) },
            generated::Request::GetTheme => Self::GetTheme,
            generated::Request::SetWarmth(value) => Self::SetWarmth { level: warmth(value.level) },
            generated::Request::SetWarmthKelvin(value) => Self::SetWarmthKelvin { kelvin: kelvin(value.kelvin)? },
            generated::Request::GetWarmth => Self::GetWarmth,
            generated::Request::StartWarmthRamp(value) => {
                Self::StartWarmthRamp { target: warmth(value.target), duration: duration(value.duration)? }
            }
            generated::Request::StartWarmthRampKelvin(value) => {
                Self::StartWarmthRampKelvin { target: kelvin(value.target)?, duration: duration(value.duration)? }
            }
            generated::Request::InterruptWarmth => Self::InterruptWarmth,
            generated::Request::SetBrightness(value) => Self::SetBrightness { level: brightness(value.level) },
            generated::Request::SetBrightnessPercent(value) => {
                Self::SetBrightnessPercent { percent: percent(value.percent)? }
            }
            generated::Request::GetBrightness => Self::GetBrightness,
            generated::Request::StartBrightnessRamp(value) => {
                Self::StartBrightnessRamp { target: brightness(value.target), duration: duration(value.duration)? }
            }
            generated::Request::StartBrightnessRampPercent(value) => {
                Self::StartBrightnessRampPercent { target: percent(value.target)?, duration: duration(value.duration)? }
            }
            generated::Request::InterruptBrightness => Self::InterruptBrightness,
            generated::Request::GetState => Self::GetState,
            generated::Request::GetSolarClock => Self::GetSolarClock,
        })
    }
}
