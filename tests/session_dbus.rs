//! A real session-bus witness for the public Chroma theme protocol.
//!
//! Run this test under `dbus-run-session`; ordinary unit runs deliberately do
//! not invent a session bus.

use chroma::{ProjectionReport, ThemeMode, ThemeProjection, ThemeSnapshot};
use futures_util::StreamExt;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

const SERVICE: &str = "io.github.LiGoldragon.Chroma.Test";
const PATH: &str = "/io/github/LiGoldragon/Chroma/Theme";
const INTERFACE: &str = "io.github.LiGoldragon.Chroma.Theme1";

struct SessionContract {
    projection: ThemeProjection,
}

#[zbus::interface(name = "io.github.LiGoldragon.Chroma.Theme1")]
impl SessionContract {
    #[zbus(out_args("state", "revision"))]
    async fn register_consumer(
        &mut self,
        consumer: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<(String, u64)> {
        let sender = header.sender().ok_or_else(|| zbus::fdo::Error::AccessDenied("unique sender required".into()))?;
        let snapshot = self
            .projection
            .register(consumer, sender.as_str())
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
        Ok((snapshot.mode().dbus_name().to_owned(), snapshot.revision()))
    }

    async fn report_projection(
        &mut self,
        consumer: &str,
        revision: u64,
        result: &str,
        code: &str,
        summary: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<()> {
        if consumer != "emacs" || result != "Applied" || !code.is_empty() || !summary.is_empty() {
            return Err(zbus::fdo::Error::InvalidArgs("unexpected projection report".into()));
        }
        let sender = header.sender().ok_or_else(|| zbus::fdo::Error::AccessDenied("unique sender required".into()))?;
        self.projection
            .report(sender.as_str(), ProjectionReport::applied(revision))
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    #[zbus(signal)]
    async fn desired_state_changed(emitter: SignalEmitter<'_>, state: &str, revision: u64) -> zbus::Result<()>;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_bus_exposes_the_fixed_register_report_and_signal_signatures() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        eprintln!("session-bus witness skipped outside dbus-run-session");
        return;
    }
    let service = zbus::Connection::session().await.expect("connect service session bus");
    service.request_name(SERVICE).await.expect("own test service name");
    service
        .object_server()
        .at(PATH, SessionContract { projection: ThemeProjection::new(ThemeSnapshot::new(ThemeMode::Dark, 4)) })
        .await
        .expect("export interface");

    let client = zbus::Connection::session().await.expect("connect client session bus");
    let proxy = zbus::Proxy::new(&client, SERVICE, PATH, INTERFACE).await.expect("create client proxy");
    let registered: (String, u64) = proxy.call("RegisterConsumer", &("emacs",)).await.expect("register consumer");
    assert_eq!(registered, ("Dark".to_owned(), 4));

    let mut signals = proxy.receive_signal("DesiredStateChanged").await.expect("subscribe signal");
    let emitter = SignalEmitter::new(&service, PATH).expect("build signal emitter");
    SessionContract::desired_state_changed(emitter, "Light", 5).await.expect("emit full desired snapshot");
    let signal = signals.next().await.expect("receive desired-state signal");
    let body: (String, u64) = signal.body().deserialize().expect("decode signal body");
    assert_eq!(body, ("Light".to_owned(), 5));

    proxy
        .call_method("ReportProjection", &("emacs", 4_u64, "Applied", "", ""))
        .await
        .expect("report fixed-shape applied acknowledgement");
}
