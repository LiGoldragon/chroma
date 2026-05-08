//! [`Response`] — what the daemon sends back to the CLI.
//!
//! Travels on the wire as a length-prefixed rkyv archive; the
//! CLI prints it as a single NOTA record.

use nota_codec::{Decoder, Encoder, NexusVerb, NotaDecode, NotaEncode};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::BrightnessPercent;
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::warmth::KelvinTemperature;

/// What the daemon sends back.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NexusVerb, Debug, Clone, PartialEq)]
pub enum Response {
    /// The request landed; no payload needed.
    Acked {},
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
        let mut encoder = Encoder::nota();
        <Self as NotaEncode>::encode(self, &mut encoder)?;
        Ok(encoder.into_string())
    }

    /// Parse from a NOTA record (used in tests).
    pub fn from_nota(text: &str) -> Result<Self> {
        let mut decoder = Decoder::nota(text);
        let response = <Self as NotaDecode>::decode(&mut decoder)?;
        Ok(response)
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
