//! The daemon entry point.
//!
//! The runtime root is a Kameo actor. It owns the state actor,
//! one actor per visual axis, the theme-concern fanout actor, and
//! the schedule actor. Mutating CLI requests persist the accepted
//! state first, enqueue the apply work to the owning actor, and
//! return `Accepted` without waiting for desktop, Ghostty,
//! Emacs, or gamma side effects to finish.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Local;
use futures_util::StreamExt;
use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};
use kameo::reply::DelegatedReply;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{Duration, Instant, MissedTickBehavior, interval, timeout};
use zbus::zvariant::OwnedObjectPath;

use crate::brightness::BrightnessPercent;
use crate::config::{Config, ConfigFile};
use crate::error::{Error, Result};
use crate::gamma::GammaClient;
use crate::geoclue::{FreshGeoclueLocation, GeoclueLocationFix, GeoclueLocationUpdate, GeoclueLocationUpdateAwaiter};
use crate::request::Request;
use crate::response::Response;
use crate::schedule::{Location, SchedulePlan, ScheduledBrightness, ScheduledValues, ScheduledWarmth};
use crate::solar_time::SolarClockProjection;
use crate::state::{
    ReadStoredLocation, ReadStoredState, RecordBrightness, RecordLocation, RecordTheme, RecordWarmth, StateStore,
    StoredVisualState,
};
use crate::theme::{ApplyTheme, ThemeApplier, ThemeMode};
use crate::time::RampDuration;
use crate::warmth::KelvinTemperature;
use crate::wire::{read_frame, socket_path, write_frame};

const GEOCLUE_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const POST_RESUME_LOCATION_REFRESH_DELAY: Duration = Duration::from_secs(5);
// A stale GeoClue cache may outlive an otherwise valid held location. Retry
// coarsely so the solar status recovers instead of remaining unavailable.
const LOCATION_REFRESH_RETRY_DELAY: Duration = Duration::from_secs(60);

/// Run the daemon until SIGTERM / Ctrl-C.
pub async fn run() -> Result<()> {
    let config_file = ConfigFile::from_default_locations()?;
    let config = config_file.config()?;
    let root = ChromaRoot::start(config).await?;
    let config_watcher = ConfigWatcher::start(config_file, root.clone()).await?;
    let sleep_watcher = SleepTransitionWatcher::start(root.clone()).await;
    root.ask(ReapplyCurrentState).await.map_err(|error| Error::ActorCall { message: error.to_string() })?;
    root.ask(BeginSchedule).await.map_err(|error| Error::ActorCall { message: error.to_string() })?;

    let path = socket_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    eprintln!("chroma-daemon listening on {}", path.display());

    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                eprintln!("chroma-daemon shutting down");
                break;
            }
            accepted = listener.accept() => {
                let (stream, _addr) = accepted?;
                if let Err(error) = serve_connection(stream, &root).await {
                    eprintln!("chroma-daemon connection error: {error}");
                }
            }
        }
    }

    let _ = config_watcher.stop_gracefully().await;
    let _ = sleep_watcher.stop_gracefully().await;
    sleep_watcher.wait_for_shutdown().await;
    config_watcher.wait_for_shutdown().await;
    let _ = root.stop_gracefully().await;
    root.wait_for_shutdown().await;
    let _ = std::fs::remove_file(&path);
    Ok(())
}

async fn serve_connection(mut stream: UnixStream, root: &ActorRef<ChromaRoot>) -> Result<()> {
    let frame = timeout(Duration::from_secs(2), read_frame(&mut stream))
        .await
        .map_err(|_| Error::Daemon { message: "client read timed out".into() })??;
    let request = Request::from_archive(&frame)?;
    let response = match root.ask(DispatchRequest { request }).await {
        Ok(outcome) => outcome.response,
        Err(error) => Response::Error { message: error.to_string() },
    };
    let archive = response.archive()?;
    timeout(Duration::from_secs(2), write_frame(&mut stream, &archive))
        .await
        .map_err(|_| Error::Daemon { message: "client write timed out".into() })??;
    Ok(())
}

struct ChromaRoot {
    theme: ThemeMode,
    warmth: KelvinTemperature,
    brightness: BrightnessPercent,
    state_store: ActorRef<StateStore>,
    theme_applier: ActorRef<ThemeApplier>,
    warmth_applier: ActorRef<WarmthApplier>,
    brightness_applier: ActorRef<BrightnessApplier>,
    schedule_engine: Option<ActorRef<ScheduleEngine>>,
}

