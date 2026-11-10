//! GeoClue location events.
//!
//! Chroma holds one GeoClue client for its whole life. GeoClue initializes
//! `Client.Location` to the root object path; the client subscribes to
//! `LocationUpdated` before `Start`, then treats each signal's new object path
//! as the authority for one location-property read. A held location stays
//! held until GeoClue announces another: a location here never ages, expires,
//! renews, or retries on a timer. When the GeoClue service changes owner on
//! the bus, that `NameOwnerChanged` event reconnects the client.

use std::pin::Pin;

use futures_util::{Stream, StreamExt};
use kameo::actor::{Actor, ActorRef, Recipient, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};
use kameo::reply::DelegatedReply;
use zbus::zvariant::OwnedObjectPath;

use crate::error::{Error, Result};
use crate::schedule::Location;

const GEOCLUE_SERVICE: &str = "org.freedesktop.GeoClue2";
const GEOCLUE_MANAGER_PATH: &str = "/org/freedesktop/GeoClue2/Manager";
const GEOCLUE_MANAGER_INTERFACE: &str = "org.freedesktop.GeoClue2.Manager";
const GEOCLUE_CLIENT_INTERFACE: &str = "org.freedesktop.GeoClue2.Client";
const GEOCLUE_LOCATION_INTERFACE: &str = "org.freedesktop.GeoClue2.Location";
// GeoClue's public GClueAccuracyLevel enum assigns 8 to Exact. Requesting
// Country (1) makes the static source add a privacy margin, turning an exact
// 1 km configured fix into a 4 km result that Chroma must reject.
const GEOCLUE_ACCURACY_LEVEL_EXACT: u32 = 8;

/// Maximum uncertainty for a GeoClue fix that may drive solar schedules.
///
/// A wider fix can move civil events enough to make Chroma's visual state
/// wrong for the place where it is running. It is therefore not a degraded
/// replacement for a held precise fix.
pub const MAX_LOCATION_ACCURACY_METERS: f64 = 1_000.0;

/// The object paths carried by GeoClue's `LocationUpdated` signal.
#[derive(Debug)]
pub struct GeoclueLocationUpdate {
    new_location_path: OwnedObjectPath,
}

impl GeoclueLocationUpdate {
    /// Decode the `old_location, new_location` signal body.
    pub fn from_signal_body((_old_location_path, new_location_path): (OwnedObjectPath, OwnedObjectPath)) -> Self {
        Self { new_location_path }
    }

    /// Decode one `LocationUpdated` signal message.
    pub fn from_message(message: zbus::Message) -> Result<Self> {
        message
            .body()
            .deserialize::<(OwnedObjectPath, OwnedObjectPath)>()
            .map(Self::from_signal_body)
            .map_err(Error::from)
    }

    /// Return the delivered location object path, rejecting GeoClue's root sentinel.
    pub fn location_path(self) -> Result<OwnedObjectPath> {
        if self.new_location_path.as_str() == "/" {
            return Err(Error::GeoclueRootLocation);
        }
        Ok(self.new_location_path)
    }
}

/// An awaited GeoClue location-update subscription.
pub struct GeoclueLocationUpdateAwaiter<Updates> {
    updates: Pin<Box<Updates>>,
}

impl<Updates> GeoclueLocationUpdateAwaiter<Updates> {
    /// Take ownership of a subscription installed before `Client.Start`.
    pub fn new(updates: Updates) -> Self {
        Self { updates: Box::pin(updates) }
    }
}

impl<Updates> GeoclueLocationUpdateAwaiter<Updates>
where
    Updates: Stream<Item = Result<GeoclueLocationUpdate>>,
{
    /// Await the next location delivery without reading `Client.Location`.
    pub async fn location_path(&mut self) -> Result<OwnedObjectPath> {
        let update = self.updates.as_mut().next().await.ok_or(Error::GeoclueLocationUpdateStreamEnded)??;
        update.location_path()
    }
}

/// A GeoClue location property's measurement.
#[derive(Debug, Clone, Copy)]
pub struct GeoclueLocationFix {
    location: Location,
    accuracy_meters: f64,
}

