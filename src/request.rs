//! [`Request`] — what the CLI sends to the daemon.
//!
//! Parses from a single DOTOS record on argv (the `chroma` CLI's
//! one positional arg). Travels on the wire as a length-prefixed
//! rkyv archive over the daemon's UDS.

use dotos::{Block, Delimiter, DotosBlock, DotosDecode, DotosDecodeError, DotosEncode, DotosSource};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::{BrightnessLevel, BrightnessPercent};
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::time::RampDuration;
use crate::warmth::{KelvinTemperature, WarmthLevel};

/// What the CLI sends to the daemon.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq)]
pub enum Request {
    /// Switch to a theme mode through native concern actors.
    SetTheme { mode: ThemeMode },
    /// Read the current theme mode.
    GetTheme,

    /// Set warmth to a named preset (instant; cancels any active ramp).
    SetWarmth { level: WarmthLevel },
    /// Set warmth to an arbitrary kelvin value (instant; cancels any active ramp).
    SetWarmthKelvin { kelvin: KelvinTemperature },
    /// Read the current kelvin.
    GetWarmth,
    /// Begin a gradual warmth ramp toward `target` over `duration`,
    /// starting from the daemon's current temperature reading.
    /// Replaces any in-flight warmth ramp.
    StartWarmthRamp { target: WarmthLevel, duration: RampDuration },
    /// Like `StartWarmthRamp`, but with an arbitrary kelvin target.
    StartWarmthRampKelvin { target: KelvinTemperature, duration: RampDuration },
    /// Cancel any in-flight warmth ramp; the screen stays where it is.
    InterruptWarmth,

    /// Set brightness to a named preset (instant; cancels any active ramp).
    SetBrightness { level: BrightnessLevel },
    /// Set brightness to an arbitrary percent (instant; cancels any active ramp).
    SetBrightnessPercent { percent: BrightnessPercent },
    /// Read the current brightness percent.
    GetBrightness,
    /// Begin a gradual brightness ramp toward `target` over `duration`,
    /// starting from the daemon's current brightness reading.
    /// Replaces any in-flight brightness ramp.
    StartBrightnessRamp { target: BrightnessLevel, duration: RampDuration },
    /// Like `StartBrightnessRamp`, but with an arbitrary percent target.
    StartBrightnessRampPercent { target: BrightnessPercent, duration: RampDuration },
    /// Cancel any in-flight brightness ramp.
    InterruptBrightness,

    /// Read the full visual state (theme + warmth + brightness).
    GetState,
    /// Read the derived apparent-solar clock correction, without exposing a coordinate.
    GetSolarClock,
}

impl Request {
    /// Parse a single DOTOS record into a typed request.
    pub fn from_dotos(text: &str) -> Result<Self> {
        Ok(DotosSource::new(text).parse()?)
    }