impl ChromaRoot {
    async fn start(config: Config) -> Result<ActorRef<Self>> {
        let state_store = StateStore::start_default()?;
        state_store.wait_for_startup().await;

        let fallback = StoredVisualState {
            theme: config.theme.schedule.default_mode(),
            kelvin: config.warmth.schedule.default_level().kelvin(),
            percent: config.brightness.schedule.default_level().percent(),
        };
        let stored = state_store
            .ask(ReadStoredState { fallback })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;
        let stored_location = state_store
            .ask(ReadStoredLocation)
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;

        let theme_applier = ThemeApplier::start(config.theme.clone()).await;
        let warmth_applier = WarmthApplier::start(GammaClient::connect().await?).await;
        let brightness_applier = BrightnessApplier::start(GammaClient::connect().await?).await;
        let reference = Self::spawn(Self {
            theme: stored.theme,
            warmth: stored.kelvin,
            brightness: stored.percent,
            state_store: state_store.clone(),
            theme_applier,
            warmth_applier,
            brightness_applier,
            schedule_engine: None,
        });
        reference.wait_for_startup().await;
        let schedule_engine =
            ScheduleEngine::start(config, reference.clone(), state_store.clone(), stored_location).await;
        reference
            .ask(InstallSchedule { schedule_engine })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;
        Ok(reference)
    }

