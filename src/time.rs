//! Time-axis primitives for the daemon's schedule.
//!
//! [`RampDuration`] — a positive duration used for gradual
//! transitions. [`SignedMinutes`] — a signed offset applied to
//! civil twilight events. [`LocalHour`] / [`LocalMinute`] — a
//! wall-clock time of day. [`RampTrigger`] — the closed enum of
//! waypoint trigger kinds.

use core::fmt;
use core::time::Duration;

use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// A positive duration over which a ramped axis interpolates.
///
/// Stored as whole seconds (`u64`) so it round-trips cleanly
/// through both rkyv (the wire) and NOTA (the CLI argv).
/// Construction clamps to ≥ 1 second so the lerp denominator
/// is never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct RampDuration(u64);

impl RampDuration {
    /// The shortest accepted ramp (1 second).
    pub const MIN: Self = Self(1);

    /// Construct from whole minutes, clamping to [`Self::MIN`].
    pub fn from_minutes(minutes: u32) -> Self {
        let seconds = u64::from(minutes).saturating_mul(60);
        Self::from_seconds(seconds)
    }

    /// Construct from whole seconds, clamping to [`Self::MIN`].
    pub const fn from_seconds(seconds: u64) -> Self {
        if seconds < 1 { Self::MIN } else { Self(seconds) }
    }

    /// The underlying [`Duration`].
    pub const fn as_duration(self) -> Duration {
        Duration::from_secs(self.0)
    }

    /// The duration in whole seconds.
    pub const fn as_seconds(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RampDuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.0;
        if total >= 60 {
            let minutes = total / 60;
            let seconds = total % 60;
            if seconds == 0 { write!(formatter, "{minutes}m") } else { write!(formatter, "{minutes}m{seconds}s") }
        } else {
            write!(formatter, "{total}s")
        }
    }
}

// NOTA wire form: `(Minutes <n>)` or `(Seconds <n>)`. Encoded
// in whichever form fits cleanly: if the duration is a whole
// number of minutes, emit `(Minutes <n>)`; otherwise emit
// `(Seconds <n>)`. Decoder accepts either.
impl NotaEncode for RampDuration {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        if self.0.is_multiple_of(60) {
            encoder.start_record("Minutes")?;
            encoder.write_u64(self.0 / 60)?;
            encoder.end_record()
        } else {
            encoder.start_record("Seconds")?;
            encoder.write_u64(self.0)?;
            encoder.end_record()
        }
    }
}

impl NotaDecode for RampDuration {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Minutes" => {
                decoder.expect_record_head("Minutes")?;
                let minutes = decoder.read_u64()?;
                decoder.expect_record_end()?;
                Ok(Self::from_seconds(minutes.saturating_mul(60)))
            }
            "Seconds" => {
                decoder.expect_record_head("Seconds")?;
                let seconds = decoder.read_u64()?;
                decoder.expect_record_end()?;
                Ok(Self::from_seconds(seconds))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb { verb: "RampDuration", got: other.to_string() }),
        }
    }
}

/// A signed minute offset applied to civil twilight events.
///
/// Range `[MIN, MAX]` = `[-720, 720]` (±12 hours), clamped at
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignedMinutes(i16);

impl SignedMinutes {
    /// The most-negative accepted offset (12 hours before).
    pub const MIN: Self = Self(-720);
    /// The most-positive accepted offset (12 hours after).
    pub const MAX: Self = Self(720);
    /// Zero offset.
    pub const ZERO: Self = Self(0);

    /// Construct, clamping to `[MIN, MAX]`.
    pub const fn new(value: i16) -> Self {
        let clamped = if value < Self::MIN.0 {
            Self::MIN.0
        } else if value > Self::MAX.0 {
            Self::MAX.0
        } else {
            value
        };
        Self(clamped)
    }

    /// The signed integer minute count.
    pub const fn as_i16(self) -> i16 {
        self.0
    }

    /// Whether the offset is negative.
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl fmt::Display for SignedMinutes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 0 { write!(formatter, "+{}m", self.0) } else { write!(formatter, "{}m", self.0) }
    }
}

impl Default for SignedMinutes {
    fn default() -> Self {
        Self::ZERO
    }
}

/// A wall-clock hour, `[0, 23]` (clamped at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalHour(u8);

impl LocalHour {
    /// Midnight.
    pub const MIN: Self = Self(0);
    /// 23:00.
    pub const MAX: Self = Self(23);

    /// Construct, clamping to `[MIN, MAX]`.
    pub const fn new(value: u8) -> Self {
        let clamped = if value > Self::MAX.0 { Self::MAX.0 } else { value };
        Self(clamped)
    }

    /// The integer hour.
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl fmt::Display for LocalHour {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}", self.0)
    }
}

/// A wall-clock minute, `[0, 59]` (clamped at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalMinute(u8);

impl LocalMinute {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(59);

    pub const fn new(value: u8) -> Self {
        let clamped = if value > Self::MAX.0 { Self::MAX.0 } else { value };
        Self(clamped)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl fmt::Display for LocalMinute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}", self.0)
    }
}

/// When in the day a waypoint fires.
///
/// `CivilDawn` and `CivilDusk` resolve to absolute times only
/// once a geolocation is known. `TimeOfDay` resolves directly
/// against the wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RampTrigger {
    /// Civil dawn (sun 6° below horizon at sunrise) plus the offset.
    CivilDawn(SignedMinutes),
    /// Civil dusk (sun 6° below horizon at sunset) plus the offset.
    CivilDusk(SignedMinutes),
    /// A wall-clock time of day, every day.
    TimeOfDay(LocalHour, LocalMinute),
}

impl RampTrigger {
    /// Whether this trigger needs a geolocation to resolve.
    pub const fn requires_geolocation(self) -> bool {
        matches!(self, RampTrigger::CivilDawn(_) | RampTrigger::CivilDusk(_))
    }
}

impl fmt::Display for RampTrigger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RampTrigger::CivilDawn(offset) => write!(formatter, "civil-dawn {offset}"),
            RampTrigger::CivilDusk(offset) => write!(formatter, "civil-dusk {offset}"),
            RampTrigger::TimeOfDay(hour, minute) => write!(formatter, "{hour}:{minute}"),
        }
    }
}
