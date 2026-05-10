//! The daemon entry point.
//!
//! The runtime root is a Kameo actor. It owns the state actor,
//! one actor per visual axis, the theme-concern fanout actor, and
//! the schedule actor. Mutating CLI requests persist the accepted
//! state first, enqueue the apply work to the owning actor, and
//! return `(Accepted)` without waiting for desktop, Ghostty,
//! Emacs, or gamma side effects to finish.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Local, LocalResult, NaiveDate, TimeZone};
use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};
use kameo::reply::DelegatedReply;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use sunrise::{Coordinates, DawnType, SolarDay, SolarEvent};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{Duration, Instant, MissedTickBehavior, interval, timeout};
use zbus::zvariant::OwnedObjectPath;

use crate::brightness::{BrightnessPercent, BrightnessSchedule};
use crate::config::{Config, ConfigFile};
use crate::error::{Error, Result};
use crate::gamma::GammaClient;
use crate::request::Request;
use crate::response::Response;
use crate::state::{ReadStoredState, RecordBrightness, RecordTheme, RecordWarmth, StateStore, StoredVisualState};
use crate::theme::{ApplyTheme, ThemeApplier, ThemeMode, ThemeSchedule};
use crate::time::{LocalHour, LocalMinute, RampDuration, RampTrigger, SignedMinutes};
use crate::warmth::{KelvinTemperature, WarmthSchedule};
use crate::wire::{read_frame, socket_path, write_frame};