    async fn persist_theme(&self, mode: ThemeMode) -> Result<()> {
        self.state_store
            .ask(RecordTheme { mode })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn persist_warmth(&self, kelvin: KelvinTemperature) -> Result<()> {
        self.state_store
            .ask(RecordWarmth { kelvin })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn persist_brightness(&self, percent: BrightnessPercent) -> Result<()> {
        self.state_store
            .ask(RecordBrightness { percent })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn enqueue_theme(&self, mode: ThemeMode) -> Result<()> {
        self.theme_applier
            .ask(ApplyTheme { mode })
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn enqueue_warmth(&self, message: WarmthApplication) -> Result<()> {
        self.warmth_applier.tell(message).await.map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn enqueue_brightness(&self, message: BrightnessApplication) -> Result<()> {
        self.brightness_applier.tell(message).await.map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn set_theme(&mut self, mode: ThemeMode) -> Result<Response> {
        self.theme = mode;
        self.persist_theme(mode).await?;
        self.enqueue_theme(mode).await?;
        Ok(Response::Accepted)
    }

    async fn instant_warmth(&mut self, kelvin: KelvinTemperature) -> Result<Response> {
        self.warmth = kelvin;
        self.persist_warmth(kelvin).await?;
        self.enqueue_warmth(WarmthApplication::Set { kelvin }).await?;
        Ok(Response::Accepted)
    }

    async fn ramp_warmth(&mut self, target: KelvinTemperature, duration: RampDuration) -> Result<Response> {
        self.warmth = target;
        self.persist_warmth(target).await?;
        self.enqueue_warmth(WarmthApplication::Ramp { target, duration }).await?;
        Ok(Response::Accepted)
    }

    async fn schedule_warmth_transition(
        &mut self,
        current: KelvinTemperature,
        target: KelvinTemperature,
        remaining_duration: RampDuration,
    ) -> Result<()> {
        self.warmth = target;
        self.persist_warmth(target).await?;
        self.enqueue_warmth(WarmthApplication::RampFrom { current, target, duration: remaining_duration }).await
    }

    async fn instant_brightness(&mut self, percent: BrightnessPercent) -> Result<Response> {
        self.brightness = percent;
        self.persist_brightness(percent).await?;
        self.enqueue_brightness(BrightnessApplication::Set { percent }).await?;
        Ok(Response::Accepted)
    }

    async fn ramp_brightness(&mut self, target: BrightnessPercent, duration: RampDuration) -> Result<Response> {
        self.brightness = target;
        self.persist_brightness(target).await?;
        self.enqueue_brightness(BrightnessApplication::Ramp { target, duration }).await?;
        Ok(Response::Accepted)
    }

    async fn schedule_brightness_transition(
        &mut self,
        current: BrightnessPercent,
        target: BrightnessPercent,
        remaining_duration: RampDuration,
    ) -> Result<()> {
        self.brightness = target;
        self.persist_brightness(target).await?;
        self.enqueue_brightness(BrightnessApplication::RampFrom { current, target, duration: remaining_duration }).await
    }

    async fn dispatch(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::SetTheme { mode } => self.set_theme(mode).await,
            Request::GetTheme => Ok(Response::Theme { mode: self.theme }),

            Request::SetWarmth { level } => self.instant_warmth(level.kelvin()).await,
            Request::SetWarmthKelvin { kelvin } => self.instant_warmth(kelvin).await,
            Request::GetWarmth => Ok(Response::Warmth { kelvin: self.warmth }),
            Request::StartWarmthRamp { target, duration } => self.ramp_warmth(target.kelvin(), duration).await,
            Request::StartWarmthRampKelvin { target, duration } => self.ramp_warmth(target, duration).await,
            Request::InterruptWarmth => {
                self.enqueue_warmth(WarmthApplication::Interrupt).await?;
                Ok(Response::Accepted)
            }

            Request::SetBrightness { level } => self.instant_brightness(level.percent()).await,
            Request::SetBrightnessPercent { percent } => self.instant_brightness(percent).await,
            Request::GetBrightness => Ok(Response::Brightness { percent: self.brightness }),
            Request::StartBrightnessRamp { target, duration } => self.ramp_brightness(target.percent(), duration).await,
            Request::StartBrightnessRampPercent { target, duration } => self.ramp_brightness(target, duration).await,
            Request::InterruptBrightness => {
                self.enqueue_brightness(BrightnessApplication::Interrupt).await?;
                Ok(Response::Accepted)
            }

            Request::GetState => {
                Ok(Response::State { theme: self.theme, kelvin: self.warmth, percent: self.brightness })
            }
            Request::GetSolarClock => self.solar_clock().await,
        }
    }

    async fn solar_clock(&self) -> Result<Response> {
        let schedule_engine = self
            .schedule_engine
            .as_ref()
            .ok_or_else(|| Error::Daemon { message: "schedule engine is not installed".into() })?;
        let projection = schedule_engine
            .ask(ReadSolarClock)
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;
        Ok(match projection {
            Some(projection) => Response::SolarClock {
                utc_offset_seconds: projection.utc_offset_seconds(),
                valid_until_unix_seconds: projection.valid_until_unix_seconds(),
            },
            None => Response::SolarClockUnavailable,
        })
    }

    async fn apply_scheduled_state(&mut self, values: ScheduledValues) -> Result<()> {
        if let Some(theme) = values.theme {
            self.set_theme(theme).await?;
        }
        if let Some(warmth) = values.warmth {
            match warmth {
                ScheduledWarmth::Settled { kelvin } => {
                    self.instant_warmth(kelvin).await?;
                }
                ScheduledWarmth::Transition { current_kelvin, target_kelvin, remaining_duration } => {
                    self.schedule_warmth_transition(current_kelvin, target_kelvin, remaining_duration).await?;
                }
            }
        }
        if let Some(brightness) = values.brightness {
            match brightness {
                ScheduledBrightness::Settled { percent } => {
                    self.instant_brightness(percent).await?;
                }
                ScheduledBrightness::Transition { current_percent, target_percent, remaining_duration } => {
                    self.schedule_brightness_transition(current_percent, target_percent, remaining_duration).await?;
                }
            }
        }
        Ok(())
    }

    async fn install_config(&mut self, config: Config, root: ActorRef<ChromaRoot>) -> Result<()> {
        let next_theme_applier = ThemeApplier::start(config.theme.clone()).await;
        let stored_location = self
            .state_store
            .ask(ReadStoredLocation)
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;
        let next_schedule_engine = ScheduleEngine::start(config, root, self.state_store.clone(), stored_location).await;
        let previous_theme_applier = std::mem::replace(&mut self.theme_applier, next_theme_applier);
        let previous_schedule_engine = self.schedule_engine.replace(next_schedule_engine.clone());

        self.enqueue_theme(self.theme).await?;
        next_schedule_engine
            .tell(ReconcileSchedule)
            .await
            .map_err(|error| Error::ActorCall { message: error.to_string() })?;

        let _ = previous_theme_applier.stop_gracefully().await;
        if let Some(previous_schedule_engine) = previous_schedule_engine {
            let _ = previous_schedule_engine.stop_gracefully().await;
        }
        Ok(())
    }
}

impl Actor for ChromaRoot {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(root: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(root)
    }
}

struct ReapplyCurrentState;

impl Message<ReapplyCurrentState> for ChromaRoot {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        _message: ReapplyCurrentState,
        _context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.enqueue_theme(self.theme).await?;
        self.enqueue_warmth(WarmthApplication::Set { kelvin: self.warmth }).await?;
        self.enqueue_brightness(BrightnessApplication::Set { percent: self.brightness }).await?;
        Ok(())
    }
}

struct DispatchRequest {
    request: Request,
}

impl Message<DispatchRequest> for ChromaRoot {
    type Reply = DispatchOutcome;

    async fn handle(&mut self, message: DispatchRequest, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let response = match self.dispatch(message.request).await {
            Ok(response) => response,
            Err(error) => Response::Error { message: error.to_string() },
        };
        DispatchOutcome { response }
    }
}

#[derive(kameo::Reply)]
struct DispatchOutcome {
    response: Response,
}

struct BeginSchedule;

impl Message<BeginSchedule> for ChromaRoot {
    type Reply = Result<()>;

    async fn handle(&mut self, _message: BeginSchedule, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let schedule_engine = self
            .schedule_engine
            .as_ref()
            .ok_or_else(|| Error::Daemon { message: "schedule engine is not installed".into() })?;
        schedule_engine.tell(ReconcileSchedule).await.map_err(|error| Error::ActorCall { message: error.to_string() })
    }
}

struct InstallSchedule {
    schedule_engine: ActorRef<ScheduleEngine>,
}

impl Message<InstallSchedule> for ChromaRoot {
    type Reply = Result<()>;

    async fn handle(&mut self, message: InstallSchedule, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.schedule_engine = Some(message.schedule_engine);
        Ok(())
    }
}

struct ResumeFromSleep;

impl Message<ResumeFromSleep> for ChromaRoot {
    type Reply = ();

    async fn handle(&mut self, _message: ResumeFromSleep, _context: &mut Context<Self, Self::Reply>) {
        let Some(schedule_engine) = self.schedule_engine.as_ref() else {
            eprintln!("chroma-daemon resume ignored because schedule engine is not installed");
            return;
        };
        if let Err(error) = schedule_engine.tell(ResumeSchedule).await {
            eprintln!("chroma-daemon resume schedule enqueue error: {error}");
        }
    }
}

struct SleepTransitionWatcher {
    root: ActorRef<ChromaRoot>,
}

impl SleepTransitionWatcher {
    async fn start(root: ActorRef<ChromaRoot>) -> ActorRef<Self> {
        let reference = Self::spawn(Self { root });
        reference.wait_for_startup().await;
        if let Err(error) = reference.tell(StartWatchingSleepTransitions).await {
            eprintln!("chroma-daemon sleep watcher start error: {error}");
        }
        reference
    }
}

impl Actor for SleepTransitionWatcher {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(watcher: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(watcher)
    }
}

struct StartWatchingSleepTransitions;

impl Message<StartWatchingSleepTransitions> for SleepTransitionWatcher {
    type Reply = DelegatedReply<()>;

    async fn handle(
        &mut self,
        _message: StartWatchingSleepTransitions,
        context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let root = self.root.clone();
        context.spawn(async move {
            match SleepTransitionSubscription::connect(root).await {
                Ok(mut subscription) => {
                    if let Err(error) = subscription.run().await {
                        eprintln!("chroma-daemon sleep watcher error: {error}");
                    }
                }
                Err(error) => eprintln!("chroma-daemon sleep watcher connect error: {error}"),
            }
        })
    }
}

struct SleepTransitionSubscription {
    root: ActorRef<ChromaRoot>,
    signals: zbus::proxy::SignalStream<'static>,
}

impl SleepTransitionSubscription {
    async fn connect(root: ActorRef<ChromaRoot>) -> Result<Self> {
        let connection = zbus::Connection::system().await?;
        let manager = zbus::Proxy::new(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .await?;
        let signals = manager.receive_signal("PrepareForSleep").await?;
        Ok(Self { root, signals })
    }

    async fn run(&mut self) -> Result<()> {
        while let Some(message) = self.signals.next().await {
            let sleeping = message.body().deserialize::<bool>()?;
            if !sleeping && let Err(error) = self.root.tell(ResumeFromSleep).await {
                eprintln!("chroma-daemon resume enqueue error: {error}");
            }
        }
        Ok(())
    }
}

struct ApplyScheduledState {
    values: ScheduledValues,
}

impl Message<ApplyScheduledState> for ChromaRoot {
    type Reply = ();

    async fn handle(&mut self, message: ApplyScheduledState, _context: &mut Context<Self, Self::Reply>) {
        if let Err(error) = self.apply_scheduled_state(message.values).await {
            eprintln!("chroma-daemon schedule apply error: {error}");
        }
    }
}

struct InstallConfig {
    config: Config,
}

impl Message<InstallConfig> for ChromaRoot {
    type Reply = Result<()>;

    async fn handle(&mut self, message: InstallConfig, context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.install_config(message.config, context.actor_ref().clone()).await
    }
}

struct ConfigWatcher {
    config_file: ConfigFile,
    root: ActorRef<ChromaRoot>,
    watcher: Option<RecommendedWatcher>,
    generation: u64,
}

impl ConfigWatcher {
    async fn start(config_file: ConfigFile, root: ActorRef<ChromaRoot>) -> Result<ActorRef<Self>> {
        let reference = Self::spawn(Self { config_file, root, watcher: None, generation: 0 });
        reference.wait_for_startup().await;
        reference.ask(StartWatchingConfig).await.map_err(|error| Error::ActorCall { message: error.to_string() })?;
        Ok(reference)
    }

    fn watched_directory(&self) -> Result<PathBuf> {
        self.config_file
            .path()
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| Error::Config { message: format!("{} has no parent directory", self.config_file) })
    }

    fn target_path(&self) -> PathBuf {
        self.config_file.path().to_path_buf()
    }
}

impl Actor for ConfigWatcher {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(watcher: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(watcher)
    }
}

struct StartWatchingConfig;

impl Message<StartWatchingConfig> for ConfigWatcher {
    type Reply = Result<()>;

    async fn handle(&mut self, _message: StartWatchingConfig, context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let watched_directory = self.watched_directory()?;
        let target_path = self.target_path();
        let actor = context.actor_ref().clone();
        let mut watcher = recommended_watcher(move |event: notify::Result<Event>| match event {
            Ok(event) if config_event_touches(&event, &target_path) => {
                let _ = actor.tell(ConfigFileChanged).try_send();
            }
            Ok(_) => {}
            Err(error) => eprintln!("chroma-daemon config watcher error: {error}"),
        })?;
        watcher.watch(&watched_directory, RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);
        Ok(())
    }
}

struct ConfigFileChanged;

impl Message<ConfigFileChanged> for ConfigWatcher {
    type Reply = ();

    async fn handle(&mut self, _message: ConfigFileChanged, context: &mut Context<Self, Self::Reply>) {
        self.generation = self.generation.saturating_add(1);
        let _scheduled = context
            .actor_ref()
            .tell(ReloadChangedConfig { generation: self.generation })
            .send_after(Duration::from_millis(100));
    }
}

struct ReloadChangedConfig {
    generation: u64,
}

impl Message<ReloadChangedConfig> for ConfigWatcher {
    type Reply = ();

    async fn handle(&mut self, message: ReloadChangedConfig, _context: &mut Context<Self, Self::Reply>) {
        if message.generation != self.generation {
            return;
        }
        match self.config_file.config_async().await {
            Ok(config) => {
                if let Err(error) = self.root.tell(InstallConfig { config }).await {
                    eprintln!("chroma-daemon config reload enqueue error: {error}");
                }
            }
            Err(error) => eprintln!("chroma-daemon config reload error: {error}"),
        }
    }
}

fn config_event_touches(event: &Event, target_path: &Path) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    let target_file_name = target_path.file_name();
    event.paths.iter().any(|path| path == target_path || path.file_name() == target_file_name)
}

struct WarmthApplier {
    gamma: GammaClient,
    generation: Arc<AtomicU64>,
}

impl WarmthApplier {
    async fn start(gamma: GammaClient) -> ActorRef<Self> {
        let reference = Self::spawn(Self { gamma, generation: Arc::new(AtomicU64::new(0)) });
        reference.wait_for_startup().await;
        reference
    }

    fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst).saturating_add(1)
    }
}

impl Actor for WarmthApplier {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(actor: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(actor)
    }
}

enum WarmthApplication {
    Set { kelvin: KelvinTemperature },
    Ramp { target: KelvinTemperature, duration: RampDuration },
    RampFrom { current: KelvinTemperature, target: KelvinTemperature, duration: RampDuration },
    Interrupt,
}

impl Message<WarmthApplication> for WarmthApplier {
    type Reply = DelegatedReply<()>;