impl GeoclueLocationFix {
    /// Construct a fix from the GeoClue location object's properties.
    pub fn new(location: Location, accuracy_meters: f64) -> Self {
        Self { location, accuracy_meters }
    }

    /// Validate the fix itself before it may affect solar scheduling.
    pub fn validated(self) -> Result<LocationAnnounced> {
        if !self.accuracy_meters.is_finite() || self.accuracy_meters < 0.0 {
            return Err(Error::GeoclueInvalidAccuracy);
        }
        if self.accuracy_meters > MAX_LOCATION_ACCURACY_METERS {
            return Err(Error::GeoclueLocationImprecise { accuracy_meters: self.accuracy_meters });
        }
        Ok(LocationAnnounced { location: self.location })
    }
}

/// The event of GeoClue announcing a validated location.
#[derive(Debug, Clone, Copy)]
pub struct LocationAnnounced {
    location: Location,
}

impl LocationAnnounced {
    /// The announced location.
    pub fn location(self) -> Location {
        self.location
    }
}

/// The location GeoClue has announced to this daemon.
///
/// It starts [`Unlocated`](Self::Unlocated) and, once located, holds its
/// location until the next announcement replaces it.
#[derive(Debug, Clone, Copy, Default)]
pub enum LocationAuthority {
    #[default]
    Unlocated,
    Located(Location),
}

impl LocationAuthority {
    /// Hold the announced location in place of any earlier one.
    pub fn announce(&mut self, announcement: LocationAnnounced) {
        *self = Self::Located(announcement.location);
    }

    /// The held location, if GeoClue has ever announced one.
    pub fn location(self) -> Option<Location> {
        match self {
            Self::Located(location) => Some(location),
            Self::Unlocated => None,
        }
    }
}

/// The actor that holds Chroma's GeoClue client for the daemon's life and
/// tells its recipient each validated announcement.
pub struct GeoclueWatcher {
    connection: zbus::Connection,
    recipient: Recipient<LocationAnnounced>,
}

impl GeoclueWatcher {
    /// Start watching the GeoClue service reachable on `connection`.
    pub async fn start(connection: zbus::Connection, recipient: Recipient<LocationAnnounced>) -> ActorRef<Self> {
        let reference = Self::spawn(Self { connection, recipient });
        reference.wait_for_startup().await;
        if let Err(error) = reference.tell(WatchGeoclue).await {
            eprintln!("chroma-daemon geoclue watcher start error: {error}");
        }
        reference
    }
}

impl Actor for GeoclueWatcher {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(watcher: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(watcher)
    }
}

struct WatchGeoclue;

impl Message<WatchGeoclue> for GeoclueWatcher {
    type Reply = DelegatedReply<()>;

    async fn handle(&mut self, _message: WatchGeoclue, context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let connection = self.connection.clone();
        let recipient = self.recipient.clone();
        context.spawn(async move {
            match GeoclueSubscription::open(connection, recipient).await {
                Ok(subscription) => subscription.run().await,
                Err(error) => eprintln!("chroma-daemon geoclue watcher subscribe error: {error}"),
            }
        })
    }
}

type LocationUpdates =
    futures_util::stream::Map<zbus::proxy::SignalStream<'static>, fn(zbus::Message) -> Result<GeoclueLocationUpdate>>;

/// The bus-side state of the watcher: the GeoClue owner subscription and the
/// client living on the current owner, if one could be created.
struct GeoclueSubscription {
    connection: zbus::Connection,
    recipient: Recipient<LocationAnnounced>,
    bus: zbus::Proxy<'static>,
    owner_changes: zbus::proxy::SignalStream<'static>,
    client: Option<GeoclueClient>,
}

