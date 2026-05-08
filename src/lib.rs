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
pub mod theme;
pub mod warmth;

pub use brightness::{BrightnessLevel, BrightnessPercent};
pub use theme::ThemeMode;
pub use warmth::{KelvinTemperature, WarmthLevel};
