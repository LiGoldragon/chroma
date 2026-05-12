//! Durable visual state, stored in redb as rkyv archives.
//!
//! Chroma owns one state database at
//! `$XDG_STATE_HOME/chroma/state.redb`. Runtime actors never open
//! that file directly; they send typed messages to [`StateStore`].

use std::fs;
use std::path::{Path, PathBuf};

use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};
use redb::{Database, TableDefinition};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::brightness::BrightnessPercent;
use crate::error::{Error, Result};
use crate::theme::ThemeMode;
use crate::warmth::KelvinTemperature;

const THEME_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("theme");
const WARMTH_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("warmth");
const BRIGHTNESS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("brightness");
const LOCATION_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("location");
const CURRENT_KEY: &str = "current";

/// The daemon's current visual state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredVisualState {
    pub theme: ThemeMode,
    pub kelvin: KelvinTemperature,
    pub percent: BrightnessPercent,
}

/// Last geoclue position known to the schedule engine.
#[derive(Debug, Clone, Copy, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct StoredLocation {
    pub latitude: f64,
    pub longitude: f64,
}

impl StoredLocation {
    /// Archive into bytes for redb persistence.
    pub fn archive(&self) -> Result<Vec<u8>> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(|err| Error::RkyvCodec(err.to_string()))
    }

    /// Decode a redb-stored rkyv archive.
    pub fn from_archive(bytes: &[u8]) -> Result<Self> {
        rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes).map_err(|err| Error::RkyvCodec(err.to_string()))
    }
}

/// redb owner for Chroma's persisted state.
pub struct StateStore {
    database: Database,
    path: PathBuf,
}

impl StateStore {
    /// Open the default user-state database.
    pub fn open_default() -> Result<Self> {
        Self::open(state_database_path()?)
    }

    /// Open or create a state database at `path`.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let database = if path.exists() { Database::open(&path)? } else { Database::create(&path)? };
        let store = Self { database, path };
        store.ensure_tables()?;
        Ok(store)
    }

    /// Spawn on a dedicated actor thread because redb's API is
    /// synchronous by design.
    pub fn start_default() -> Result<ActorRef<Self>> {
        let store = Self::open_default()?;
        let reference = Self::spawn_in_thread(store);
        Ok(reference)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_tables(&self) -> Result<()> {
        let transaction = self.database.begin_write()?;
        {
            transaction.open_table(THEME_TABLE)?;
            transaction.open_table(WARMTH_TABLE)?;
            transaction.open_table(BRIGHTNESS_TABLE)?;
            transaction.open_table(LOCATION_TABLE)?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn record_theme(&self, mode: ThemeMode) -> Result<()> {
        self.write_archive(THEME_TABLE, &mode.archive()?)
    }

    fn record_warmth(&self, kelvin: KelvinTemperature) -> Result<()> {
        self.write_archive(WARMTH_TABLE, &kelvin.archive()?)
    }

    fn record_brightness(&self, percent: BrightnessPercent) -> Result<()> {
        self.write_archive(BRIGHTNESS_TABLE, &percent.archive()?)
    }

    fn record_location(&self, location: StoredLocation) -> Result<()> {
        self.write_archive(LOCATION_TABLE, &location.archive()?)
    }

    fn write_archive(&self, definition: TableDefinition<&str, &[u8]>, bytes: &[u8]) -> Result<()> {
        let transaction = self.database.begin_write()?;
        {
            let mut table = transaction.open_table(definition)?;
            table.insert(CURRENT_KEY, bytes)?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn read_state(&self, fallback: StoredVisualState) -> Result<StoredVisualState> {
        Ok(StoredVisualState {
            theme: self.read_theme()?.unwrap_or(fallback.theme),
            kelvin: self.read_warmth()?.unwrap_or(fallback.kelvin),
            percent: self.read_brightness()?.unwrap_or(fallback.percent),
        })
    }

    fn read_theme(&self) -> Result<Option<ThemeMode>> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(THEME_TABLE)?;
        let Some(bytes) = table.get(CURRENT_KEY)? else {
            return Ok(None);
        };
        Ok(Some(ThemeMode::from_archive(bytes.value())?))
    }

    fn read_warmth(&self) -> Result<Option<KelvinTemperature>> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(WARMTH_TABLE)?;
        let Some(bytes) = table.get(CURRENT_KEY)? else {
            return Ok(None);
        };
        Ok(Some(KelvinTemperature::from_archive(bytes.value())?))
    }

    fn read_brightness(&self) -> Result<Option<BrightnessPercent>> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(BRIGHTNESS_TABLE)?;
        let Some(bytes) = table.get(CURRENT_KEY)? else {
            return Ok(None);
        };
        Ok(Some(BrightnessPercent::from_archive(bytes.value())?))
    }

    fn read_location(&self) -> Result<Option<StoredLocation>> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(LOCATION_TABLE)?;
        let Some(bytes) = table.get(CURRENT_KEY)? else {
            return Ok(None);
        };
        Ok(Some(StoredLocation::from_archive(bytes.value())?))
    }
}

impl Actor for StateStore {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(store: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(store)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordTheme {
    pub mode: ThemeMode,
}

impl Message<RecordTheme> for StateStore {
    type Reply = Result<()>;

    async fn handle(&mut self, message: RecordTheme, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.record_theme(message.mode)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordWarmth {
    pub kelvin: KelvinTemperature,
}

impl Message<RecordWarmth> for StateStore {
    type Reply = Result<()>;

    async fn handle(&mut self, message: RecordWarmth, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.record_warmth(message.kelvin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordBrightness {
    pub percent: BrightnessPercent,
}

impl Message<RecordBrightness> for StateStore {
    type Reply = Result<()>;

    async fn handle(&mut self, message: RecordBrightness, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.record_brightness(message.percent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordLocation {
    pub location: StoredLocation,
}

impl Message<RecordLocation> for StateStore {
    type Reply = Result<()>;

    async fn handle(&mut self, message: RecordLocation, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.record_location(message.location)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadStoredState {
    pub fallback: StoredVisualState,
}

impl Message<ReadStoredState> for StateStore {
    type Reply = Result<StoredVisualState>;

    async fn handle(&mut self, message: ReadStoredState, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.read_state(message.fallback)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadStoredLocation;

impl Message<ReadStoredLocation> for StateStore {
    type Reply = Result<Option<StoredLocation>>;

    async fn handle(&mut self, _message: ReadStoredLocation, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.read_location()
    }
}

fn state_database_path() -> Result<PathBuf> {
    Ok(state_home()?.join("chroma/state.redb"))
}

fn state_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from) {
        return Ok(path);
    }
    Ok(home_directory()?.join(".local/state"))
}

fn home_directory() -> Result<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| Error::Config { message: "HOME is not set".into() })
}