impl GeoclueSubscription {
    async fn open(connection: zbus::Connection, recipient: Recipient<LocationAnnounced>) -> Result<Self> {
        let bus =
            zbus::Proxy::new(&connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")
                .await?;
        let owner_changes = bus.receive_signal_with_args("NameOwnerChanged", &[(0, GEOCLUE_SERVICE)]).await?;
        let mut subscription = Self { connection, recipient, bus, owner_changes, client: None };
        subscription.connect_client().await;
        Ok(subscription)
    }

    /// Answer GeoClue events until the bus connection itself ends.
    async fn run(mut self) {
        loop {
            tokio::select! {
                update = Self::next_location_path(self.client.as_mut()) => self.answer_location_update(update).await,
                change = self.owner_changes.next() => match change {
                    Some(message) => self.answer_owner_change(message).await,
                    None => break,
                },
            }
        }
        eprintln!("chroma-daemon geoclue bus connection ended; no GeoClue event remains to answer");
    }

    async fn next_location_path(client: Option<&mut GeoclueClient>) -> Result<OwnedObjectPath> {
        match client {
            Some(client) => client.updates.location_path().await,
            None => std::future::pending().await,
        }
    }

    async fn connect_client(&mut self) {
        match GeoclueClient::connect(&self.connection, &self.bus).await {
            Ok(client) => self.client = Some(client),
            Err(error) => {
                eprintln!("chroma-daemon geoclue client error: {error}; waiting for GeoClue to change bus owner")
            }
        }
    }

    async fn answer_location_update(&mut self, update: Result<OwnedObjectPath>) {
        match update {
            Ok(location_path) => match self.read_fix(location_path).await {
                Ok(fix) => self.answer_fix(fix).await,
                Err(error) => eprintln!("chroma-daemon geoclue location read error: {error}"),
            },
            Err(Error::GeoclueLocationUpdateStreamEnded) => {
                eprintln!("chroma-daemon geoclue location-update stream ended; reconnecting");
                self.client = None;
                self.connect_client().await;
            }
            Err(error) => eprintln!("chroma-daemon geoclue location update error: {error}"),
        }
    }

    async fn answer_fix(&self, fix: GeoclueLocationFix) {
        match fix.validated() {
            Ok(announcement) => {
                eprintln!("chroma-daemon accepted GeoClue location (accuracy: {:.0}m)", fix.accuracy_meters);
                if let Err(error) = self.recipient.tell(announcement).send().await {
                    eprintln!("chroma-daemon geoclue announcement enqueue error: {error}");
                }
            }
            Err(error) => eprintln!(
                "chroma-daemon rejected GeoClue location: {error}; holding the current location until the next LocationUpdated"
            ),
        }
    }

    async fn answer_owner_change(&mut self, message: zbus::Message) {
        // Foreign D-Bus wire body: NameOwnerChanged(name, old-owner, new-owner).
        let Ok((_name, _old_owner, new_owner)) = message.body().deserialize::<(String, String, String)>() else {
            eprintln!("chroma-daemon geoclue owner change was not decodable");
            return;
        };
        if self.client.as_ref().is_some_and(|client| client.owner == new_owner) {
            return;
        }
        // A GeoClue client lives on the owner that created it and leaves with it.
        self.client = None;
        if new_owner.is_empty() {
            eprintln!("chroma-daemon geoclue service left the bus; reconnecting");
        } else {
            eprintln!("chroma-daemon geoclue service took the bus name; connecting");
        }
        self.connect_client().await;
    }

    async fn read_fix(&self, location_path: OwnedObjectPath) -> Result<GeoclueLocationFix> {
        let location =
            zbus::Proxy::new(&self.connection, GEOCLUE_SERVICE, location_path, GEOCLUE_LOCATION_INTERFACE).await?;
        let latitude: f64 = location.get_property("Latitude").await?;
        let longitude: f64 = location.get_property("Longitude").await?;
        let accuracy_meters: f64 = location.get_property("Accuracy").await?;
        Ok(GeoclueLocationFix::new(Location { latitude, longitude }, accuracy_meters))
    }
}

/// One started GeoClue client and its `LocationUpdated` subscription.
struct GeoclueClient {
    owner: String,
    updates: GeoclueLocationUpdateAwaiter<LocationUpdates>,
}

impl GeoclueClient {
    async fn connect(connection: &zbus::Connection, bus: &zbus::Proxy<'static>) -> Result<Self> {
        let manager =
            zbus::Proxy::new(connection, GEOCLUE_SERVICE, GEOCLUE_MANAGER_PATH, GEOCLUE_MANAGER_INTERFACE).await?;
        let client_path: OwnedObjectPath = manager.call("CreateClient", &()).await?;
        let owner: String = bus.call("GetNameOwner", &(GEOCLUE_SERVICE,)).await?;
        let client = zbus::Proxy::new(connection, GEOCLUE_SERVICE, client_path, GEOCLUE_CLIENT_INTERFACE).await?;
        client.set_property("DesktopId", "chroma").await?;
        client.set_property("RequestedAccuracyLevel", GEOCLUE_ACCURACY_LEVEL_EXACT).await?;
        let updates = client
            .receive_signal("LocationUpdated")
            .await?
            .map(GeoclueLocationUpdate::from_message as fn(zbus::Message) -> Result<GeoclueLocationUpdate>);
        let updates = GeoclueLocationUpdateAwaiter::new(updates);
        let _: () = client.call("Start", &()).await?;
        Ok(Self { owner, updates })
    }
}

/// Private integration witness. It is deliberately ignored in ordinary cargo
/// runs and has no missing-bus escape: the durable Nix check runs it inside
/// `dbus-run-session`, against a fake GeoClue service on that private bus.
#[cfg(test)]
mod private_session_bus_tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::time::Duration;

