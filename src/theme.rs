//! `ThemeMode` — the desktop's colour-scheme axis.
//!
//! Two values: [`ThemeMode::Dark`] and [`ThemeMode::Light`].
//! Theme switches are always instant; the applier invokes the
//! configured shell script with the lowercase variant name as a
//! single positional argument.
//!
//! This module also names the axis's scheduling shape:
//! [`ThemeWaypoint`], [`ThemeSchedule`], [`ThemeAxis`].

use core::fmt;

use nota_codec::NotaEnum;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::config::ApplyCommand;
use crate::time::RampTrigger;

/// The active colour scheme.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, NotaEnum, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// The lowercase name passed as the apply-command argument.
    pub const fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }

    /// The opposite mode.
    pub const fn toggled(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One scheduled theme switch — at this trigger, become this mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeWaypoint {
    pub trigger: RampTrigger,
    pub mode: ThemeMode,
}

/// The theme axis's schedule.
///
/// Either a single [`Manual`](ThemeSchedule::Manual) value (no
/// scheduled fires; the daemon only switches when commanded), or
/// a [`Scheduled`](ThemeSchedule::Scheduled) list of waypoints
/// plus a default mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSchedule {
    Manual(ThemeMode),
    Scheduled { waypoints: Vec<ThemeWaypoint>, default: ThemeMode },
}

impl ThemeSchedule {
    /// Whether any waypoint in this schedule needs geolocation.
    pub fn needs_geolocation(&self) -> bool {
        match self {
            ThemeSchedule::Manual(_) => false,
            ThemeSchedule::Scheduled { waypoints, .. } => {
                waypoints.iter().any(|waypoint| waypoint.trigger.requires_geolocation())
            }
        }
    }

    /// The mode that holds when no waypoint applies (for Manual,
    /// the manual value itself).
    pub fn default_mode(&self) -> ThemeMode {
        match self {
            ThemeSchedule::Manual(mode) => *mode,
            ThemeSchedule::Scheduled { default, .. } => *default,
        }
    }
}

/// The full theme-axis configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeAxis {
    /// The shell script the daemon spawns to apply a theme.
    pub apply_command: ApplyCommand,
    /// The theme schedule.
    pub schedule: ThemeSchedule,
}