    async fn handle(&mut self, message: WarmthApplication, context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let generation = self.next_generation();
        let active = Arc::clone(&self.generation);
        let gamma = self.gamma.clone();
        context.spawn(async move {
            match message {
                WarmthApplication::Set { kelvin } => {
                    if active.load(Ordering::SeqCst) == generation
                        && let Err(error) = gamma.set_temperature(kelvin).await
                    {
                        eprintln!("chroma-daemon warmth apply error: {error}");
                    }
                }
                WarmthApplication::Ramp { target, duration } => {
                    run_warmth_ramp(gamma, active, generation, target, duration).await;
                }
                WarmthApplication::RampFrom { current, target, duration } => {
                    run_warmth_ramp_from(gamma, active, generation, current, target, duration).await;
                }
                WarmthApplication::Interrupt => {}
            }
        })
    }
}

struct BrightnessApplier {
    gamma: GammaClient,
    generation: Arc<AtomicU64>,
}

impl BrightnessApplier {
    async fn start(gamma: GammaClient) -> ActorRef<Self> {
        let reference = Self::spawn(Self { gamma, generation: Arc::new(AtomicU64::new(0)) });
        reference.wait_for_startup().await;
        reference
    }

    fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst).saturating_add(1)
    }
}

impl Actor for BrightnessApplier {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(actor: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(actor)
    }
}

