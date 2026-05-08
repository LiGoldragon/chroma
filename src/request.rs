//! [`Request`] — what the CLI sends to the daemon.
//!
//! Parses from a single NOTA record on argv (the `chroma` CLI's
//! one positional arg). Travels on the wire as a length-prefixed
//! rkyv archive over the daemon's UDS.

use nota_codec::{Decoder, Encoder, NexusVerb, NotaDecode, NotaEncode};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::{BrightnessLevel, BrightnessPercent};
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::warmth::{KelvinTemperature, WarmthLevel};

/// What the CLI sends to the daemon.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NexusVerb, Debug, Clone, PartialEq)]
pub enum Request {
    /// Switch to a theme mode (apply runs the configured shell script).
    SetTheme { mode: ThemeMode },
    /// Read the current theme mode.
    GetTheme {},
    /// Set warmth to a named preset.
    SetWarmth { level: WarmthLevel },
    /// Set warmth to an arbitrary kelvin value.
    SetWarmthKelvin { kelvin: KelvinTemperature },
    /// Read the current kelvin.
    GetWarmth {},
    /// Set brightness to a named preset.
    SetBrightness { level: BrightnessLevel },
    /// Set brightness to an arbitrary percent.
    SetBrightnessPercent { percent: BrightnessPercent },
    /// Read the current brightness percent.
    GetBrightness {},
    /// Read the full visual state (theme + warmth + brightness).
    GetState {},
}

impl Request {
    /// Parse a single NOTA record into a typed request.
    pub fn from_nota(text: &str) -> Result<Self> {
        let mut decoder = Decoder::nota(text);
        let request = <Self as NotaDecode>::decode(&mut decoder)?;
        Ok(request)
    }

    /// Render this request as a NOTA record.
    pub fn to_nota(&self) -> Result<String> {
        let mut encoder = Encoder::nota();
        <Self as NotaEncode>::encode(self, &mut encoder)?;
        Ok(encoder.into_string())
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
}
