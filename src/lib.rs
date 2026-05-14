//! chroma — one Rust daemon for theme, warmth, and brightness,
//! controlled via NOTA.
//!
//! Per the design report at
//! `~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`:
//! a single user-service daemon that owns three independent
//! visual axes — theme, warmth, brightness — each with its own
//! schedule, its own applier, and its own persisted state.
//!
//! See `ARCHITECTURE.md` for the system shape, `AGENTS.md` for
//! the agent contract, and `skills.md` for project-specific
//! intent and invariants.

pub mod brightness;
pub mod client;
pub mod config;
pub mod daemon;
pub mod error;
pub mod gamma;
pub mod request;
pub mod response;
pub mod schedule;
pub mod state;
pub mod theme;
pub mod time;
pub mod warmth;
pub mod wire;

pub use brightness::{BrightnessAxis, BrightnessLevel, BrightnessPercent, BrightnessSchedule, BrightnessWaypoint};
pub use config::{Config, ConfigFile};
pub use error::{Error, Result};
pub use request::Request;
pub use response::Response;
pub use schedule::{Location, SchedulePlan, ScheduledBrightness, ScheduledValues, ScheduledWarmth};
pub use state::{
    ReadStoredLocation, ReadStoredState, RecordBrightness, RecordLocation, RecordTheme, RecordWarmth, StateStore,
    StoredLocation, StoredVisualState,
};
pub use theme::{
    GhosttyConfigTemplates, ThemeAdapters, ThemeApplier, ThemeAxis, ThemeConcern, ThemeMode, ThemePalette,
    ThemePalettes, ThemeSchedule, ThemeWaypoint,
};
pub use time::{LocalHour, LocalMinute, RampDuration, RampTrigger, RelativeSolarOffset, SignedMinutes};
pub use warmth::{KelvinTemperature, WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint};
