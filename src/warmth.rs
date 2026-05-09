//! [`WarmthLevel`] and [`KelvinTemperature`] — the warmth axis.
//!
//! Six discrete preset levels ([`WarmthLevel::Cold`] through
//! [`WarmthLevel::Warmest`]) over a kelvin range
//! `[KelvinTemperature::MIN, KelvinTemperature::MAX]` =
//! `[1000, 10000]`. The wl-gammarelay-rs `Temperature` DBus
//! property is a `q` (u16); [`KelvinTemperature::as_u16`] is the
//! wire form.
//!
//! This module also names the axis's scheduling shape:
//! [`WarmthWaypoint`], [`WarmthSchedule`], [`WarmthAxis`].

use core::fmt;

use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode, NotaEnum};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::time::{RampDuration, RampTrigger};

/// A discrete warmth level on the daemon's standard ladder.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, NotaEnum, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum WarmthLevel {
    Cold,
    Cool,
    #[default]
    Neutral,
    Warm,
    Warmer,
    Warmest,
}

impl WarmthLevel {
    /// The canonical kelvin value for this preset.
    pub const fn kelvin(self) -> KelvinTemperature {
        match self {
            WarmthLevel::Cold => KelvinTemperature(6500),
            WarmthLevel::Cool => KelvinTemperature(5500),
            WarmthLevel::Neutral => KelvinTemperature(4500),
            WarmthLevel::Warm => KelvinTemperature(3700),
            WarmthLevel::Warmer => KelvinTemperature(3200),
            WarmthLevel::Warmest => KelvinTemperature(2700),
        }
    }

    /// The next-warmer preset. Saturates at [`WarmthLevel::Warmest`].
    pub const fn warmer(self) -> Self {
        match self {
            WarmthLevel::Cold => WarmthLevel::Cool,
            WarmthLevel::Cool => WarmthLevel::Neutral,
            WarmthLevel::Neutral => WarmthLevel::Warm,
            WarmthLevel::Warm => WarmthLevel::Warmer,
            WarmthLevel::Warmer => WarmthLevel::Warmest,
            WarmthLevel::Warmest => WarmthLevel::Warmest,
        }
    }

    /// The next-cooler preset. Saturates at [`WarmthLevel::Cold`].
    pub const fn cooler(self) -> Self {
        match self {
            WarmthLevel::Cold => WarmthLevel::Cold,
            WarmthLevel::Cool => WarmthLevel::Cold,
            WarmthLevel::Neutral => WarmthLevel::Cool,
            WarmthLevel::Warm => WarmthLevel::Neutral,
            WarmthLevel::Warmer => WarmthLevel::Warm,
            WarmthLevel::Warmest => WarmthLevel::Warmer,
        }
    }

    /// Lowercase name (for human-facing display and CLI replies).
    pub const fn as_str(self) -> &'static str {
        match self {
            WarmthLevel::Cold => "cold",
            WarmthLevel::Cool => "cool",
            WarmthLevel::Neutral => "neutral",
            WarmthLevel::Warm => "warm",
            WarmthLevel::Warmer => "warmer",
            WarmthLevel::Warmest => "warmest",
        }
    }
}

impl fmt::Display for WarmthLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A colour-temperature value in kelvins.
///
/// The wire form for wl-gammarelay-rs's `Temperature` (q) DBus
/// property. All construction paths — including [`NotaDecode`]
/// — clamp to the daemon's accepted range `[MIN, MAX]` =
/// `[1000, 10000]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct KelvinTemperature(u16);

impl KelvinTemperature {
    /// The lowest accepted value (warmest extreme).
    pub const MIN: Self = Self(1000);
    /// The highest accepted value (coldest extreme).
    pub const MAX: Self = Self(10000);

    /// Construct a kelvin value, clamping to `[MIN, MAX]`.
    pub const fn new(value: u16) -> Self {
        let clamped = if value < Self::MIN.0 {
            Self::MIN.0
        } else if value > Self::MAX.0 {
            Self::MAX.0
        } else {
            value
        };
        Self(clamped)
    }

    /// The integer kelvin value, suitable for the wl-gammarelay-rs
    /// `Temperature` (q) DBus property.
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Linear interpolation from `self` toward `target` at
    /// `fraction` ∈ `[0, 1]` (clamped at the boundaries).
    pub fn lerp_to(self, target: Self, fraction: f64) -> Self {
        let fraction = fraction.clamp(0.0, 1.0);
        let from = self.0 as f64;
        let to = target.0 as f64;
        let interpolated = from + (to - from) * fraction;
        Self::new(interpolated.round() as u16)
    }
}

impl fmt::Display for KelvinTemperature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}K", self.0)
    }
}

// Hand-written NOTA codec — routes decode through `new` so
// out-of-range values clamp consistently. NotaTransparent would
// bypass the clamp.
impl NotaEncode for KelvinTemperature {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        encoder.write_u64(self.0 as u64)
    }
}

impl NotaDecode for KelvinTemperature {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let raw = decoder.read_u16()?;
        Ok(Self::new(raw))
    }
}

/// One scheduled warmth waypoint — at this trigger, ramp to
/// this level over this duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WarmthWaypoint {
    pub trigger: RampTrigger,
    pub target: WarmthLevel,
    pub ramp_duration: RampDuration,
}

/// The warmth axis's schedule.
///
/// Either a single [`Manual`](WarmthSchedule::Manual) value (no
/// scheduled fires), or [`Scheduled`](WarmthSchedule::Scheduled)
/// waypoints plus a default level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarmthSchedule {
    Manual(WarmthLevel),
    Scheduled { waypoints: Vec<WarmthWaypoint>, default: WarmthLevel },
}

impl WarmthSchedule {
    /// Whether any waypoint needs geolocation.
    pub fn needs_geolocation(&self) -> bool {
        match self {
            WarmthSchedule::Manual(_) => false,
            WarmthSchedule::Scheduled { waypoints, .. } => {
                waypoints.iter().any(|waypoint| waypoint.trigger.requires_geolocation())
            }
        }
    }

    /// The level that holds when no waypoint applies.
    pub fn default_level(&self) -> WarmthLevel {
        match self {
            WarmthSchedule::Manual(level) => *level,
            WarmthSchedule::Scheduled { default, .. } => *default,
        }
    }
}

/// The full warmth-axis configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarmthAxis {
    pub schedule: WarmthSchedule,
}