enum BrightnessApplication {
    Set { percent: BrightnessPercent },
    Ramp { target: BrightnessPercent, duration: RampDuration },
    RampFrom { current: BrightnessPercent, target: BrightnessPercent, duration: RampDuration },
    Interrupt,
}

impl Message<BrightnessApplication> for BrightnessApplier {
    type Reply = DelegatedReply<()>;

    async fn handle(
        &mut self,
        message: BrightnessApplication,
        context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let generation = self.next_generation();
        let active = Arc::clone(&self.generation);
        let gamma = self.gamma.clone();
        context.spawn(async move {
            match message {
                BrightnessApplication::Set { percent } => {
                    if active.load(Ordering::SeqCst) == generation
                        && let Err(error) = gamma.set_brightness(percent).await
                    {
                        eprintln!("chroma-daemon brightness apply error: {error}");
                    }
                }
                BrightnessApplication::Ramp { target, duration } => {
                    run_brightness_ramp(gamma, active, generation, target, duration).await;
                }
                BrightnessApplication::RampFrom { current, target, duration } => {
                    run_brightness_ramp_from(gamma, active, generation, current, target, duration).await;
                }
                BrightnessApplication::Interrupt => {}
            }
        })
    }
}

async fn run_warmth_ramp(
    gamma: GammaClient,
    active: Arc<AtomicU64>,
    generation: u64,
    target: KelvinTemperature,
    duration: RampDuration,
) {
    let from = match gamma.temperature().await {
        Ok(kelvin) => kelvin,
        Err(error) => {
            eprintln!("chroma-daemon warmth ramp read error: {error}");
            return;
        }
    };
    run_warmth_ramp_from(gamma, active, generation, from, target, duration).await;
}

