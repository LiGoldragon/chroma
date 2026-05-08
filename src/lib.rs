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
pub mod config;
pub mod theme;
pub mod time;
pub mod warmth;

pub use brightness::{BrightnessAxis, BrightnessLevel, BrightnessPercent, BrightnessSchedule, BrightnessWaypoint};
pub use config::{ApplyCommand, Config};
pub use theme::{ThemeAxis, ThemeMode, ThemeSchedule, ThemeWaypoint};
pub use time::{LocalHour, LocalMinute, RampDuration, RampTrigger, SignedMinutes};
pub use warmth::{KelvinTemperature, WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint};