/// Run the daemon until SIGTERM / Ctrl-C.
pub async fn run() -> Result<()> {
    let config_file = ConfigFile::from_default_locations()?;
    let config = config_file.config()?;
    let root = ChromaRoot::start(config).await?;
    let config_watcher = ConfigWatcher::start(config_file, root.clone()).await?;
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

        let theme_applier = ThemeApplier::start(config.theme.clone()).await;
        let warmth_applier = WarmthApplier::start(GammaClient::connect().await?).await;
        let brightness_applier = BrightnessApplier::start(GammaClient::connect().await?).await;
        let reference = Self::spawn(Self {
            theme: stored.theme,
            warmth: stored.kelvin,
            brightness: stored.percent,
            state_store,
            theme_applier,
            warmth_applier,
            brightness_applier,
            schedule_engine: None,
        });
        reference.wait_for_startup().await;
        let schedule_engine = ScheduleEngine::start(config, reference.clone()).await;
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
        Ok(Response::Accepted {})
    }

    async fn instant_warmth(&mut self, kelvin: KelvinTemperature) -> Result<Response> {
        self.warmth = kelvin;
        self.persist_warmth(kelvin).await?;
        self.enqueue_warmth(WarmthApplication::Set { kelvin }).await?;
        Ok(Response::Accepted {})
    }

    async fn ramp_warmth(&mut self, target: KelvinTemperature, duration: RampDuration) -> Result<Response> {
        self.warmth = target;
        self.persist_warmth(target).await?;
        self.enqueue_warmth(WarmthApplication::Ramp { target, duration }).await?;
        Ok(Response::Accepted {})
    }

    async fn instant_brightness(&mut self, percent: BrightnessPercent) -> Result<Response> {
        self.brightness = percent;
        self.persist_brightness(percent).await?;
        self.enqueue_brightness(BrightnessApplication::Set { percent }).await?;
        Ok(Response::Accepted {})
    }

    async fn ramp_brightness(&mut self, target: BrightnessPercent, duration: RampDuration) -> Result<Response> {
        self.brightness = target;
        self.persist_brightness(target).await?;
        self.enqueue_brightness(BrightnessApplication::Ramp { target, duration }).await?;
        Ok(Response::Accepted {})
    }

    async fn dispatch(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::SetTheme { mode } => self.set_theme(mode).await,
            Request::GetTheme {} => Ok(Response::Theme { mode: self.theme }),

            Request::SetWarmth { level } => self.instant_warmth(level.kelvin()).await,
            Request::SetWarmthKelvin { kelvin } => self.instant_warmth(kelvin).await,
            Request::GetWarmth {} => Ok(Response::Warmth { kelvin: self.warmth }),
            Request::StartWarmthRamp { target, duration } => self.ramp_warmth(target.kelvin(), duration).await,
            Request::StartWarmthRampKelvin { target, duration } => self.ramp_warmth(target, duration).await,
            Request::InterruptWarmth {} => {
                self.enqueue_warmth(WarmthApplication::Interrupt).await?;
                Ok(Response::Accepted {})
            }

            Request::SetBrightness { level } => self.instant_brightness(level.percent()).await,
            Request::SetBrightnessPercent { percent } => self.instant_brightness(percent).await,
            Request::GetBrightness {} => Ok(Response::Brightness { percent: self.brightness }),
            Request::StartBrightnessRamp { target, duration } => self.ramp_brightness(target.percent(), duration).await,
            Request::StartBrightnessRampPercent { target, duration } => self.ramp_brightness(target, duration).await,
            Request::InterruptBrightness {} => {
                self.enqueue_brightness(BrightnessApplication::Interrupt).await?;
                Ok(Response::Accepted {})
            }

            Request::GetState {} => {
                Ok(Response::State { theme: self.theme, kelvin: self.warmth, percent: self.brightness })
            }
        }
    }

    async fn apply_scheduled_state(&mut self, values: ScheduledValues) -> Result<()> {
        self.set_theme(values.theme).await?;
        match values.warmth.ramp_duration {
            Some(duration) => {
                self.ramp_warmth(values.warmth.kelvin, duration).await?;
            }
            None => {
                self.instant_warmth(values.warmth.kelvin).await?;
            }
        }
        match values.brightness.ramp_duration {
            Some(duration) => {
                self.ramp_brightness(values.brightness.percent, duration).await?;
            }
            None => {
                self.instant_brightness(values.brightness.percent).await?;
            }
        }
        Ok(())
    }

    async fn install_config(&mut self, config: Config, root: ActorRef<ChromaRoot>) -> Result<()> {
        let next_theme_applier = ThemeApplier::start(config.theme.clone()).await;
        let next_schedule_engine = ScheduleEngine::start(config, root).await;
        let previous_theme_applier = std::mem::replace(&mut self.theme_applier, next_theme_applier);
        let previous_schedule_engine = self.schedule_engine.replace(next_schedule_engine.clone());

        self.enqueue_theme(self.theme).await?;
        next_schedule_engine
            .tell(EvaluateSchedule)
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
        schedule_engine.tell(EvaluateSchedule).await.map_err(|error| Error::ActorCall { message: error.to_string() })
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
    location: Option<Location>,
    evaluation_count: u64,
}

impl ScheduleEngine {
    async fn start(config: Config, root: ActorRef<ChromaRoot>) -> ActorRef<Self> {
        let reference = Self::spawn(ScheduleArgs { config, root });
        reference.wait_for_startup().await;
        reference
    }
}

struct ScheduleArgs {
    config: Config,
    root: ActorRef<ChromaRoot>,
}

impl Actor for ScheduleEngine {
    type Args = ScheduleArgs;
    type Error = Infallible;

    async fn on_start(args: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        let location = if args.config.needs_geolocation() {
            match timeout(Duration::from_secs(2), GeoclueLocator::current_location()).await {
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
        } else {
            None
        };
        Ok(Self { config: args.config, root: args.root, location, evaluation_count: 0 })
    }
}

struct EvaluateSchedule;

impl Message<EvaluateSchedule> for ScheduleEngine {
    type Reply = ();