async fn run_warmth_ramp_from(
    gamma: GammaClient,
    active: Arc<AtomicU64>,
    generation: u64,
    from: KelvinTemperature,
    target: KelvinTemperature,
    duration: RampDuration,
) {
    if active.load(Ordering::SeqCst) != generation {
        return;
    }
    if let Err(error) = gamma.set_temperature(from).await {
        eprintln!("chroma-daemon warmth ramp apply error: {error}");
        return;
    }
    if from == target {
        return;
    }
    let total = duration.as_duration();
    let mut ticker = interval(compute_tick(total));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let start = Instant::now();
    loop {
        ticker.tick().await;
        if active.load(Ordering::SeqCst) != generation {
            return;
        }
        let elapsed = start.elapsed();
        let fraction = (elapsed.as_secs_f64() / total.as_secs_f64()).min(1.0);
        if let Err(error) = gamma.set_temperature(from.lerp_to(target, fraction)).await {
            eprintln!("chroma-daemon warmth ramp apply error: {error}");
            return;
        }
        if fraction >= 1.0 {
            return;
        }
    }
}

async fn run_brightness_ramp(
    gamma: GammaClient,
    active: Arc<AtomicU64>,
    generation: u64,
    target: BrightnessPercent,
    duration: RampDuration,
) {
    let from = match gamma.brightness().await {
        Ok(percent) => percent,
        Err(error) => {
            eprintln!("chroma-daemon brightness ramp read error: {error}");
            return;
        }
    };
    run_brightness_ramp_from(gamma, active, generation, from, target, duration).await;
}

async fn run_brightness_ramp_from(
    gamma: GammaClient,
    active: Arc<AtomicU64>,
    generation: u64,
    from: BrightnessPercent,
    target: BrightnessPercent,
    duration: RampDuration,
) {
    if active.load(Ordering::SeqCst) != generation {
        return;
    }
    if let Err(error) = gamma.set_brightness(from).await {
        eprintln!("chroma-daemon brightness ramp apply error: {error}");
        return;
    }
    if from == target {
        return;
    }
    let total = duration.as_duration();
    let mut ticker = interval(compute_tick(total));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let start = Instant::now();
    loop {
        ticker.tick().await;
        if active.load(Ordering::SeqCst) != generation {
            return;
        }
        let elapsed = start.elapsed();
        let fraction = (elapsed.as_secs_f64() / total.as_secs_f64()).min(1.0);
        if let Err(error) = gamma.set_brightness(from.lerp_to(target, fraction)).await {
            eprintln!("chroma-daemon brightness ramp apply error: {error}");
            return;
        }
        if fraction >= 1.0 {
            return;
        }
    }
}