    use kameo::actor::{Actor, ActorRef, Spawn};
    use kameo::error::Infallible;
    use kameo::message::{Context, Message};
    use tokio::sync::mpsc::{Receiver, Sender, channel};
    use tokio::time::timeout;
    use zbus::Connection;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{ObjectPath, OwnedObjectPath};

    use super::{
        GEOCLUE_MANAGER_PATH, GEOCLUE_SERVICE, GeoclueWatcher, LocationAnnounced, MAX_LOCATION_ACCURACY_METERS,
    };
    use crate::schedule::Location;

    const CLIENT_PATH: &str = "/org/freedesktop/GeoClue2/Client/1";
    const HOME_PATH: &str = "/org/freedesktop/GeoClue2/Location/home";
    const AWAY_PATH: &str = "/org/freedesktop/GeoClue2/Location/away";
    const IMPRECISE_PATH: &str = "/org/freedesktop/GeoClue2/Location/imprecise";
    const HOME: Location = Location { latitude: 1.0, longitude: 1.0 };
    const AWAY: Location = Location { latitude: 2.0, longitude: 2.0 };

    struct FakeManager {
        created: Arc<AtomicUsize>,
    }

    #[zbus::interface(name = "org.freedesktop.GeoClue2.Manager")]
    impl FakeManager {
        fn create_client(&self) -> OwnedObjectPath {
            self.created.fetch_add(1, Ordering::SeqCst);
            OwnedObjectPath::try_from(CLIENT_PATH).expect("client path is valid")
        }
    }

    struct FakeClient {
        started: Arc<AtomicUsize>,
        desktop_id: String,
        requested_accuracy_level: Arc<AtomicU32>,
    }

    #[zbus::interface(name = "org.freedesktop.GeoClue2.Client")]
    impl FakeClient {
        /// GeoClue's static source announces its one location when a client starts.
        async fn start(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> zbus::fdo::Result<()> {
            self.started.fetch_add(1, Ordering::SeqCst);
            Self::location_updated(emitter, ObjectPath::from_static_str_unchecked("/"), path(HOME_PATH)).await?;
            Ok(())
        }

        #[zbus(property)]
        fn desktop_id(&self) -> String {
            self.desktop_id.clone()
        }

        #[zbus(property)]
        fn set_desktop_id(&mut self, value: String) {
            self.desktop_id = value;
        }

        #[zbus(property)]
        fn requested_accuracy_level(&self) -> u32 {
            self.requested_accuracy_level.load(Ordering::SeqCst)
        }

        #[zbus(property)]
        fn set_requested_accuracy_level(&self, value: u32) {
            self.requested_accuracy_level.store(value, Ordering::SeqCst);
        }

        #[zbus(signal)]
        async fn location_updated(
            emitter: SignalEmitter<'_>,
            old: ObjectPath<'_>,
            new: ObjectPath<'_>,
        ) -> zbus::Result<()>;
    }

    struct FakeLocation {
        location: Location,
        accuracy: f64,
    }

    #[zbus::interface(name = "org.freedesktop.GeoClue2.Location")]
    impl FakeLocation {
        #[zbus(property)]
        fn latitude(&self) -> f64 {
            self.location.latitude
        }

