//! The daemon entry point.
//!
//! Binds the UDS at [`crate::wire::socket_path`], opens a zbus
//! connection to `wl-gammarelay-rs`, and accepts client
//! connections forever. Each connection reads one
//! length-prefixed [`Request`] frame, dispatches through the
//! shared [`DaemonState`] (gamma client + per-axis ramp
//! handles), and writes a single [`Response`] frame back.
//!
//! Per-axis side effects run as detached `tokio::spawn` tasks
//! where the effect can outlive the request. The state holds an
//! [`AbortHandle`] per cancellable axis. Any new `SetTheme`,
//! instant `SetWarmth` / `SetBrightness`, fresh ramp request, or
//! explicit interrupt aborts the older in-flight work before
//! taking effect.

use std::sync::Arc;

use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tokio::task::AbortHandle;
use tokio::time::{Duration, Instant, MissedTickBehavior, interval};

use crate::brightness::BrightnessPercent;
use crate::config::ConfigFile;
use crate::error::Result;
use crate::gamma::GammaClient;
use crate::request::Request;
use crate::response::Response;
use crate::theme::{ThemeApplier, ThemeMode};
use crate::time::RampDuration;
use crate::warmth::KelvinTemperature;
use crate::wire::{read_frame, socket_path, write_frame};

/// Shared per-daemon state. Held behind an `Arc` so connection
/// tasks and ramp tasks can both reach it.
struct DaemonState {
    theme_applier: ThemeApplier,
    theme: Mutex<ThemeMode>,
    theme_apply: Mutex<Option<AbortHandle>>,
    gamma: GammaClient,
    warmth_ramp: Mutex<Option<AbortHandle>>,
    brightness_ramp: Mutex<Option<AbortHandle>>,
}

impl DaemonState {
    fn new(theme_applier: ThemeApplier, gamma: GammaClient) -> Arc<Self> {
        Arc::new(Self {
            theme_applier,
            theme: Mutex::new(ThemeMode::Light),
            theme_apply: Mutex::new(None),
            gamma,
            warmth_ramp: Mutex::new(None),
            brightness_ramp: Mutex::new(None),
        })
    }

    async fn theme(&self) -> ThemeMode {
        *self.theme.lock().await
    }

    async fn set_theme(&self, mode: ThemeMode) -> Result<()> {
        let process = self.theme_applier.spawn(mode)?;
        *self.theme.lock().await = mode;
        self.install_theme_apply(process).await;
        Ok(())
    }

    /// Replace the theme-apply worker, aborting the previous if any.
    async fn install_theme_apply(&self, process: crate::theme::ThemeApplyProcess) {
        let handle = tokio::spawn(async move {
            if let Err(error) = process.wait().await {
                eprintln!("chroma-daemon theme apply error: {error}");
            }
        });

        let mut guard = self.theme_apply.lock().await;
        if let Some(old) = guard.replace(handle.abort_handle()) {
            old.abort();
        }
    }

    /// Abort any active warmth ramp. Idempotent.
    async fn cancel_warmth_ramp(&self) {
        let mut guard = self.warmth_ramp.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    /// Abort any active brightness ramp. Idempotent.
    async fn cancel_brightness_ramp(&self) {
        let mut guard = self.brightness_ramp.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    /// Replace the warmth-ramp handle, aborting the previous if any.
    async fn install_warmth_ramp(&self, handle: AbortHandle) {
        let mut guard = self.warmth_ramp.lock().await;
        if let Some(old) = guard.replace(handle) {
            old.abort();
        }
    }

    /// Replace the brightness-ramp handle, aborting the previous if any.
    async fn install_brightness_ramp(&self, handle: AbortHandle) {
        let mut guard = self.brightness_ramp.lock().await;
        if let Some(old) = guard.replace(handle) {
            old.abort();
        }
    }
}

/// Run the daemon until SIGTERM / Ctrl-C.
pub async fn run() -> Result<()> {
    let path = socket_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    let theme_applier = ThemeApplier::from_apply_command(ConfigFile::from_default_locations()?.theme_apply_command()?);
    let gamma = GammaClient::connect().await?;
    let state = DaemonState::new(theme_applier, gamma);
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
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(error) = serve_connection(stream, &state).await {
                        eprintln!("chroma-daemon connection error: {error}");
                    }
                });
            }
        }
    }

    let _ = std::fs::remove_file(&path);
    Ok(())
}

async fn serve_connection(mut stream: UnixStream, state: &Arc<DaemonState>) -> Result<()> {
    let frame = read_frame(&mut stream).await?;
    let request = Request::from_archive(&frame)?;
    let response = dispatch(request, state).await;
    let archive = response.archive()?;
    write_frame(&mut stream, &archive).await?;
    Ok(())
}

