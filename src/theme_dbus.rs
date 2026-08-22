//! Chroma's resident session-bus projection for the Emacs theme consumer.
//!
//! The session bus is the trust boundary: a registration is bound to its
//! unique sender, rather than to an assertion made in method arguments.

use futures_util::StreamExt;
use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

use crate::daemon::{
    InstallThemeSignalPublisher, ProjectionOwnerDisappeared, QueryProjectionStatus, RegisterThemeConsumer,
    ReportThemeProjection,
};
use crate::error::{Error, Result};
use crate::theme::ThemeMode;

/// The public service name owned by Chroma.
pub const THEME_SERVICE: &str = "io.github.LiGoldragon.Chroma";
/// The sole object exposing the theme projection protocol.
pub const THEME_OBJECT_PATH: &str = "/io/github/LiGoldragon/Chroma/Theme";
/// The resident consumer protocol interface.
pub const THEME_INTERFACE: &str = "io.github.LiGoldragon.Chroma.Theme1";
/// The only supported consumer label.
pub const EMACS_CONSUMER: &str = "emacs";
/// The plugin's bounded diagnostic vocabulary.
pub const FAILURE_CODES: [&str; 4] = ["configuration", "load-failed", "verification-failed", "application-failed"];
/// The plugin caps summaries by UTF-8 byte length, so Chroma enforces the same bound.
pub const FAILURE_SUMMARY_MAX_BYTES: usize = 240;

/// The desired mode and its monotonic change revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeSnapshot {
    mode: ThemeMode,
    revision: u64,
}

impl ThemeSnapshot {
    pub const fn new(mode: ThemeMode, revision: u64) -> Self {
        Self { mode, revision }
    }

    pub const fn mode(self) -> ThemeMode {
        self.mode
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// Chroma's observable state for the resident projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionStatus {
    Unavailable,
    Pending,
    Applied { revision: u64 },
    Failed { revision: u64 },
}

/// Query reply containing status and the desired revision it describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionStatusRecord {
    snapshot: ThemeSnapshot,
    status: ProjectionStatus,
}

impl ProjectionStatusRecord {
    pub const fn new(snapshot: ThemeSnapshot, status: ProjectionStatus) -> Self {
        Self { snapshot, status }
    }

    pub const fn snapshot(self) -> ThemeSnapshot {
        self.snapshot
    }

    pub const fn status(self) -> ProjectionStatus {
        self.status
    }
}

impl ProjectionStatus {
    pub const fn dbus_name(self) -> &'static str {
        match self {
            Self::Unavailable => "Unavailable",
            Self::Pending => "Pending",
            Self::Applied { .. } => "Applied",
            Self::Failed { .. } => "Failed",
        }
    }
}

/// A consumer's current acknowledgement, decoded from the public D-Bus call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionReport {
    Applied { revision: u64 },
    Failed { revision: u64, code: String, summary: String },
}

impl ProjectionReport {
    pub const fn applied(revision: u64) -> Self {
        Self::Applied { revision }
    }

    pub fn failed(revision: u64, code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::Failed { revision, code: code.into(), summary: summary.into() }
    }

    pub const fn revision(&self) -> u64 {
        match self {
            Self::Applied { revision } | Self::Failed { revision, .. } => *revision,
        }
    }

    fn validate(&self) -> std::result::Result<(), ProjectionError> {
        match self {
            Self::Applied { .. } => Ok(()),
            Self::Failed { code, summary, .. } => {
                if !FAILURE_CODES.contains(&code.as_str()) {
                    return Err(ProjectionError::InvalidFailureCode);
                }
                if summary.len() > FAILURE_SUMMARY_MAX_BYTES {
                    return Err(ProjectionError::FailureSummaryTooLong);
                }
                Ok(())
            }
        }
    }
}

/// Rejection reasons exposed as bounded D-Bus failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    UnsupportedConsumer,
    LiveOwnerExists,
    SenderDoesNotOwnConsumer,
    FutureRevision,
    InvalidFailureCode,
    FailureSummaryTooLong,
}

impl ProjectionError {
    pub const fn summary(self) -> &'static str {
        match self {
            Self::UnsupportedConsumer => "unsupported consumer",
            Self::LiveOwnerExists => "consumer already has a live owner",
            Self::SenderDoesNotOwnConsumer => "caller does not own consumer",
            Self::FutureRevision => "acknowledgement revision is newer than desired state",
            Self::InvalidFailureCode => "unsupported failure code",
            Self::FailureSummaryTooLong => "failure summary exceeds 240 bytes",
        }
    }
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.summary())
    }
}

