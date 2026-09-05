//! [`Response`] — Chroma's internal rkyv reply frame.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::BrightnessPercent;
use crate::error::{Error, Result};
use crate::generated;
use crate::theme::ThemeMode;
use crate::warmth::KelvinTemperature;

/// What the daemon sends back over the local rkyv socket frame.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq)]
pub enum Response {
    Accepted,
    Theme { mode: ThemeMode },
    Warmth { kelvin: KelvinTemperature },
    Brightness { percent: BrightnessPercent },
    State { theme: ThemeMode, kelvin: KelvinTemperature, percent: BrightnessPercent },
    SolarClock { utc_offset_seconds: i32, equation_of_time_valid_until_unix_seconds: i64 },
    SolarClockUnavailable,
    Error { message: String },
}

impl Response {
    pub fn archive(&self) -> Result<Vec<u8>> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(|err| Error::RkyvCodec(err.to_string()))
    }

    pub fn from_archive(bytes: &[u8]) -> Result<Self> {
        rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes).map_err(|err| Error::RkyvCodec(err.to_string()))
    }
}

impl TryFrom<Response> for generated::Reply {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self> {
        let theme = |value: ThemeMode| match value {
            ThemeMode::Dark => generated::ThemeMode::Dark,
            ThemeMode::Light => generated::ThemeMode::Light,
        };
        Ok(match response {
            Response::Accepted => Self::Accepted,
            Response::Theme { mode } => Self::Theme(generated::ReplyTheme(theme(mode))),
            Response::Warmth { kelvin } => {
                Self::Warmth(generated::ReplyWarmth(protos::Integer::from(i64::from(kelvin.as_u16()))))
            }
            Response::Brightness { percent } => {
                Self::Brightness(generated::ReplyBrightness(protos::Integer::from(i64::from(percent.as_u8()))))
            }
            Response::State { theme: value, kelvin, percent } => Self::State(generated::ReplyState(
                theme(value),
                protos::Integer::from(i64::from(kelvin.as_u16())),
                protos::Integer::from(i64::from(percent.as_u8())),
            )),
            Response::SolarClock { utc_offset_seconds, equation_of_time_valid_until_unix_seconds } => {
                Self::SolarClock(generated::ReplySolarClock(
                    protos::Integer::from(i64::from(utc_offset_seconds)),
                    protos::Integer::from(equation_of_time_valid_until_unix_seconds),
                ))
            }
            Response::SolarClockUnavailable => Self::SolarClockUnavailable,
            Response::Error { message } => {
                Self::Error(generated::ReplyError(protos::Text::try_from(message).map_err(|error| Error::Config {
                    message: format!("daemon reply cannot be rendered as Datom: {error:?}"),
                })?))
            }
        })
    }
}