    async fn handle(&mut self, _message: EvaluateSchedule, context: &mut Context<Self, Self::Reply>) {
        self.evaluation_count = self.evaluation_count.saturating_add(1);
        let now = Local::now();
        let plan = SchedulePlan::from_config(&self.config, self.location, now);
        if let Err(error) = self.root.tell(ApplyScheduledState { values: plan.values }).await {
            eprintln!("chroma-daemon schedule enqueue error: {error}");
        }
        if let Some(next) = plan.next_delay_from(now) {
            let _scheduled = context.actor_ref().tell(EvaluateSchedule).send_after(next);
        } else if self.config.needs_geolocation() && self.location.is_none() {
            let _retry = context.actor_ref().tell(EvaluateSchedule).send_after(Duration::from_secs(300));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScheduledValues {
    theme: ThemeMode,
    warmth: ScheduledWarmth,
    brightness: ScheduledBrightness,
}

#[derive(Debug, Clone, Copy)]
struct ScheduledWarmth {
    kelvin: KelvinTemperature,
    ramp_duration: Option<RampDuration>,
}

#[derive(Debug, Clone, Copy)]
struct ScheduledBrightness {
    percent: BrightnessPercent,
    ramp_duration: Option<RampDuration>,
}

struct SchedulePlan {
    values: ScheduledValues,
    next_at: Option<DateTime<Local>>,
}

impl SchedulePlan {
    fn from_config(config: &Config, location: Option<Location>, now: DateTime<Local>) -> Self {
        let theme = scheduled_theme(&config.theme.schedule, location, now);
        let warmth = scheduled_warmth(&config.warmth.schedule, location, now);
        let brightness = scheduled_brightness(&config.brightness.schedule, location, now);
        Self {
            values: ScheduledValues { theme: theme.value, warmth: warmth.value, brightness: brightness.value },
            next_at: [theme.next_at, warmth.next_at, brightness.next_at].into_iter().flatten().min(),
        }
    }

    fn next_delay_from(&self, now: DateTime<Local>) -> Option<Duration> {
        let next = self.next_at?;
        next.signed_duration_since(now).to_std().ok().map(|duration| duration.max(Duration::from_secs(1)))
    }
}

struct AxisSchedule<T> {
    value: T,
    next_at: Option<DateTime<Local>>,
}

fn scheduled_theme(
    schedule: &ThemeSchedule,
    location: Option<Location>,
    now: DateTime<Local>,
) -> AxisSchedule<ThemeMode> {
    match schedule {
        ThemeSchedule::Manual(mode) => AxisSchedule { value: *mode, next_at: None },
        ThemeSchedule::Scheduled { waypoints, default } => {
            let events = waypoints.iter().flat_map(|waypoint| {
                trigger_datetimes(waypoint.trigger, location, now).map(move |time| (time, waypoint.mode))
            });
            current_value(events, *default, now)
        }
    }
}

fn scheduled_warmth(
    schedule: &WarmthSchedule,
    location: Option<Location>,
    now: DateTime<Local>,
) -> AxisSchedule<ScheduledWarmth> {
    match schedule {
        WarmthSchedule::Manual(level) => {
            AxisSchedule { value: ScheduledWarmth { kelvin: level.kelvin(), ramp_duration: None }, next_at: None }
        }
        WarmthSchedule::Scheduled { waypoints, default } => {
            let events = waypoints.iter().flat_map(|waypoint| {
                trigger_datetimes(waypoint.trigger, location, now).map(move |time| {
                    (
                        time,
                        ScheduledWarmth {
                            kelvin: waypoint.target.kelvin(),
                            ramp_duration: Some(waypoint.ramp_duration),
                        },
                    )
                })
            });
            current_value(events, ScheduledWarmth { kelvin: default.kelvin(), ramp_duration: None }, now)
        }
    }
}

fn scheduled_brightness(
    schedule: &BrightnessSchedule,
    location: Option<Location>,
    now: DateTime<Local>,
) -> AxisSchedule<ScheduledBrightness> {
    match schedule {
        BrightnessSchedule::Manual(level) => {
            AxisSchedule { value: ScheduledBrightness { percent: level.percent(), ramp_duration: None }, next_at: None }
        }
        BrightnessSchedule::Scheduled { waypoints, default } => {
            let events = waypoints.iter().flat_map(|waypoint| {
                trigger_datetimes(waypoint.trigger, location, now).map(move |time| {
                    (
                        time,
                        ScheduledBrightness {
                            percent: waypoint.target.percent(),
                            ramp_duration: Some(waypoint.ramp_duration),
                        },
                    )
                })
            });
            current_value(events, ScheduledBrightness { percent: default.percent(), ramp_duration: None }, now)
        }
    }
}

fn current_value<T>(
    events: impl IntoIterator<Item = (DateTime<Local>, T)>,
    default: T,
    now: DateTime<Local>,
) -> AxisSchedule<T>
where
    T: Copy,
{
    let mut current = default;
    let mut current_at = None;
    let mut next_at = None;
    for (time, value) in events {
        if time <= now && current_at.is_none_or(|at| time > at) {
            current = value;
            current_at = Some(time);
        } else if time > now && next_at.is_none_or(|at| time < at) {
            next_at = Some(time);
        }
    }
    AxisSchedule { value: current, next_at }
}

fn trigger_datetimes(
    trigger: RampTrigger,
    location: Option<Location>,
    now: DateTime<Local>,
) -> impl Iterator<Item = DateTime<Local>> {
    [-1_i64, 0, 1]
        .into_iter()
        .filter_map(move |offset| now.date_naive().checked_add_signed(chrono::Duration::days(offset)))
        .filter_map(move |date| trigger_datetime(trigger, location, date))
}

fn trigger_datetime(trigger: RampTrigger, location: Option<Location>, date: NaiveDate) -> Option<DateTime<Local>> {
    match trigger {
        RampTrigger::TimeOfDay(hour, minute) => local_time_on(date, hour, minute),
        RampTrigger::CivilDawn(offset) => civil_time_on(date, location?, SolarEvent::Dawn(DawnType::Civil), offset),
        RampTrigger::CivilDusk(offset) => civil_time_on(date, location?, SolarEvent::Dusk(DawnType::Civil), offset),
    }
}

fn local_time_on(date: NaiveDate, hour: LocalHour, minute: LocalMinute) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(hour.as_u8() as u32, minute.as_u8() as u32, 0)?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(datetime) => Some(datetime),
        LocalResult::Ambiguous(first, _) => Some(first),
        LocalResult::None => None,
    }
}

fn civil_time_on(
    date: NaiveDate,
    location: Location,
    event: SolarEvent,
    offset: SignedMinutes,
) -> Option<DateTime<Local>> {
    let coordinates = Coordinates::new(location.latitude, location.longitude)?;
    let utc = SolarDay::new(coordinates, date).event_time(event)?;
    Some(utc.with_timezone(&Local) + chrono::Duration::minutes(offset.as_i16() as i64))
}

#[derive(Debug, Clone, Copy)]
struct Location {
    latitude: f64,
    longitude: f64,
}

struct GeoclueLocator;

impl GeoclueLocator {
    async fn current_location() -> Result<Location> {
        let connection = zbus::Connection::system().await?;
        let manager = zbus::Proxy::new(
            &connection,
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Manager",
            "org.freedesktop.GeoClue2.Manager",
        )
        .await?;
        let client_path: OwnedObjectPath = manager.call("CreateClient", &()).await?;
        let client = zbus::Proxy::new(
            &connection,
            "org.freedesktop.GeoClue2",
            client_path.clone(),
            "org.freedesktop.GeoClue2.Client",
        )
        .await?;
        client.set_property("DesktopId", "chroma").await?;
        client.set_property("RequestedAccuracyLevel", 1_u32).await?;
        let _: () = client.call("Start", &()).await?;
        let location_path: OwnedObjectPath = client.get_property("Location").await?;
        let location = zbus::Proxy::new(
            &connection,
            "org.freedesktop.GeoClue2",
            location_path,
            "org.freedesktop.GeoClue2.Location",
        )
        .await?;
        let latitude: f64 = location.get_property("Latitude").await?;
        let longitude: f64 = location.get_property("Longitude").await?;
        let _: std::result::Result<(), zbus::Error> = client.call("Stop", &()).await;
        Ok(Location { latitude, longitude })
    }
}