async fn dispatch(request: Request, state: &Arc<DaemonState>) -> Response {
    match request {
        Request::SetTheme { mode } => set_theme(state, mode).await,
        Request::GetTheme {} => Response::Theme { mode: state.theme().await },

        Request::SetWarmth { level } => instant_warmth(state, level.kelvin()).await,
        Request::SetWarmthKelvin { kelvin } => instant_warmth(state, kelvin).await,
        Request::GetWarmth {} => match state.gamma.temperature().await {
            Ok(kelvin) => Response::Warmth { kelvin },
            Err(error) => Response::Error { message: error.to_string() },
        },
        Request::StartWarmthRamp { target, duration } => start_warmth_ramp(state, target.kelvin(), duration).await,
        Request::StartWarmthRampKelvin { target, duration } => start_warmth_ramp(state, target, duration).await,
        Request::InterruptWarmth {} => {
            state.cancel_warmth_ramp().await;
            Response::Accepted {}
        }

        Request::SetBrightness { level } => instant_brightness(state, level.percent()).await,
        Request::SetBrightnessPercent { percent } => instant_brightness(state, percent).await,
        Request::GetBrightness {} => match state.gamma.brightness().await {
            Ok(percent) => Response::Brightness { percent },
            Err(error) => Response::Error { message: error.to_string() },
        },
        Request::StartBrightnessRamp { target, duration } => {
            start_brightness_ramp(state, target.percent(), duration).await
        }
        Request::StartBrightnessRampPercent { target, duration } => {
            start_brightness_ramp(state, target, duration).await
        }
        Request::InterruptBrightness {} => {
            state.cancel_brightness_ramp().await;
            Response::Accepted {}
        }

        Request::GetState {} => match (state.gamma.temperature().await, state.gamma.brightness().await) {
            (Ok(kelvin), Ok(percent)) => Response::State { theme: state.theme().await, kelvin, percent },
            (Err(error), _) | (_, Err(error)) => Response::Error { message: error.to_string() },
        },
    }
}

async fn set_theme(state: &Arc<DaemonState>, mode: ThemeMode) -> Response {
    match state.set_theme(mode).await {
        Ok(()) => Response::Accepted {},
        Err(error) => Response::Error { message: error.to_string() },
    }
}

async fn instant_warmth(state: &Arc<DaemonState>, kelvin: KelvinTemperature) -> Response {
    let owned = Arc::clone(state);
    let handle = tokio::spawn(async move {
        if let Err(error) = owned.gamma.set_temperature(kelvin).await {
            eprintln!("chroma-daemon warmth apply error: {error}");
        }
    });
    state.install_warmth_ramp(handle.abort_handle()).await;
    Response::Accepted {}
}

async fn instant_brightness(state: &Arc<DaemonState>, percent: BrightnessPercent) -> Response {
    let owned = Arc::clone(state);
    let handle = tokio::spawn(async move {
        if let Err(error) = owned.gamma.set_brightness(percent).await {
            eprintln!("chroma-daemon brightness apply error: {error}");
        }
    });
    state.install_brightness_ramp(handle.abort_handle()).await;
    Response::Accepted {}
}

async fn start_warmth_ramp(state: &Arc<DaemonState>, target: KelvinTemperature, duration: RampDuration) -> Response {
    let owned = Arc::clone(state);
    let handle = tokio::spawn(async move {
        let from = match owned.gamma.temperature().await {
            Ok(kelvin) => kelvin,
            Err(error) => {
                eprintln!("chroma-daemon warmth ramp read error: {error}");
                return;
            }
        };
        run_warmth_ramp(&owned, from, target, duration).await;
    });
    state.install_warmth_ramp(handle.abort_handle()).await;
    Response::Accepted {}
}

async fn start_brightness_ramp(
    state: &Arc<DaemonState>,
    target: BrightnessPercent,
    duration: RampDuration,
) -> Response {
    let owned = Arc::clone(state);
    let handle = tokio::spawn(async move {
        let from = match owned.gamma.brightness().await {
            Ok(percent) => percent,
            Err(error) => {
                eprintln!("chroma-daemon brightness ramp read error: {error}");
                return;
            }
        };
        run_brightness_ramp(&owned, from, target, duration).await;
    });
    state.install_brightness_ramp(handle.abort_handle()).await;
    Response::Accepted {}
}

async fn run_warmth_ramp(
    state: &Arc<DaemonState>,
    from: KelvinTemperature,
    target: KelvinTemperature,
    duration: RampDuration,
) {
    if from == target {
        return;
    }
    let total = duration.as_duration();
    let mut ticker = interval(compute_tick(total));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let start = Instant::now();
    loop {
        ticker.tick().await;
        let elapsed = start.elapsed();
        let fraction = (elapsed.as_secs_f64() / total.as_secs_f64()).min(1.0);
        let kelvin = from.lerp_to(target, fraction);
        if state.gamma.set_temperature(kelvin).await.is_err() {
            return;
        }
        if fraction >= 1.0 {
            return;
        }
    }
}

async fn run_brightness_ramp(
    state: &Arc<DaemonState>,
    from: BrightnessPercent,
    target: BrightnessPercent,
    duration: RampDuration,
) {
    if from == target {
        return;
    }
    let total = duration.as_duration();
    let mut ticker = interval(compute_tick(total));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let start = Instant::now();
    loop {
        ticker.tick().await;
        let elapsed = start.elapsed();
        let fraction = (elapsed.as_secs_f64() / total.as_secs_f64()).min(1.0);
        let percent = from.lerp_to(target, fraction);
        if state.gamma.set_brightness(percent).await.is_err() {
            return;
        }
        if fraction >= 1.0 {
            return;
        }
    }
}

/// Choose a tick interval that gives ~60 evenly-spaced steps,
/// clamped to `[100ms, 1s]`. A 60-minute ramp ticks every 1s
/// (3600 writes); a 30-second ramp ticks every 500ms (60
/// writes); a 6-second ramp ticks every 100ms (60 writes).
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
