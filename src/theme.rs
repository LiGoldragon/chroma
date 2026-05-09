//! `ThemeMode` — the desktop's colour-scheme axis.
//!
//! Two values: [`ThemeMode::Dark`] and [`ThemeMode::Light`].
//! Theme switches are accepted instantly; the daemon spawns the
//! configured shell script with the lowercase variant name as a
//! single positional argument and waits for completion outside
//! the request path.
//!
//! This module also names the axis's scheduling shape:
//! [`ThemeWaypoint`], [`ThemeSchedule`], [`ThemeAxis`].

use core::fmt;
use std::process::Stdio;

use nota_codec::NotaEnum;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use tokio::process::Child;

use crate::config::ApplyCommand;
use crate::error::{Error, Result};
use crate::time::RampTrigger;

/// The active colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NotaEnum, Archive, RkyvSerialize, RkyvDeserialize)]
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

/// Applies theme changes through the configured external script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeApplier {
    apply_command: ApplyCommand,
}

impl ThemeApplier {
    pub fn from_apply_command(apply_command: ApplyCommand) -> Self {
        Self { apply_command }
    }

    pub fn spawn(&self, mode: ThemeMode) -> Result<ThemeApplyProcess> {
        let command = self.apply_command.to_string();
        let child = tokio::process::Command::new(self.apply_command.as_path())
            .arg(mode.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| Error::ThemeApply {
                command: command.clone(),
                mode: mode.to_string(),
                message: source.to_string(),
            })?;

        Ok(ThemeApplyProcess { child, command, mode })
    }

    pub async fn apply(&self, mode: ThemeMode) -> Result<()> {
        self.spawn(mode)?.wait().await
    }
}

/// A spawned theme application process.
///
/// Dropping this value kills the process, so aborted daemon tasks
/// cannot leave older theme applications racing newer requests.
pub struct ThemeApplyProcess {
    child: Child,
    command: String,
    mode: ThemeMode,
}

impl ThemeApplyProcess {
    pub async fn wait(self) -> Result<()> {
        let output = self.child.wait_with_output().await.map_err(|source| Error::ThemeApply {
            command: self.command.clone(),
            mode: self.mode.to_string(),
            message: source.to_string(),
        })?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };

        Err(Error::ThemeApply { command: self.command, mode: self.mode.to_string(), message })
    }
}