/// In-memory liveness and acknowledgement state. The desired snapshot itself
/// is persisted by `StateStore`; sender identity deliberately is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeProjection {
    snapshot: ThemeSnapshot,
    owner: Option<String>,
    status: ProjectionStatus,
}

impl ThemeProjection {
    pub const fn new(snapshot: ThemeSnapshot) -> Self {
        Self { snapshot, owner: None, status: ProjectionStatus::Unavailable }
    }

    pub const fn snapshot(&self) -> ThemeSnapshot {
        self.snapshot
    }

    pub const fn status(&self) -> ProjectionStatus {
        self.status
    }

    pub fn register(&mut self, consumer: &str, sender: &str) -> std::result::Result<ThemeSnapshot, ProjectionError> {
        if consumer != EMACS_CONSUMER {
            return Err(ProjectionError::UnsupportedConsumer);
        }
        if self.owner.as_deref().is_some_and(|owner| owner != sender) {
            return Err(ProjectionError::LiveOwnerExists);
        }
        self.owner = Some(sender.to_owned());
        self.status = ProjectionStatus::Pending;
        Ok(self.snapshot)
    }

    /// Replace desired state after durable persistence. Equal state/revision is
    /// intentionally a no-op, so duplicate desired deliveries are harmless.
    pub fn replace_desired(&mut self, snapshot: ThemeSnapshot) -> Option<ThemeSnapshot> {
        if snapshot == self.snapshot {
            return None;
        }
        self.snapshot = snapshot;
        self.status = if self.owner.is_some() { ProjectionStatus::Pending } else { ProjectionStatus::Unavailable };
        Some(snapshot)
    }

    pub fn report(&mut self, sender: &str, report: ProjectionReport) -> std::result::Result<(), ProjectionError> {
        if self.owner.as_deref() != Some(sender) {
            return Err(ProjectionError::SenderDoesNotOwnConsumer);
        }
        if report.revision() < self.snapshot.revision() {
            return Ok(());
        }
        if report.revision() > self.snapshot.revision() {
            return Err(ProjectionError::FutureRevision);
        }
        report.validate()?;
        self.status = match report {
            ProjectionReport::Applied { revision } => ProjectionStatus::Applied { revision },
            ProjectionReport::Failed { revision, .. } => ProjectionStatus::Failed { revision },
        };
        Ok(())
    }

    pub fn owner_disappeared(&mut self, sender: &str) {
        if self.owner.as_deref() == Some(sender) {
            self.owner = None;
            self.status = ProjectionStatus::Unavailable;
        }
    }

    pub const fn status_record(&self) -> ProjectionStatusRecord {
        ProjectionStatusRecord::new(self.snapshot, self.status)
    }
}

/// Signal sender retained by the root after it has exported the interface.
#[derive(Clone)]
pub(crate) struct ThemeSignalPublisher {
    emitter: SignalEmitter<'static>,
}

impl ThemeSignalPublisher {
    pub(crate) fn new(connection: &zbus::Connection) -> Result<Self> {
        Ok(Self { emitter: SignalEmitter::new(connection, THEME_OBJECT_PATH)?.to_owned() })
    }

    pub(crate) async fn publish(&self, snapshot: ThemeSnapshot) -> Result<()> {
        ThemeDbusInterface::desired_state_changed(
            self.emitter.clone(),
            snapshot.mode().dbus_name(),
            snapshot.revision(),
        )
        .await?;
        Ok(())
    }
}

/// The exported server endpoint. State stays in the Kameo root, which is the
/// sole owner of desired state and consumer liveness.
pub(crate) struct ThemeDbusInterface {
    root: ActorRef<crate::daemon::ChromaRoot>,
}

impl ThemeDbusInterface {
    pub(crate) fn new(root: ActorRef<crate::daemon::ChromaRoot>) -> Self {
        Self { root }
    }

    fn sender(header: &Header<'_>) -> zbus::fdo::Result<String> {
        header
            .sender()
            .map(ToString::to_string)
            .ok_or_else(|| zbus::fdo::Error::AccessDenied("caller has no unique session-bus name".into()))
    }

    fn fdo(error: impl ToString) -> zbus::fdo::Error {
        zbus::fdo::Error::Failed(error.to_string())
    }
}