        #[zbus(property)]
        fn longitude(&self) -> f64 {
            self.location.longitude
        }

        #[zbus(property)]
        fn accuracy(&self) -> f64 {
            self.accuracy
        }
    }

    struct AnnouncementSink {
        announcements: Sender<Location>,
    }

    impl Actor for AnnouncementSink {
        type Args = Self;
        type Error = Infallible;

        async fn on_start(sink: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
            Ok(sink)
        }
    }

    impl Message<LocationAnnounced> for AnnouncementSink {
        type Reply = ();

        async fn handle(&mut self, message: LocationAnnounced, _context: &mut Context<Self, Self::Reply>) {
            let _ = self.announcements.try_send(message.location());
        }
    }

    fn path(text: &'static str) -> ObjectPath<'static> {
        ObjectPath::from_static_str_unchecked(text)
    }

    async fn next_announcement(announcements: &mut Receiver<Location>) -> Location {
        timeout(Duration::from_secs(5), announcements.recv())
            .await
            .expect("an announcement arrives after a LocationUpdated event")
            .expect("the sink is alive")
    }

    async fn announce(service: &Connection, old: &'static str, new: &'static str) {
        let emitter = SignalEmitter::new(service, CLIENT_PATH).expect("client emitter");
        FakeClient::location_updated(emitter, path(old), path(new)).await.expect("emit LocationUpdated");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "run under dbus-run-session; this is the private durable bus witness"]
    async fn held_location_is_kept_without_renewal_and_replaced_by_location_updated() {
        let created = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(AtomicUsize::new(0));
        let requested_accuracy_level = Arc::new(AtomicU32::new(0));
        let service = Connection::session().await.expect("private bus is required");
        let objects = service.object_server();
        objects.at(GEOCLUE_MANAGER_PATH, FakeManager { created: created.clone() }).await.expect("serve manager");
        objects
            .at(
                CLIENT_PATH,
                FakeClient {
                    started: started.clone(),
                    desktop_id: String::new(),
                    requested_accuracy_level: requested_accuracy_level.clone(),
                },
            )
            .await
            .expect("serve client");
        objects.at(HOME_PATH, FakeLocation { location: HOME, accuracy: 25.0 }).await.expect("serve home");
        objects.at(AWAY_PATH, FakeLocation { location: AWAY, accuracy: 25.0 }).await.expect("serve away");
        objects
            .at(
                IMPRECISE_PATH,
                FakeLocation {
                    location: Location { latitude: 3.0, longitude: 3.0 },
                    accuracy: MAX_LOCATION_ACCURACY_METERS + 1.0,
                },
            )
            .await
            .expect("serve imprecise");
        service.request_name(GEOCLUE_SERVICE).await.expect("own the GeoClue name");

        let (sender, mut announcements) = channel(8);
        let sink = AnnouncementSink::spawn(AnnouncementSink { announcements: sender });
        let watcher_connection = Connection::session().await.expect("watcher connection");
        let _watcher = GeoclueWatcher::start(watcher_connection, sink.recipient()).await;

        assert_eq!(next_announcement(&mut announcements).await, HOME);
        assert_eq!(requested_accuracy_level.load(Ordering::SeqCst), 8, "GeoClue's exact accuracy level is requested");

        // Silence from GeoClue keeps the held location: no renewal, no new
        // client, no repeated Start, no further announcement.
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(announcements.try_recv().is_err(), "a held location is not re-read without an event");
        assert_eq!(created.load(Ordering::SeqCst), 1);
        assert_eq!(started.load(Ordering::SeqCst), 1);

        // An imprecise fix is rejected and the next LocationUpdated replaces the held location.
        announce(&service, HOME_PATH, IMPRECISE_PATH).await;
        announce(&service, IMPRECISE_PATH, AWAY_PATH).await;
        assert_eq!(next_announcement(&mut announcements).await, AWAY);

        // The service leaving and retaking its bus name reconnects the client.
        service.release_name(GEOCLUE_SERVICE).await.expect("release the GeoClue name");
        service.request_name(GEOCLUE_SERVICE).await.expect("retake the GeoClue name");
        assert_eq!(next_announcement(&mut announcements).await, HOME);
        assert_eq!(started.load(Ordering::SeqCst), 2);
    }
}