fn compute_tick(total: Duration) -> Duration {
    const MIN_TICK: Duration = Duration::from_millis(100);
    const MAX_TICK: Duration = Duration::from_secs(1);
    let candidate = total / 60;
    if candidate < MIN_TICK {
        MIN_TICK
    } else if candidate > MAX_TICK {
        MAX_TICK
    } else {
        candidate
    }
}

struct ScheduleEngine {
    config: Config,
    root: ActorRef<ChromaRoot>,
    state_store: ActorRef<StateStore>,
    location: Option<Location>,
    fresh_location: Option<FreshGeoclueLocation>,
    schedule_generation: u64,
    location_generation: u64,
}

impl ScheduleEngine {
    async fn start(
        config: Config,
        root: ActorRef<ChromaRoot>,
        state_store: ActorRef<StateStore>,
        location: Option<Location>,
    ) -> ActorRef<Self> {
        let reference = Self::spawn(ScheduleArgs { config, root, state_store, location });
        reference.wait_for_startup().await;
        reference
    }

    async fn current_location() -> Option<FreshGeoclueLocation> {
        match timeout(GEOCLUE_REQUEST_TIMEOUT, async {
            let locator = GeoclueLocator::from_system().await?;
            locator.current_location().await
        })
        .await
        {
            Ok(Ok(location)) => Some(location),
            Ok(Err(error)) => {
                eprintln!("chroma-daemon geoclue location error: {error}");
                None
            }
            Err(_) => {
                eprintln!("chroma-daemon geoclue location timed out");
                None
            }
        }
    }

    fn next_schedule_generation(&mut self) -> u64 {
        self.schedule_generation = self.schedule_generation.saturating_add(1);
        self.schedule_generation
    }

    fn request_location_refresh(&mut self, context: &mut Context<Self, ()>, delay: Duration) {
        self.location_generation = self.location_generation.saturating_add(1);
        let generation = self.location_generation;
        let delivery = context.actor_ref().tell(LocationRefreshDue { generation });
        if delay.is_zero() {
            let _sent = delivery.try_send();
        } else {
            let _scheduled = delivery.send_after(delay);
        }
    }

    async fn reconcile(&mut self, generation: u64, context: &mut Context<Self, ()>) {
        let now = Local::now();
        let plan = SchedulePlan::from_config(&self.config, self.location, now);
        if let Err(error) = self.root.tell(ApplyScheduledState { values: plan.values() }).await {
            eprintln!("chroma-daemon schedule enqueue error: {error}");
        }
        if let Some(next) = plan.next_delay_from(now) {
            let _scheduled = context.actor_ref().tell(ScheduledScheduleEvaluation { generation }).send_after(next);
        } else if self.config.needs_geolocation() && self.location.is_none() {
            let _retry = context
                .actor_ref()
                .tell(ScheduledScheduleEvaluation { generation })
                .send_after(LOCATION_REFRESH_RETRY_DELAY);
        }
    }

    async fn complete_location_refresh(
        &mut self,
        generation: u64,
        location: Option<FreshGeoclueLocation>,
        context: &mut Context<Self, ()>,
    ) {
        if generation != self.location_generation {
            return;
        }
        match location {
            Some(fresh_location) => {
                let location = fresh_location.location();
                if self.location != Some(location) {
                    if let Err(error) = self.state_store.ask(RecordLocation { location }).await {
                        eprintln!("chroma-daemon location persist error: {error}");
                    }
                    self.location = Some(location);
                }
                let refresh_delay = fresh_location.refresh_delay_at(std::time::SystemTime::now());
                self.fresh_location = Some(fresh_location);
                let schedule_generation = self.next_schedule_generation();
                self.reconcile(schedule_generation, context).await;
                self.request_location_refresh(context, refresh_delay);
                eprintln!("chroma-daemon recomputed solar schedule from fresh GeoClue location");
            }
            None => {
                self.request_location_refresh(context, LOCATION_REFRESH_RETRY_DELAY);
            }
        }
    }
}

struct ScheduleArgs {
    config: Config,
    root: ActorRef<ChromaRoot>,
    state_store: ActorRef<StateStore>,
    location: Option<Location>,
}

impl Actor for ScheduleEngine {
    type Args = ScheduleArgs;
    type Error = Infallible;

    async fn on_start(args: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            config: args.config,
            root: args.root,
            state_store: args.state_store,
            location: args.location,
            fresh_location: None,
            schedule_generation: 0,
            location_generation: 0,
        })
    }
}

struct ResumeSchedule;

impl Message<ResumeSchedule> for ScheduleEngine {
    type Reply = ();

    async fn handle(&mut self, _message: ResumeSchedule, context: &mut Context<Self, Self::Reply>) {
        let generation = self.next_schedule_generation();
        self.reconcile(generation, context).await;
        self.request_location_refresh(context, POST_RESUME_LOCATION_REFRESH_DELAY);
    }
}

