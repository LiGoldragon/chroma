//! `ThemeMode` — the desktop's colour-scheme axis.
//!
//! Two values: [`ThemeMode::Dark`] and [`ThemeMode::Light`].
//! Theme switches are always instant; the applier invokes the
//! configured shell script with the lowercase variant name as a
//! single positional argument.

use core::fmt;

/// The active colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
