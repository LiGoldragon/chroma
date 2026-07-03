//! [`Request`] — what the CLI sends to the daemon.
//!
//! Parses from a single NOTA record on argv (the `chroma` CLI's
//! one positional arg). Travels on the wire as a length-prefixed
//! rkyv archive over the daemon's UDS.

use nota::{Block, Delimiter, NotaBlock, NotaDecode, NotaDecodeError, NotaEncode, NotaSource};
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
}

impl Request {
    /// Parse a single NOTA record into a typed request.
    pub fn from_nota(text: &str) -> Result<Self> {
        Ok(NotaSource::new(text).parse()?)
    }

    /// Render this request as a NOTA record.
    pub fn to_nota(&self) -> Result<String> {
        Ok(NotaEncode::to_nota(self))
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

    fn decode_unit(tag: &str) -> std::result::Result<Self, NotaDecodeError> {
        match tag {
            "GetTheme" => Ok(Self::GetTheme),
            "GetWarmth" => Ok(Self::GetWarmth),
            "InterruptWarmth" => Ok(Self::InterruptWarmth),
            "GetBrightness" => Ok(Self::GetBrightness),
            "InterruptBrightness" => Ok(Self::InterruptBrightness),
            "GetState" => Ok(Self::GetState),
            other => Err(NotaDecodeError::UnknownVariant { enum_name: "Request", variant: other.to_string() }),
        }
    }

    fn decode_tagged(children: &[Block]) -> std::result::Result<Self, NotaDecodeError> {
        let tag = Self::tag(children)?;
        let payload = &children[1..];
        match tag {
            "GetTheme" | "GetWarmth" | "InterruptWarmth" | "GetBrightness" | "InterruptBrightness" | "GetState" => {
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
            other => Err(NotaDecodeError::UnknownVariant { enum_name: "Request", variant: other.to_string() }),
        }
    }

    fn tag(children: &[Block]) -> std::result::Result<&str, NotaDecodeError> {
        children
            .first()
            .and_then(Block::demote_to_string)
            .ok_or(NotaDecodeError::ExpectedAtom { type_name: "Request variant" })
    }

    fn expect_payload_count(
        _tag: &str,
        payload: &[Block],
        expected: usize,
    ) -> std::result::Result<(), NotaDecodeError> {
        if payload.len() == expected {
            Ok(())
        } else {
            Err(NotaDecodeError::ExpectedRootCount { type_name: "Request payload", expected, found: payload.len() })
        }
    }

    fn decode_payload<Value: NotaDecode>(
        payload: &[Block],
        index: usize,
    ) -> std::result::Result<Value, NotaDecodeError> {
        Value::from_nota_block(&payload[index])
    }

    fn tagged(tag: &'static str, payload: impl IntoIterator<Item = String>) -> String {
        let mut fields = Vec::new();
        fields.push(tag.to_owned());
        fields.extend(payload);
        Delimiter::Parenthesis.wrap(fields)
    }
}

impl NotaDecode for Request {
    fn from_nota_block(block: &Block) -> std::result::Result<Self, NotaDecodeError> {
        if let Some(tag) = block.demote_to_string() {
            return Self::decode_unit(tag);
        }
        let children = NotaBlock::new(block).expect_delimited(Delimiter::Parenthesis, "Request")?;
        Self::decode_tagged(children)
    }
}

impl NotaEncode for Request {
    fn to_nota(&self) -> String {
        match self {
            Self::SetTheme { mode } => Self::tagged("SetTheme", [mode.to_nota()]),
            Self::GetTheme => "GetTheme".to_owned(),
            Self::SetWarmth { level } => Self::tagged("SetWarmth", [level.to_nota()]),
            Self::SetWarmthKelvin { kelvin } => Self::tagged("SetWarmthKelvin", [kelvin.to_nota()]),
            Self::GetWarmth => "GetWarmth".to_owned(),
            Self::StartWarmthRamp { target, duration } => {
                Self::tagged("StartWarmthRamp", [target.to_nota(), duration.to_nota()])
            }
            Self::StartWarmthRampKelvin { target, duration } => {
                Self::tagged("StartWarmthRampKelvin", [target.to_nota(), duration.to_nota()])
            }
            Self::InterruptWarmth => "InterruptWarmth".to_owned(),
            Self::SetBrightness { level } => Self::tagged("SetBrightness", [level.to_nota()]),
            Self::SetBrightnessPercent { percent } => Self::tagged("SetBrightnessPercent", [percent.to_nota()]),
            Self::GetBrightness => "GetBrightness".to_owned(),
            Self::StartBrightnessRamp { target, duration } => {
                Self::tagged("StartBrightnessRamp", [target.to_nota(), duration.to_nota()])
            }
            Self::StartBrightnessRampPercent { target, duration } => {
                Self::tagged("StartBrightnessRampPercent", [target.to_nota(), duration.to_nota()])
            }
            Self::InterruptBrightness => "InterruptBrightness".to_owned(),
            Self::GetState => "GetState".to_owned(),
        }
    }
}
