//! [`Response`] — what the daemon sends back to the CLI.
//!
//! Travels on the wire as a length-prefixed rkyv archive; the
//! CLI prints it as a single NOTA record.

use nota::{Block, Delimiter, NotaBlock, NotaDecode, NotaDecodeError, NotaEncode, NotaSource};
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
    /// The daemon refused the request.
    Error { message: String },
}

impl Response {
    /// Render as NOTA for the CLI to print.
    pub fn to_nota(&self) -> Result<String> {
        Ok(NotaEncode::to_nota(self))
    }

    /// Parse from a NOTA record (used in tests).
    pub fn from_nota(text: &str) -> Result<Self> {
        Ok(NotaSource::new(text).parse()?)
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

    fn decode_tagged(children: &[Block]) -> std::result::Result<Self, NotaDecodeError> {
        let tag = children
            .first()
            .and_then(Block::demote_to_string)
            .ok_or(NotaDecodeError::ExpectedAtom { type_name: "Response variant" })?;
        let payload = &children[1..];
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
            "Error" => {
                Self::expect_payload_count("Error", payload, 1)?;
                Ok(Self::Error { message: Self::decode_payload(payload, 0)? })
            }
            other => Err(NotaDecodeError::UnknownVariant { enum_name: "Response", variant: other.to_string() }),
        }
    }

    fn expect_payload_count(
        type_name: &'static str,
        payload: &[Block],
        expected: usize,
    ) -> std::result::Result<(), NotaDecodeError> {
        if payload.len() == expected {
            Ok(())
        } else {
            Err(NotaDecodeError::ExpectedRootCount { type_name, expected, found: payload.len() })
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

impl NotaDecode for Response {
    fn from_nota_block(block: &Block) -> std::result::Result<Self, NotaDecodeError> {
        if let Some(tag) = block.demote_to_string() {
            return match tag {
                "Accepted" => Ok(Self::Accepted),
                other => Err(NotaDecodeError::UnknownVariant { enum_name: "Response", variant: other.to_string() }),
            };
        }
        let children = NotaBlock::new(block).expect_delimited(Delimiter::Parenthesis, "Response")?;
        Self::decode_tagged(children)
    }
}

impl NotaEncode for Response {
    fn to_nota(&self) -> String {
        match self {
            Self::Accepted => "Accepted".to_owned(),
            Self::Theme { mode } => Self::tagged("Theme", [mode.to_nota()]),
            Self::Warmth { kelvin } => Self::tagged("Warmth", [kelvin.to_nota()]),
            Self::Brightness { percent } => Self::tagged("Brightness", [percent.to_nota()]),
            Self::State { theme, kelvin, percent } => {
                Self::tagged("State", [theme.to_nota(), kelvin.to_nota(), percent.to_nota()])
            }
            Self::Error { message } => Self::tagged("Error", [message.to_nota()]),
        }
    }
}
