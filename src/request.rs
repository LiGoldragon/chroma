//! [`Request`] — what the CLI sends to the daemon.
//!
//! Parses from a single NOTA record on argv (the `chroma` CLI's
//! one positional arg). Travels on the wire as a length-prefixed
//! rkyv archive over the daemon's UDS.

use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode, NotaSum};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::{BrightnessLevel, BrightnessPercent};
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::time::RampDuration;
use crate::warmth::{KelvinTemperature, WarmthLevel};

/// What the CLI sends to the daemon.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaSum, Debug, Clone, PartialEq)]
pub enum Request {
    /// Switch to a theme mode through native concern actors.
    SetTheme { mode: ThemeMode },
    /// Read the current theme mode.
    GetTheme {},

    /// Set warmth to a named preset (instant; cancels any active ramp).
    SetWarmth { level: WarmthLevel },
    /// Set warmth to an arbitrary kelvin value (instant; cancels any active ramp).
    SetWarmthKelvin { kelvin: KelvinTemperature },
    /// Read the current kelvin.
    GetWarmth {},
    /// Begin a gradual warmth ramp toward `target` over `duration`,
    /// starting from the daemon's current temperature reading.
    /// Replaces any in-flight warmth ramp.
    StartWarmthRamp { target: WarmthLevel, duration: RampDuration },
    /// Like `StartWarmthRamp`, but with an arbitrary kelvin target.
    StartWarmthRampKelvin { target: KelvinTemperature, duration: RampDuration },
    /// Cancel any in-flight warmth ramp; the screen stays where it is.
    InterruptWarmth {},

    /// Set brightness to a named preset (instant; cancels any active ramp).
    SetBrightness { level: BrightnessLevel },
    /// Set brightness to an arbitrary percent (instant; cancels any active ramp).
    SetBrightnessPercent { percent: BrightnessPercent },
    /// Read the current brightness percent.
    GetBrightness {},
    /// Begin a gradual brightness ramp toward `target` over `duration`,
    /// starting from the daemon's current brightness reading.
    /// Replaces any in-flight brightness ramp.
    StartBrightnessRamp { target: BrightnessLevel, duration: RampDuration },
    /// Like `StartBrightnessRamp`, but with an arbitrary percent target.
    StartBrightnessRampPercent { target: BrightnessPercent, duration: RampDuration },
    /// Cancel any in-flight brightness ramp.
    InterruptBrightness {},

    /// Read the full visual state (theme + warmth + brightness).
    GetState {},
}

impl Request {
    /// Parse a single NOTA record into a typed request.
    pub fn from_nota(text: &str) -> Result<Self> {
        let mut decoder = Decoder::new(text);
        let request = <Self as NotaDecode>::decode(&mut decoder)?;
        Ok(request)
    }

    /// Render this request as a NOTA record.
    pub fn to_nota(&self) -> Result<String> {
        let mut encoder = Encoder::new();
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
