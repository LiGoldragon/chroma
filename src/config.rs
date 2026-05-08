//! [`Config`] — top-level chroma configuration.
//!
//! Parsed from a single NOTA record at
//! `~/.config/chroma/config.nota`. Re-parsed on inotify push.

use core::fmt;
use std::path::{Path, PathBuf};

use crate::brightness::BrightnessAxis;
use crate::theme::ThemeAxis;
use crate::warmth::WarmthAxis;

/// Path to the home-manager-built shell script that applies a
/// theme. The daemon spawns it with one positional argument
/// (`dark` or `light`) and waits for exit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApplyCommand(PathBuf);

impl ApplyCommand {
    /// Construct from any path-like value.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Borrow as a [`Path`] for spawning.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for ApplyCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}

/// Top-level chroma configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeAxis,
    pub warmth: WarmthAxis,
    pub brightness: BrightnessAxis,
}

impl Config {
    /// Whether any axis schedule requires the geoclue subscription.
    pub fn needs_geolocation(&self) -> bool {
        self.theme.schedule.needs_geolocation()
            || self.warmth.schedule.needs_geolocation()
            || self.brightness.schedule.needs_geolocation()
    }
}
