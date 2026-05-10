//! [`Error`] — the crate's typed error enum.
//!
//! Every fallible boundary in the crate returns
//! `Result<T, Error>`. No `anyhow::Error` / `eyre::Report` /
//! `Box<dyn Error>` at any boundary; per
//! `~/primary/skills/rust-discipline.md` §"Errors: typed enum
//! per crate via thiserror".

use thiserror::Error as ThisError;

/// The crate's error type.
#[derive(Debug, ThisError)]
pub enum Error {
    /// Failed to parse a NOTA document at the CLI or config boundary.
    #[error("nota parse failed: {0}")]
    NotaParse(#[from] nota_codec::Error),

    /// Failed to encode or decode an rkyv archive on the wire.
    #[error("rkyv codec failed: {0}")]
    RkyvCodec(String),

    /// I/O error from the OS (UDS, file, process).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration is missing or malformed.
    #[error("config: {message}")]
    Config { message: String },

    /// DBus error from a wl-gammarelay-rs / geoclue method call.
    #[error("dbus: {0}")]
    Dbus(#[from] zbus::Error),

    /// The daemon refused a request — see [`Self::message`].
    #[error("daemon: {message}")]
    Daemon { message: String },

    /// A native theme concern failed while applying a mode.
    #[error("theme concern {concern} failed for {mode}: {message}")]
    ThemeConcern { concern: String, mode: String, message: String },
}

/// Crate-local result alias.
pub type Result<T> = core::result::Result<T, Error>;