#[zbus::interface(name = "io.github.LiGoldragon.Chroma.Theme1")]
impl ThemeDbusInterface {
    /// Foreign-ABI exception: D-Bus exposes two positional return values.
    #[zbus(out_args("state", "revision"))]
    async fn register_consumer(
        &self,
        consumer: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<(String, u64)> {
        let sender = Self::sender(&header)?;
        let snapshot =
            self.root.ask(RegisterThemeConsumer { consumer: consumer.to_owned(), sender }).await.map_err(Self::fdo)?;
        Ok((snapshot.mode().dbus_name().to_owned(), snapshot.revision()))
    }

    /// `code` and `summary` are always present; `Applied` carries two empty
    /// strings. D-Bus has fixed method signatures and cannot represent the
    /// formerly variable-arity client call.
    async fn report_projection(
        &self,
        consumer: &str,
        revision: u64,
        result: &str,
        code: &str,
        summary: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = Self::sender(&header)?;
        self.root
            .ask(ReportThemeProjection {
                consumer: consumer.to_owned(),
                sender,
                revision,
                result: result.to_owned(),
                code: code.to_owned(),
                summary: summary.to_owned(),
            })
            .await
            .map_err(Self::fdo)?;
        Ok(())
    }

    /// Foreign-ABI exception: status and the observed desired revision are
    /// separate D-Bus return values for easy inspection from Emacs.
    #[zbus(out_args("status", "revision"))]
    async fn get_projection_status(&self, consumer: &str) -> zbus::fdo::Result<(String, u64)> {
        let status = self.root.ask(QueryProjectionStatus { consumer: consumer.to_owned() }).await.map_err(Self::fdo)?;
        Ok((status.status().dbus_name().to_owned(), status.snapshot().revision()))
    }

    #[zbus(signal)]
    async fn desired_state_changed(emitter: SignalEmitter<'_>, state: &str, revision: u64) -> zbus::Result<()>;
}

/// Keeps the session connection and D-Bus owner watcher alive for the daemon.
pub(crate) struct ThemeDbusService {
    _connection: zbus::Connection,
    owner_watcher: ActorRef<ThemeOwnerWatcher>,
}

impl ThemeDbusService {
    pub(crate) async fn start(root: ActorRef<crate::daemon::ChromaRoot>) -> Result<Self> {
        let connection = zbus::Connection::session().await?;
        connection.request_name(THEME_SERVICE).await?;
        connection.object_server().at(THEME_OBJECT_PATH, ThemeDbusInterface::new(root.clone())).await?;
        root.ask(InstallThemeSignalPublisher { publisher: ThemeSignalPublisher::new(&connection)? })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;
        let owner_watcher = ThemeOwnerWatcher::start(connection.clone(), root).await?;
        Ok(Self { _connection: connection, owner_watcher })
    }

    pub(crate) async fn stop(self) {
        let _ = self.owner_watcher.stop_gracefully().await;
        self.owner_watcher.wait_for_shutdown().await;
    }
}

struct ThemeOwnerWatcher {
    root: ActorRef<crate::daemon::ChromaRoot>,
    signals: Option<zbus::proxy::SignalStream<'static>>,
}

impl ThemeOwnerWatcher {
    async fn start(connection: zbus::Connection, root: ActorRef<crate::daemon::ChromaRoot>) -> Result<ActorRef<Self>> {
        let bus =
            zbus::Proxy::new(&connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")
                .await?;
        let signals = bus.receive_signal("NameOwnerChanged").await?;
        let reference = Self::spawn(Self { root, signals: Some(signals) });
        reference.wait_for_startup().await;
        reference.tell(BeginWatchingOwners).await.map_err(|error| Error::ActorCall { message: error.to_string() })?;
        Ok(reference)
    }
}

impl Actor for ThemeOwnerWatcher {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(watcher: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(watcher)
    }
}

struct BeginWatchingOwners;

impl Message<BeginWatchingOwners> for ThemeOwnerWatcher {
    type Reply = ();

    async fn handle(&mut self, _message: BeginWatchingOwners, context: &mut Context<Self, Self::Reply>) {
        let root = self.root.clone();
        let Some(mut signals) = self.signals.take() else {
            return;
        };
        let _ = context.spawn(async move {
            while let Some(message) = signals.next().await {
                // Foreign D-Bus wire body: NameOwnerChanged(name, old-owner, new-owner).
                let Ok((_name, old_owner, new_owner)) = message.body().deserialize::<(String, String, String)>() else {
                    continue;
                };
                if !old_owner.is_empty() && new_owner.is_empty() {
                    let _ = root.tell(ProjectionOwnerDisappeared { sender: old_owner }).await;
                }
            }
        });
    }
}
