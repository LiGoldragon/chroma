//! The daemon entry point.
//!
//! Binds the UDS at [`crate::wire::socket_path`], opens a zbus
//! connection to `wl-gammarelay-rs`, then accepts client
//! connections forever. Each connection reads exactly one
//! length-prefixed [`Request`] frame, dispatches it through
//! the held [`GammaClient`], and writes a single
//! [`Response`] frame back.

use std::sync::Arc;

use tokio::net::{UnixListener, UnixStream};

use crate::brightness::BrightnessPercent;
use crate::error::{Error, Result};
use crate::gamma::GammaClient;
use crate::request::Request;
use crate::response::Response;
use crate::theme::ThemeMode;
use crate::warmth::KelvinTemperature;
use crate::wire::{read_frame, socket_path, write_frame};

/// Run the daemon until SIGTERM / Ctrl-C.
pub async fn run() -> Result<()> {
    let path = socket_path();
    if path.exists() {
        // Stale socket from a previous run. Remove it.
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    let gamma = Arc::new(GammaClient::connect().await?);
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
                let gamma = Arc::clone(&gamma);
                tokio::spawn(async move {
                    if let Err(error) = serve_connection(stream, &gamma).await {
                        eprintln!("chroma-daemon connection error: {error}");
                    }
                });
            }
        }
    }

    let _ = std::fs::remove_file(&path);
    Ok(())
}

async fn serve_connection(mut stream: UnixStream, gamma: &GammaClient) -> Result<()> {
    let frame = read_frame(&mut stream).await?;
    let request = Request::from_archive(&frame)?;
    let response = dispatch(request, gamma).await;
    let archive = response.archive()?;
    write_frame(&mut stream, &archive).await?;
    Ok(())
}

async fn dispatch(request: Request, gamma: &GammaClient) -> Response {
    match request {
        Request::SetTheme { mode: _ } => Response::Error {
            message: "SetTheme is not yet implemented; see report 28 for the apply-command boundary".into(),
        },
        Request::GetTheme {} => Response::Theme { mode: ThemeMode::Light },
        Request::SetWarmth { level } => apply_warmth(gamma, level.kelvin()).await,
        Request::SetWarmthKelvin { kelvin } => apply_warmth(gamma, kelvin).await,
        Request::GetWarmth {} => match gamma.temperature().await {
            Ok(kelvin) => Response::Warmth { kelvin },
            Err(error) => Response::Error { message: error.to_string() },
        },
        Request::SetBrightness { level } => apply_brightness(gamma, level.percent()).await,
        Request::SetBrightnessPercent { percent } => apply_brightness(gamma, percent).await,
        Request::GetBrightness {} => match gamma.brightness().await {
            Ok(percent) => Response::Brightness { percent },
            Err(error) => Response::Error { message: error.to_string() },
        },
        Request::GetState {} => match (gamma.temperature().await, gamma.brightness().await) {
            (Ok(kelvin), Ok(percent)) => Response::State { theme: ThemeMode::Light, kelvin, percent },
            (Err(error), _) | (_, Err(error)) => Response::Error { message: error.to_string() },
        },
    }
}

async fn apply_warmth(gamma: &GammaClient, kelvin: KelvinTemperature) -> Response {
    match gamma.set_temperature(kelvin).await {
        Ok(()) => Response::Acked {},
        Err(error) => Response::Error { message: error.to_string() },
    }
}

async fn apply_brightness(gamma: &GammaClient, percent: BrightnessPercent) -> Response {
    match gamma.set_brightness(percent).await {
        Ok(()) => Response::Acked {},
        Err(error) => Response::Error { message: error.to_string() },
    }
}

// Allow unused error import pending future variants.
#[allow(dead_code)]
fn _silence_unused() -> Error {
    Error::Daemon { message: String::new() }
}
