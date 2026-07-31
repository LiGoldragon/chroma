//! [`Response`] — what the daemon sends back to the CLI.
//!
//! Travels on the wire as a length-prefixed rkyv archive; the
//! CLI prints it as a single DOTOS record.

use dotos::{Block, Delimiter, DotosBlock, DotosDecode, DotosDecodeError, DotosEncode, DotosSource};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::BrightnessPercent;
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::warmth::KelvinTemperature;

/// What the daemon sends back.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq)]
pub enum Response {
    /// The daemon accepted the request; the side effect may still be running.
    Accepted,
    /// The current theme.
    Theme { mode: ThemeMode },
    /// The current warmth, as a kelvin reading.
    Warmth { kelvin: KelvinTemperature },
    /// The current brightness, as a percent reading.
    Brightness { percent: BrightnessPercent },
    /// The full visual state.
    State { theme: ThemeMode, kelvin: KelvinTemperature, percent: BrightnessPercent },
    /// Derived UTC correction for local apparent solar time; no coordinate crosses this boundary.
    ///
    /// The second positional wire value is only the UTC-day boundary for the
    /// equation-of-time calculation. GeoClue freshness is represented solely
    /// by `SolarClockUnavailable`, never by this field.
    SolarClock { utc_offset_seconds: i32, equation_of_time_valid_until_unix_seconds: i64 },
    /// No fresh authoritative GeoClue fix is available for solar-time projection.
    SolarClockUnavailable,
    /// The daemon refused the request.
    Error { message: String },
}

impl Response {
    /// Render as DOTOS for the CLI to print.
    pub fn to_dotos(&self) -> Result<String> {
        Ok(DotosEncode::to_dotos(self))
    }

    /// Parse from a DOTOS record (used in tests).
    pub fn from_dotos(text: &str) -> Result<Self> {
        Ok(DotosSource::new(text).parse()?)
    }

    /// Archive into rkyv bytes for the wire.
    pub fn archive(&self) -> Result<Vec<u8>> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(|err| Error::RkyvCodec(err.to_string()))
    }

    /// Reconstruct from an rkyv archive coming off the wire.
    pub fn from_archive(bytes: &[u8]) -> Result<Self> {
        rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes).map_err(|err| Error::RkyvCodec(err.to_string()))
    }

    fn decode_tagged(tag: &str, payload: &[Block]) -> std::result::Result<Self, DotosDecodeError> {
        match tag {
            "Accepted" => {
                Self::expect_payload_count("Accepted", payload, 0)?;
                Ok(Self::Accepted)
            }
            "Theme" => {
                Self::expect_payload_count("Theme", payload, 1)?;
                Ok(Self::Theme { mode: Self::decode_payload(payload, 0)? })
            }
            "Warmth" => {
                Self::expect_payload_count("Warmth", payload, 1)?;
                Ok(Self::Warmth { kelvin: Self::decode_payload(payload, 0)? })
            }
            "Brightness" => {
                Self::expect_payload_count("Brightness", payload, 1)?;
                Ok(Self::Brightness { percent: Self::decode_payload(payload, 0)? })
            }
            "State" => {
                Self::expect_payload_count("State", payload, 3)?;
                Ok(Self::State {
                    theme: Self::decode_payload(payload, 0)?,
                    kelvin: Self::decode_payload(payload, 1)?,
                    percent: Self::decode_payload(payload, 2)?,
                })
            }
            "SolarClock" => {
                Self::expect_payload_count("SolarClock", payload, 2)?;
                Ok(Self::SolarClock {
                    utc_offset_seconds: Self::decode_payload(payload, 0)?,
                    equation_of_time_valid_until_unix_seconds: Self::decode_payload(payload, 1)?,
                })
            }
            "SolarClockUnavailable" => {
                Self::expect_payload_count("SolarClockUnavailable", payload, 0)?;
                Ok(Self::SolarClockUnavailable)
            }
            "Error" => {
                Self::expect_payload_count("Error", payload, 1)?;
                Ok(Self::Error { message: Self::decode_payload(payload, 0)? })
            }
            other => Err(DotosDecodeError::UnknownVariant { enum_name: "Response", variant: other.to_string() }),
        }
    }

    fn expect_payload_count(
        type_name: &'static str,
        payload: &[Block],
        expected: usize,
    ) -> std::result::Result<(), DotosDecodeError> {
        if payload.len() == expected {
            Ok(())
        } else {
            Err(DotosDecodeError::ExpectedRootCount { type_name, expected, found: payload.len() })
        }
    }

    fn decode_payload<Value: DotosDecode>(
        payload: &[Block],
        index: usize,
    ) -> std::result::Result<Value, DotosDecodeError> {
        Value::from_dotos_block(&payload[index])
    }

    fn tagged(tag: &'static str, payload: impl IntoIterator<Item = String>) -> String {
        format!("{tag}.{}", Delimiter::Parenthesis.wrap(payload))
    }
}

impl DotosDecode for Response {
    fn from_dotos_block(block: &Block) -> std::result::Result<Self, DotosDecodeError> {
        if let Some(tag) = block.demote_to_string() {
            return match tag {
                "Accepted" => Ok(Self::Accepted),
                "SolarClockUnavailable" => Ok(Self::SolarClockUnavailable),
                other => Err(DotosDecodeError::UnknownVariant { enum_name: "Response", variant: other.to_string() }),
            };
        }
        let (head, payload) = block.as_application().ok_or(DotosDecodeError::ExpectedDelimited {
            type_name: "Response",
            delimiter: "Response.(payload) application",
        })?;
        let tag = head.demote_to_string().ok_or(DotosDecodeError::ExpectedAtom { type_name: "Response variant" })?;
        let payload = DotosBlock::new(payload).expect_delimited(Delimiter::Parenthesis, "Response")?;
        Self::decode_tagged(tag, payload)
    }
}

impl DotosEncode for Response {
    fn to_dotos(&self) -> String {
        match self {
            Self::Accepted => "Accepted".to_owned(),
            Self::Theme { mode } => Self::tagged("Theme", [mode.to_dotos()]),
            Self::Warmth { kelvin } => Self::tagged("Warmth", [kelvin.to_dotos()]),
            Self::Brightness { percent } => Self::tagged("Brightness", [percent.to_dotos()]),
            Self::State { theme, kelvin, percent } => {
                Self::tagged("State", [theme.to_dotos(), kelvin.to_dotos(), percent.to_dotos()])
            }
            Self::SolarClock { utc_offset_seconds, equation_of_time_valid_until_unix_seconds } => Self::tagged(
                "SolarClock",
                [utc_offset_seconds.to_dotos(), equation_of_time_valid_until_unix_seconds.to_dotos()],
            ),
            Self::SolarClockUnavailable => "SolarClockUnavailable".to_owned(),
            Self::Error { message } => Self::tagged("Error", [message.to_dotos()]),
        }
    }
}
