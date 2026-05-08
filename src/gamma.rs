//! Async client for `wl-gammarelay-rs`.
//!
//! Owns a held zbus session connection and a typed proxy onto
//! `rs.wl-gammarelay /` (interface `rs.wl.gammarelay`). Reads
//! and writes the `Temperature` (q) and `Brightness` (d)
//! properties.

use zbus::Connection;
use zbus::proxy;

use crate::brightness::BrightnessPercent;
use crate::error::Result;
use crate::warmth::KelvinTemperature;

#[proxy(
    interface = "rs.wl.gammarelay",
    default_service = "rs.wl-gammarelay",
    default_path = "/",
    gen_blocking = false
)]
pub trait GammaRelay {
    #[zbus(property)]
    fn temperature(&self) -> zbus::Result<u16>;
    #[zbus(property)]
    fn set_temperature(&self, value: u16) -> zbus::Result<()>;

    #[zbus(property)]
    fn brightness(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn set_brightness(&self, value: f64) -> zbus::Result<()>;
}

/// A long-lived client to the running `wl-gammarelay-rs` daemon.
pub struct GammaClient {
    proxy: GammaRelayProxy<'static>,
}

impl GammaClient {
    /// Open a session-bus connection and bind the typed proxy.
    pub async fn connect() -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = GammaRelayProxy::new(&connection).await?;
        Ok(Self { proxy })
    }

    /// Read the current colour temperature.
    pub async fn temperature(&self) -> Result<KelvinTemperature> {
        let raw = self.proxy.temperature().await?;
        Ok(KelvinTemperature::new(raw))
    }

    /// Write a new colour temperature.
    pub async fn set_temperature(&self, kelvin: KelvinTemperature) -> Result<()> {
        self.proxy.set_temperature(kelvin.as_u16()).await?;
        Ok(())
    }

    /// Read the current brightness as a percent.
    pub async fn brightness(&self) -> Result<BrightnessPercent> {
        let fraction = self.proxy.brightness().await?;
        let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as u8;
        Ok(BrightnessPercent::new(percent))
    }

    /// Write a new brightness percent.
    pub async fn set_brightness(&self, percent: BrightnessPercent) -> Result<()> {
        self.proxy.set_brightness(percent.as_fraction()).await?;
        Ok(())
    }
}