    /// Render this request as a DOTOS record.
    pub fn to_dotos(&self) -> Result<String> {
        Ok(DotosEncode::to_dotos(self))
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

    fn decode_unit(tag: &str) -> std::result::Result<Self, DotosDecodeError> {
        match tag {
            "GetTheme" => Ok(Self::GetTheme),
            "GetWarmth" => Ok(Self::GetWarmth),
            "InterruptWarmth" => Ok(Self::InterruptWarmth),
            "GetBrightness" => Ok(Self::GetBrightness),
            "InterruptBrightness" => Ok(Self::InterruptBrightness),
            "GetState" => Ok(Self::GetState),
            "GetSolarClock" => Ok(Self::GetSolarClock),
            other => Err(DotosDecodeError::UnknownVariant { enum_name: "Request", variant: other.to_string() }),
        }
    }

    fn decode_tagged(tag: &str, payload: &[Block]) -> std::result::Result<Self, DotosDecodeError> {
        match tag {
            "GetTheme"
            | "GetWarmth"
            | "InterruptWarmth"
            | "GetBrightness"
            | "InterruptBrightness"
            | "GetState"
            | "GetSolarClock" => {
                Self::expect_payload_count(tag, payload, 0)?;
                Self::decode_unit(tag)
            }
            "SetTheme" => {
                Self::expect_payload_count(tag, payload, 1)?;
                Ok(Self::SetTheme { mode: Self::decode_payload(payload, 0)? })
            }
            "SetWarmth" => {
                Self::expect_payload_count(tag, payload, 1)?;
                Ok(Self::SetWarmth { level: Self::decode_payload(payload, 0)? })
            }
            "SetWarmthKelvin" => {
                Self::expect_payload_count(tag, payload, 1)?;
                Ok(Self::SetWarmthKelvin { kelvin: Self::decode_payload(payload, 0)? })
            }
            "StartWarmthRamp" => {
                Self::expect_payload_count(tag, payload, 2)?;
                Ok(Self::StartWarmthRamp {
                    target: Self::decode_payload(payload, 0)?,
                    duration: Self::decode_payload(payload, 1)?,
                })
            }
            "StartWarmthRampKelvin" => {
                Self::expect_payload_count(tag, payload, 2)?;
                Ok(Self::StartWarmthRampKelvin {
                    target: Self::decode_payload(payload, 0)?,
                    duration: Self::decode_payload(payload, 1)?,
                })
            }
            "SetBrightness" => {
                Self::expect_payload_count(tag, payload, 1)?;
                Ok(Self::SetBrightness { level: Self::decode_payload(payload, 0)? })
            }
            "SetBrightnessPercent" => {
                Self::expect_payload_count(tag, payload, 1)?;
                Ok(Self::SetBrightnessPercent { percent: Self::decode_payload(payload, 0)? })
            }
            "StartBrightnessRamp" => {
                Self::expect_payload_count(tag, payload, 2)?;
                Ok(Self::StartBrightnessRamp {
                    target: Self::decode_payload(payload, 0)?,
                    duration: Self::decode_payload(payload, 1)?,
                })
            }
            "StartBrightnessRampPercent" => {
                Self::expect_payload_count(tag, payload, 2)?;
                Ok(Self::StartBrightnessRampPercent {
                    target: Self::decode_payload(payload, 0)?,
                    duration: Self::decode_payload(payload, 1)?,
                })
            }
            other => Err(DotosDecodeError::UnknownVariant { enum_name: "Request", variant: other.to_string() }),
        }
    }

    fn expect_payload_count(
        _tag: &str,
        payload: &[Block],
        expected: usize,
    ) -> std::result::Result<(), DotosDecodeError> {
        if payload.len() == expected {
            Ok(())
        } else {
            Err(DotosDecodeError::ExpectedRootCount { type_name: "Request payload", expected, found: payload.len() })
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

impl DotosDecode for Request {
    fn from_dotos_block(block: &Block) -> std::result::Result<Self, DotosDecodeError> {
        if let Some(tag) = block.demote_to_string() {
            return Self::decode_unit(tag);
        }
        let (head, payload) = block.as_application().ok_or(DotosDecodeError::ExpectedDelimited {
            type_name: "Request",
            delimiter: "Request.(payload) application",
        })?;
        let tag = head.demote_to_string().ok_or(DotosDecodeError::ExpectedAtom { type_name: "Request variant" })?;
        let payload = DotosBlock::new(payload).expect_delimited(Delimiter::Parenthesis, "Request")?;
        Self::decode_tagged(tag, payload)
    }
}

impl DotosEncode for Request {
    fn to_dotos(&self) -> String {
        match self {
            Self::SetTheme { mode } => Self::tagged("SetTheme", [mode.to_dotos()]),
            Self::GetTheme => "GetTheme".to_owned(),
            Self::SetWarmth { level } => Self::tagged("SetWarmth", [level.to_dotos()]),
            Self::SetWarmthKelvin { kelvin } => Self::tagged("SetWarmthKelvin", [kelvin.to_dotos()]),
            Self::GetWarmth => "GetWarmth".to_owned(),
            Self::StartWarmthRamp { target, duration } => {
                Self::tagged("StartWarmthRamp", [target.to_dotos(), duration.to_dotos()])
            }
            Self::StartWarmthRampKelvin { target, duration } => {
                Self::tagged("StartWarmthRampKelvin", [target.to_dotos(), duration.to_dotos()])
            }
            Self::InterruptWarmth => "InterruptWarmth".to_owned(),
            Self::SetBrightness { level } => Self::tagged("SetBrightness", [level.to_dotos()]),
            Self::SetBrightnessPercent { percent } => Self::tagged("SetBrightnessPercent", [percent.to_dotos()]),
            Self::GetBrightness => "GetBrightness".to_owned(),
            Self::StartBrightnessRamp { target, duration } => {
                Self::tagged("StartBrightnessRamp", [target.to_dotos(), duration.to_dotos()])
            }
            Self::StartBrightnessRampPercent { target, duration } => {
                Self::tagged("StartBrightnessRampPercent", [target.to_dotos(), duration.to_dotos()])
            }
            Self::InterruptBrightness => "InterruptBrightness".to_owned(),
            Self::GetState => "GetState".to_owned(),
            Self::GetSolarClock => "GetSolarClock".to_owned(),
        }
    }
}
