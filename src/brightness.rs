//! [`BrightnessLevel`] and [`BrightnessPercent`] — the brightness axis.
//!
//! Six discrete preset levels ([`BrightnessLevel::Dim`] through
//! [`BrightnessLevel::Brightest`]) over the percent range `[0, 100]`.
//! wl-gammarelay-rs's `Brightness` DBus property is a `d` (double)
//! in `[0.0, 1.0]`; [`BrightnessPercent::as_fraction`] is the
//! wire form.
//!
//! This module also names the axis's scheduling shape:
//! [`BrightnessWaypoint`], [`BrightnessSchedule`], [`BrightnessAxis`].

use core::fmt;

use nota_codec::NotaEnum;

use crate::time::{RampDuration, RampTrigger};

/// A discrete brightness level on the daemon's standard ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NotaEnum)]
pub enum BrightnessLevel {
    Dim,
    Dimmer,
    Mid,
    Bright,
    Brighter,
    Brightest,
}

impl BrightnessLevel {
    /// The canonical percent value for this preset.
    pub const fn percent(self) -> BrightnessPercent {
        match self {
            BrightnessLevel::Dim => BrightnessPercent(35),
            BrightnessLevel::Dimmer => BrightnessPercent(50),
            BrightnessLevel::Mid => BrightnessPercent(70),
            BrightnessLevel::Bright => BrightnessPercent(85),
            BrightnessLevel::Brighter => BrightnessPercent(95),
            BrightnessLevel::Brightest => BrightnessPercent(100),
        }
    }

    /// The next-brighter preset. Saturates at [`BrightnessLevel::Brightest`].
    pub const fn brighter(self) -> Self {
        match self {
            BrightnessLevel::Dim => BrightnessLevel::Dimmer,
            BrightnessLevel::Dimmer => BrightnessLevel::Mid,
            BrightnessLevel::Mid => BrightnessLevel::Bright,
            BrightnessLevel::Bright => BrightnessLevel::Brighter,
            BrightnessLevel::Brighter => BrightnessLevel::Brightest,
            BrightnessLevel::Brightest => BrightnessLevel::Brightest,
        }
    }

    /// The next-dimmer preset. Saturates at [`BrightnessLevel::Dim`].
    pub const fn dimmer(self) -> Self {
        match self {
            BrightnessLevel::Dim => BrightnessLevel::Dim,
            BrightnessLevel::Dimmer => BrightnessLevel::Dim,
            BrightnessLevel::Mid => BrightnessLevel::Dimmer,
            BrightnessLevel::Bright => BrightnessLevel::Mid,
            BrightnessLevel::Brighter => BrightnessLevel::Bright,
            BrightnessLevel::Brightest => BrightnessLevel::Brighter,
        }
    }

    /// Lowercase name (for human-facing display and CLI replies).
    pub const fn as_str(self) -> &'static str {
        match self {
            BrightnessLevel::Dim => "dim",
            BrightnessLevel::Dimmer => "dimmer",
            BrightnessLevel::Mid => "mid",
            BrightnessLevel::Bright => "bright",
            BrightnessLevel::Brighter => "brighter",
            BrightnessLevel::Brightest => "brightest",
        }
    }
}

impl Default for BrightnessLevel {
    fn default() -> Self {
        BrightnessLevel::Bright
    }
}

impl fmt::Display for BrightnessLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A brightness value as a percent in `[0, 100]`.
///
/// wl-gammarelay-rs's `Brightness` DBus property is a `d`
/// (double) in `[0.0, 1.0]`; convert with [`Self::as_fraction`].
/// Construction clamps to `[MIN, MAX]` = `[0, 100]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BrightnessPercent(pub(crate) u8);

impl BrightnessPercent {
    /// The minimum value (fully dim).
    pub const MIN: Self = Self(0);
    /// The maximum value (fully bright).
    pub const MAX: Self = Self(100);

    /// Construct a percent value, clamping to `[MIN, MAX]`.
    pub const fn new(value: u8) -> Self {
        let clamped = if value > Self::MAX.0 { Self::MAX.0 } else { value };
        Self(clamped)
    }

    /// The integer percent value.
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// The fraction in `[0.0, 1.0]` for wl-gammarelay-rs's
    /// `Brightness` (d) DBus property.
    pub fn as_fraction(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Linear interpolation from `self` toward `target` at
    /// `fraction` ∈ `[0, 1]` (clamped at the boundaries).
    pub fn lerp_to(self, target: Self, fraction: f64) -> Self {
        let fraction = fraction.clamp(0.0, 1.0);
        let from = self.0 as f64;
        let to = target.0 as f64;
        let interpolated = from + (to - from) * fraction;
        Self::new(interpolated.round() as u8)
    }
}

impl fmt::Display for BrightnessPercent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}%", self.0)
    }
}

/// One scheduled brightness waypoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BrightnessWaypoint {
    pub trigger: RampTrigger,
    pub target: BrightnessLevel,
    pub ramp_duration: RampDuration,
}

/// The brightness axis's schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrightnessSchedule {
    Manual(BrightnessLevel),
    Scheduled { waypoints: Vec<BrightnessWaypoint>, default: BrightnessLevel },
}

impl BrightnessSchedule {
    pub fn needs_geolocation(&self) -> bool {
        match self {
            BrightnessSchedule::Manual(_) => false,
            BrightnessSchedule::Scheduled { waypoints, .. } => {
                waypoints.iter().any(|waypoint| waypoint.trigger.requires_geolocation())
            }
        }
    }

    pub fn default_level(&self) -> BrightnessLevel {
        match self {
            BrightnessSchedule::Manual(level) => *level,
            BrightnessSchedule::Scheduled { default, .. } => *default,
        }
    }
}

/// The full brightness-axis configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessAxis {
    pub schedule: BrightnessSchedule,
}