struct ReconcileSchedule;

impl Message<ReconcileSchedule> for ScheduleEngine {
    type Reply = ();

    async fn handle(&mut self, _message: ReconcileSchedule, context: &mut Context<Self, Self::Reply>) {
        let generation = self.next_schedule_generation();
        self.reconcile(generation, context).await;
        self.request_location_refresh(context, Duration::ZERO);
    }
}

struct ScheduledScheduleEvaluation {
    generation: u64,
}

impl Message<ScheduledScheduleEvaluation> for ScheduleEngine {
    type Reply = ();

    async fn handle(&mut self, message: ScheduledScheduleEvaluation, context: &mut Context<Self, Self::Reply>) {
        if message.generation == self.schedule_generation {
            self.reconcile(message.generation, context).await;
            self.request_location_refresh(context, Duration::ZERO);
        }
    }
}

struct LocationRefreshDue {
    generation: u64,
}

impl Message<LocationRefreshDue> for ScheduleEngine {
    type Reply = DelegatedReply<()>;

    async fn handle(&mut self, message: LocationRefreshDue, context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if message.generation != self.location_generation {
            return context.spawn(async {});
        }
        let actor = context.actor_ref().clone();
        context.spawn(async move {
            let location = ScheduleEngine::current_location().await;
            if let Err(error) = actor.tell(LocationRefreshCompleted { generation: message.generation, location }).await
            {
                eprintln!("chroma-daemon location refresh completion enqueue error: {error}");
            }
        })
    }
}

struct LocationRefreshCompleted {
    generation: u64,
    location: Option<FreshGeoclueLocation>,
}

struct ReadSolarClock;

impl Message<ReadSolarClock> for ScheduleEngine {
    type Reply = Option<SolarClockProjection>;

    async fn handle(&mut self, _message: ReadSolarClock, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.fresh_location
            .filter(|location| location.is_current_at(std::time::SystemTime::now()))
            .map(|location| SolarClockProjection::at(location.location(), chrono::Utc::now()))
    }
}

impl Message<LocationRefreshCompleted> for ScheduleEngine {
    type Reply = ();

    async fn handle(&mut self, message: LocationRefreshCompleted, context: &mut Context<Self, Self::Reply>) {
        self.complete_location_refresh(message.generation, message.location, context).await;
    }
}

struct GeoclueLocator {
    connection: zbus::Connection,
}

impl GeoclueLocator {
    async fn from_system() -> Result<Self> {
        Ok(Self { connection: zbus::Connection::system().await? })
    }

    async fn current_location(&self) -> Result<FreshGeoclueLocation> {
        self.await_location_update().await
    }

    async fn await_location_update(&self) -> Result<FreshGeoclueLocation> {
        let manager = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Manager",
            "org.freedesktop.GeoClue2.Manager",
        )
        .await?;
        let client_path: OwnedObjectPath = manager.call("CreateClient", &()).await?;
        let client = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.GeoClue2",
            client_path,
            "org.freedesktop.GeoClue2.Client",
        )
        .await?;
        client.set_property("DesktopId", "chroma").await?;
        client.set_property("RequestedAccuracyLevel", 1_u32).await?;

        let updates = client.receive_signal("LocationUpdated").await?.map(|message| {
            message
                .body()
                .deserialize::<(OwnedObjectPath, OwnedObjectPath)>()
                .map(GeoclueLocationUpdate::from_signal_body)
                .map_err(Error::from)
        });
        let mut location_updates = GeoclueLocationUpdateAwaiter::new(updates);
        let result = async {
            let _: () = client.call("Start", &()).await?;
            let location_path = location_updates.location_path().await?;
            let location = zbus::Proxy::new(
                &self.connection,
                "org.freedesktop.GeoClue2",
                location_path,
                "org.freedesktop.GeoClue2.Location",
            )
            .await?;
            let latitude: f64 = location.get_property("Latitude").await?;
            let longitude: f64 = location.get_property("Longitude").await?;
            let accuracy_meters: f64 = location.get_property("Accuracy").await?;
            let timestamp: (u64, u64) = location.get_property("Timestamp").await?;
            let location = GeoclueLocationFix::new(Location { latitude, longitude }, accuracy_meters, timestamp)
                .location_at(std::time::SystemTime::now())?;
            eprintln!("chroma-daemon accepted fresh GeoClue location (accuracy: {:.0}m)", accuracy_meters);
            Ok(location)
        }
        .await;
        let _: std::result::Result<(), zbus::Error> = client.call("Stop", &()).await;
        result
    }
}
