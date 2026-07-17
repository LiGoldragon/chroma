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
pub mod geoclue;
pub mod request;
pub mod response;
pub mod schedule;
pub mod solar_time;
pub mod state;
pub mod theme;
pub mod time;
pub mod warmth;
pub mod wire;

pub use brightness::{BrightnessAxis, BrightnessLevel, BrightnessPercent, BrightnessSchedule, BrightnessWaypoint};
pub use config::{Config, ConfigFile};
pub use error::{Error, Result};
pub use geoclue::{
    FreshGeoclueLocation, FreshLocationLease, GeoclueLocationFix, GeoclueLocationSource, GeoclueLocationUpdate,
    GeoclueLocationUpdateAwaiter, MAX_LOCATION_AGE, MAX_SOLAR_LOCATION_ACCURACY_METERS, MINIMUM_SOLAR_CLOCK_VALIDITY,
    SolarLocationQuality,
};
pub use request::Request;
pub use response::Response;
pub use schedule::{Location, SchedulePlan, ScheduledBrightness, ScheduledValues, ScheduledWarmth};
pub use solar_time::SolarClockProjection;
pub use state::{
    ReadStoredLocation, ReadStoredState, RecordBrightness, RecordLocation, RecordTheme, RecordWarmth, StateStore,
    StoredLocation, StoredVisualState,
};
pub use theme::{
    ApplyTheme, GhosttyConfigTemplates, PiThemeControl, PiThemeControlRegistryDirectory, ThemeAdapters, ThemeApplier,
    ThemeAxis, ThemeConcern, ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule, ThemeWaypoint,
};
pub use time::{LocalHour, LocalMinute, RampDuration, RampTrigger, RelativeSolarOffset, SignedMinutes};
pub use warmth::{KelvinTemperature, WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint};
