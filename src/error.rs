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
    NotaParse(#[from] nota::NotaDecodeError),

    /// Failed to encode or decode an rkyv archive on the wire.
    #[error("rkyv codec failed: {0}")]
    RkyvCodec(String),

    /// I/O error from the OS (UDS, file, process).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Native filesystem notification setup failed.
    #[error("notify: {0}")]
    Notify(#[source] Box<notify::Error>),

    /// Configuration is missing or malformed.
    #[error("config: {message}")]
    Config { message: String },

    /// DBus error from a wl-gammarelay-rs / geoclue method call.
    #[error("dbus: {0}")]
    Dbus(#[from] zbus::Error),

    /// DBus freedesktop interface error.
    #[error("dbus fdo: {0}")]
    DbusFdo(#[from] zbus::fdo::Error),

    /// GeoClue delivered its root-path sentinel instead of a location object.
    #[error("geoclue delivered a root location path")]
    GeoclueRootLocation,

    /// GeoClue ended the location-update subscription without a location.
    #[error("geoclue location-update stream ended")]
    GeoclueLocationUpdateStreamEnded,

    /// GeoClue returned a location whose accuracy value is not usable.
    #[error("geoclue returned an invalid location accuracy")]
    GeoclueInvalidAccuracy,

    /// GeoClue returned a location whose measurement timestamp is not usable.
    #[error("geoclue returned an invalid location timestamp")]
    GeoclueInvalidTimestamp,

    /// GeoClue returned a location older than Chroma's freshness boundary.
    #[error("geoclue location is stale ({age_seconds}s old)")]
    GeoclueLocationStale { age_seconds: u64 },

    /// GeoClue returned a technically fresh location that would expire before
    /// the next coarse solar-clock status interval.
    #[error("geoclue location expires too soon ({remaining_seconds}s remaining)")]
    GeoclueLocationExpiresSoon { remaining_seconds: u64 },

    /// The redb database could not be opened or created.
    #[error("redb database: {0}")]
    RedbDatabase(#[source] Box<redb::DatabaseError>),

    /// A redb table operation failed.
    #[error("redb table: {0}")]
    RedbTable(#[source] Box<redb::TableError>),

    /// A redb transaction could not be started.
    #[error("redb transaction: {0}")]
    RedbTransaction(#[source] Box<redb::TransactionError>),

    /// A redb write transaction could not be committed.
    #[error("redb commit: {0}")]
    RedbCommit(#[source] Box<redb::CommitError>),

    /// A redb storage operation failed.
    #[error("redb storage: {0}")]
    RedbStorage(#[source] Box<redb::StorageError>),

    /// A typed actor call failed.
    #[error("actor call: {message}")]
    ActorCall { message: String },

    /// The daemon refused a request — see [`Self::message`].
    #[error("daemon: {message}")]
    Daemon { message: String },

    /// A native theme concern failed while applying a mode.
    #[error("theme concern {concern} failed for {mode}: {message}")]
    ThemeConcern { concern: String, mode: String, message: String },
}

/// Crate-local result alias.
pub type Result<T> = core::result::Result<T, Error>;

impl From<redb::DatabaseError> for Error {
    fn from(error: redb::DatabaseError) -> Self {
        Self::RedbDatabase(Box::new(error))
    }
}

impl From<redb::TableError> for Error {
    fn from(error: redb::TableError) -> Self {
        Self::RedbTable(Box::new(error))
    }
}

impl From<redb::TransactionError> for Error {
    fn from(error: redb::TransactionError) -> Self {
        Self::RedbTransaction(Box::new(error))
    }
}

impl From<redb::CommitError> for Error {
    fn from(error: redb::CommitError) -> Self {
        Self::RedbCommit(Box::new(error))
    }
}

impl From<redb::StorageError> for Error {
    fn from(error: redb::StorageError) -> Self {
        Self::RedbStorage(Box::new(error))
    }
}

impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Self {
        Self::Notify(Box::new(error))
    }
}
