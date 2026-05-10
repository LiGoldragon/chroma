//! `ThemeMode` — the desktop's colour-scheme axis.
//!
//! Theme switches are accepted instantly. The daemon fans the
//! requested mode out to independent native concern actors. There is
//! no shell-script apply boundary and no legacy apply-command schema.

use core::fmt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use kameo::actor::{Actor, ActorRef, Spawn, WeakActorRef};
use kameo::error::{ActorStopReason, Infallible};
use kameo::message::{Context, Message};
use nota_codec::NotaEnum;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use tokio::time::{Duration, timeout};
use zbus::zvariant::Value;

use crate::error::{Error, Result};
use crate::time::RampTrigger;

/// The active colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, NotaEnum, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// The lowercase name used in human-facing files and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }

    /// The opposite mode.
    pub const fn toggled(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }

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

impl fmt::Display for ThemeMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A separately-owned native theme application concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeConcern {
    /// Terminal palettes and terminal-local state.
    Terminal,
    /// Desktop/GTK color-scheme state consumed by apps.
    Desktop,
    /// Ghostty configuration.
    Ghostty,
    /// Running Emacs daemons.
    Emacs,
}

/// Read-only complete Ghostty config templates produced by the host profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhosttyConfigTemplates {
    pub dark: PathBuf,
    pub light: PathBuf,
}

impl GhosttyConfigTemplates {
    pub fn template_for(&self, mode: ThemeMode) -> &Path {
        match mode {
            ThemeMode::Dark => &self.dark,
            ThemeMode::Light => &self.light,
        }
    }
}

impl ThemeConcern {
    pub fn from_config_name(name: &str) -> Result<Self> {
        match name {
            "Terminal" => Ok(Self::Terminal),
            "Desktop" | "Gtk" | "GTK" => Ok(Self::Desktop),
            "Ghostty" => Ok(Self::Ghostty),
            "Emacs" => Ok(Self::Emacs),
            "Legacy" | "ApplyCommand" | "ApplyTargets" => {
                Err(Error::Config { message: format!("{name} belongs to the removed apply-command architecture") })
            }
            _ => Err(Error::Config { message: format!("unknown theme concern {name:?}") }),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Desktop => "desktop",
            Self::Ghostty => "ghostty",
            Self::Emacs => "emacs",
        }
    }
}

impl fmt::Display for ThemeConcern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A base16 palette consumed by native theme concerns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePalette {
    pub base00: String,
    pub base01: String,
    pub base02: String,
    pub base03: String,
    pub base04: String,
    pub base05: String,
    pub base06: String,
    pub base07: String,
    pub base08: String,
    pub base09: String,
    pub base0a: String,
    pub base0b: String,
    pub base0c: String,
    pub base0d: String,
    pub base0e: String,
    pub base0f: String,
}

impl ThemePalette {
    pub fn from_base16_slots(slots: [&str; 16]) -> Self {
        Self {
            base00: slots[0].to_string(),
            base01: slots[1].to_string(),
            base02: slots[2].to_string(),
            base03: slots[3].to_string(),
            base04: slots[4].to_string(),
            base05: slots[5].to_string(),
            base06: slots[6].to_string(),
            base07: slots[7].to_string(),
            base08: slots[8].to_string(),
            base09: slots[9].to_string(),
            base0a: slots[10].to_string(),
            base0b: slots[11].to_string(),
            base0c: slots[12].to_string(),
            base0d: slots[13].to_string(),
            base0e: slots[14].to_string(),
            base0f: slots[15].to_string(),
        }
    }

    pub fn fzf_options(&self) -> String {
        format!(
            "--color=bg:{},bg+:{},fg:{},fg+:{}\
             ,hl:{},hl+:{},info:{},marker:{}\
             ,prompt:{},spinner:{},pointer:{},header:{}",
            self.base00,
            self.base01,
            self.base04,
            self.base06,
            self.base0d,
            self.base0d,
            self.base0a,
            self.base0c,
            self.base0a,
            self.base0c,
            self.base0c,
            self.base0d,
        )
    }

    /// Ghostty's ANSI palette entries, using base16's conventional
    /// terminal-color mapping.
    pub fn ghostty_palette_lines(&self) -> String {
        let colors = [
            &self.base00,
            &self.base08,
            &self.base0b,
            &self.base0a,
            &self.base0d,
            &self.base0e,
            &self.base0c,
            &self.base05,
            &self.base03,
            &self.base08,
            &self.base0b,
            &self.base0a,
            &self.base0d,
            &self.base0e,
            &self.base0c,
            &self.base07,
        ];
        let mut lines = String::new();
        for (index, color) in colors.into_iter().enumerate() {
            lines.push_str(&format!("palette = {index}={color}\n"));
        }
        lines
    }
}

/// Dark and light palettes read from NOTA config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePalettes {
    pub dark: ThemePalette,
    pub light: ThemePalette,
}

impl ThemePalettes {
    pub fn for_mode(&self, mode: ThemeMode) -> &ThemePalette {
        match mode {
            ThemeMode::Dark => &self.dark,
            ThemeMode::Light => &self.light,
        }
    }
}

/// Native adapter binaries used only where the platform exposes no
/// stable Rust API. These are direct concern adapters, not shell
/// scripts and not user-configured theme launchers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeAdapters {
    pub dconf: Option<PathBuf>,
    pub emacsclient: Option<PathBuf>,
}

/// One scheduled theme switch — at this trigger, become this mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeWaypoint {
    pub trigger: RampTrigger,
    pub mode: ThemeMode,
}

/// The theme axis's schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSchedule {
    Manual(ThemeMode),
    Scheduled { waypoints: Vec<ThemeWaypoint>, default: ThemeMode },
}

impl ThemeSchedule {
    /// Whether any waypoint in this schedule needs geolocation.
    pub fn needs_geolocation(&self) -> bool {
        match self {
            ThemeSchedule::Manual(_) => false,
            ThemeSchedule::Scheduled { waypoints, .. } => {
                waypoints.iter().any(|waypoint| waypoint.trigger.requires_geolocation())
            }
        }
    }

    /// The mode that holds when no waypoint applies.
    pub fn default_mode(&self) -> ThemeMode {
        match self {
            ThemeSchedule::Manual(mode) => *mode,
            ThemeSchedule::Scheduled { default, .. } => *default,
        }
    }
}

/// The full theme-axis configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeAxis {
    pub concerns: Vec<ThemeConcern>,
    pub palettes: ThemePalettes,
    pub adapters: ThemeAdapters,
    pub font_point_size: u8,
    pub ghostty_config_templates: Option<GhosttyConfigTemplates>,
    pub schedule: ThemeSchedule,
}

/// Applies theme changes through independent native concern actors.
pub struct ThemeApplier {
    concerns: Vec<ThemeConcernReference>,
}

impl ThemeApplier {
    pub async fn start(axis: ThemeAxis) -> ActorRef<Self> {
        let reference = Self::spawn(axis);
        reference.wait_for_startup().await;
        reference
    }

    fn from_axis(axis: ThemeAxis) -> Self {
        let mut concerns = Vec::new();
        for concern in axis.concerns {
            let reference = match concern {
                ThemeConcern::Terminal => {
                    ThemeConcernReference::Terminal(TerminalThemeConcern::spawn(TerminalThemeConcern {
                        palettes: axis.palettes.clone(),
                    }))
                }
                ThemeConcern::Desktop => {
                    ThemeConcernReference::Desktop(DesktopThemeConcern::spawn(DesktopThemeConcern {
                        adapters: axis.adapters.clone(),
                    }))
                }
                ThemeConcern::Ghostty => {
                    let templates = axis
                        .ghostty_config_templates
                        .clone()
                        .expect("Ghostty concern requires GhosttyConfigTemplates in parsed config");
                    ThemeConcernReference::Ghostty(GhosttyThemeConcern::spawn(GhosttyThemeConcern {
                        templates,
                        reloader: GhosttyReloader::application_action(),
                    }))
                }
                ThemeConcern::Emacs => ThemeConcernReference::Emacs(EmacsThemeConcern::spawn(EmacsThemeConcern {
                    adapters: axis.adapters.clone(),
                })),
            };
            concerns.push(reference);
        }
        Self { concerns }
    }
}

#[derive(Clone)]
enum ThemeConcernReference {
    Terminal(ActorRef<TerminalThemeConcern>),
    Desktop(ActorRef<DesktopThemeConcern>),
    Ghostty(ActorRef<GhosttyThemeConcern>),
    Emacs(ActorRef<EmacsThemeConcern>),
}

impl ThemeConcernReference {
    async fn apply(&self, mode: ThemeMode) -> Result<()> {
        match self {
            Self::Terminal(reference) => reference.tell(ApplyThemeConcern { mode }).await,
            Self::Desktop(reference) => reference.tell(ApplyThemeConcern { mode }).await,
            Self::Ghostty(reference) => reference.tell(ApplyThemeConcern { mode }).await,
            Self::Emacs(reference) => reference.tell(ApplyThemeConcern { mode }).await,
        }
        .map_err(|error| Error::ActorCall { message: error.to_string() })
    }

    async fn stop(&self) {
        match self {
            Self::Terminal(reference) => {
                let _ = reference.stop_gracefully().await;
                reference.wait_for_shutdown().await;
            }
            Self::Desktop(reference) => {
                let _ = reference.stop_gracefully().await;
                reference.wait_for_shutdown().await;
            }
            Self::Ghostty(reference) => {
                let _ = reference.stop_gracefully().await;
                reference.wait_for_shutdown().await;
            }
            Self::Emacs(reference) => {
                let _ = reference.stop_gracefully().await;
                reference.wait_for_shutdown().await;
            }
        }
    }
}

impl Actor for ThemeApplier {
    type Args = ThemeAxis;
    type Error = Infallible;

    async fn on_start(axis: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(Self::from_axis(axis))
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> std::result::Result<(), Self::Error> {
        for concern in &self.concerns {
            concern.stop().await;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyTheme {
    pub mode: ThemeMode,
}

impl Message<ApplyTheme> for ThemeApplier {
    type Reply = Result<()>;

    async fn handle(&mut self, message: ApplyTheme, _context: &mut Context<Self, Self::Reply>) -> Self::Reply {
        for concern in &self.concerns {
            concern.apply(message.mode).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ApplyThemeConcern {
    mode: ThemeMode,
}

struct TerminalThemeConcern {
    palettes: ThemePalettes,
}

impl Actor for TerminalThemeConcern {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(concern: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(concern)
    }
}

impl Message<ApplyThemeConcern> for TerminalThemeConcern {
    type Reply = ();

    async fn handle(&mut self, message: ApplyThemeConcern, _context: &mut Context<Self, Self::Reply>) {
        let palette = self.palettes.for_mode(message.mode).clone();
        if let Err(error) = self.apply(message.mode, palette).await {
            eprintln!("chroma-daemon terminal theme concern error: {error}");
        }
    }
}

impl TerminalThemeConcern {
    async fn apply(&self, mode: ThemeMode, palette: ThemePalette) -> Result<()> {
        let state_dir = state_home()?.join("chroma");
        tokio::fs::create_dir_all(&state_dir).await?;
        tokio::fs::write(state_dir.join("current-mode"), format!("{mode}\n")).await?;
        tokio::fs::write(
            state_dir.join("fzf-theme.sh"),
            format!("export FZF_DEFAULT_OPTS=\"$FZF_DEFAULT_OPTS {}\"\n", palette.fzf_options()),
        )
        .await?;
        Ok(())
    }
}

struct DesktopThemeConcern {
    adapters: ThemeAdapters,
}

impl Actor for DesktopThemeConcern {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(concern: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(concern)
    }
}

impl Message<ApplyThemeConcern> for DesktopThemeConcern {
    type Reply = ();

    async fn handle(&mut self, message: ApplyThemeConcern, _context: &mut Context<Self, Self::Reply>) {
        if let Err(error) = self.apply(message.mode).await {
            eprintln!("chroma-daemon desktop theme concern error: {error}");
        }
    }
}

impl DesktopThemeConcern {
    async fn apply(&self, mode: ThemeMode) -> Result<()> {
        if let Some(dconf) = &self.adapters.dconf {
            self.write_dconf(mode, dconf).await?;
        }
        self.write_gtk_settings(mode).await
    }

    async fn write_dconf(&self, mode: ThemeMode, dconf: &Path) -> Result<()> {
        let color_scheme = if mode == ThemeMode::Dark { "'prefer-dark'" } else { "'prefer-light'" };
        let gtk_theme = if mode == ThemeMode::Dark { "'adw-gtk3-dark'" } else { "'adw-gtk3'" };
        let icon_theme = if mode == ThemeMode::Dark { "'Papirus-Dark'" } else { "'Papirus-Light'" };
        self.run_dconf_write(dconf, "/org/gnome/desktop/interface/color-scheme", color_scheme).await?;
        self.run_dconf_write(dconf, "/org/gnome/desktop/interface/gtk-theme", gtk_theme).await?;
        self.run_dconf_write(dconf, "/org/gnome/desktop/interface/icon-theme", icon_theme).await
    }

    async fn run_dconf_write(&self, dconf: &Path, key: &str, value: &str) -> Result<()> {
        let mut child = tokio::process::Command::new(dconf)
            .args(["write", key, value])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        match timeout(Duration::from_secs(1), child.wait()).await {
            Ok(Ok(status)) if status.success() => Ok(()),
            Ok(Ok(status)) => Err(Error::Daemon { message: format!("dconf write {key} exited with {status}") }),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => {
                let _ = child.kill().await;
                Err(Error::Daemon { message: format!("dconf write {key} timed out") })
            }
        }
    }

    async fn write_gtk_settings(&self, mode: ThemeMode) -> Result<()> {
        let config_home = config_home()?;
        let gtk_theme = if mode == ThemeMode::Dark { "adw-gtk3-dark" } else { "adw-gtk3" };
        let icon_theme = if mode == ThemeMode::Dark { "Papirus-Dark" } else { "Papirus-Light" };
        let prefer_dark = if mode == ThemeMode::Dark { "true" } else { "false" };
        let text = format!(
            "[Settings]\n\
             gtk-theme-name={gtk_theme}\n\
             gtk-cursor-theme-name=Bibata-Modern-Classic\n\
             gtk-cursor-theme-size=24\n\
             gtk-font-name=DejaVu Sans 12\n\
             gtk-icon-theme-name={icon_theme}\n\
             gtk-application-prefer-dark-theme={prefer_dark}\n"
        );
        for version in ["gtk-3.0", "gtk-4.0"] {
            let directory = config_home.join(version);
            tokio::fs::create_dir_all(&directory).await?;
            tokio::fs::write(directory.join("settings.ini"), &text).await?;
        }
        Ok(())
    }
}

struct GhosttyThemeConcern {
    templates: GhosttyConfigTemplates,
    reloader: GhosttyReloader,
}

impl Actor for GhosttyThemeConcern {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(concern: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(concern)
    }
}

impl Message<ApplyThemeConcern> for GhosttyThemeConcern {
    type Reply = ();

    async fn handle(&mut self, message: ApplyThemeConcern, _context: &mut Context<Self, Self::Reply>) {
        if let Err(error) = self.apply(message.mode).await {
            eprintln!("chroma-daemon ghostty theme concern error: {error}");
        }
    }
}

impl GhosttyThemeConcern {
    async fn apply(&self, mode: ThemeMode) -> Result<()> {
        let directory = config_home()?.join("ghostty");
        tokio::fs::create_dir_all(&directory).await?;
        let config = tokio::fs::read_to_string(self.templates.template_for(mode)).await?;
        tokio::fs::write(directory.join("config.ghostty"), config).await?;
        self.reloader.reload().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GhosttyReloader {
    bus_name: String,
    object_path: String,
    action_name: String,
}

impl GhosttyReloader {
    fn application_action() -> Self {
        Self {
            bus_name: "com.mitchellh.ghostty".into(),
            object_path: "/com/mitchellh/ghostty".into(),
            action_name: "reload-config".into(),
        }
    }

    async fn reload(&self) -> Result<()> {
        let reload = async {
            let connection = zbus::Connection::session().await?;
            let proxy =
                zbus::Proxy::new(&connection, self.bus_name.as_str(), self.object_path.as_str(), "org.gtk.Actions")
                    .await?;
            let parameters = Vec::<Value<'static>>::new();
            let platform_data = HashMap::<&str, Value<'static>>::new();
            let _: () = proxy.call("Activate", &(self.action_name.as_str(), parameters, platform_data)).await?;
            Ok(())
        };
        match timeout(Duration::from_secs(1), reload).await {
            Ok(result) => result,
            Err(_) => Err(Error::Daemon { message: "ghostty reload-config action timed out".into() }),
        }
    }
}

struct EmacsThemeConcern {
    adapters: ThemeAdapters,
}

impl Actor for EmacsThemeConcern {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(concern: Self::Args, _reference: ActorRef<Self>) -> std::result::Result<Self, Self::Error> {
        Ok(concern)
    }
}

impl Message<ApplyThemeConcern> for EmacsThemeConcern {
    type Reply = ();

    async fn handle(&mut self, message: ApplyThemeConcern, _context: &mut Context<Self, Self::Reply>) {
        if let Err(error) = self.apply(message.mode).await {
            eprintln!("chroma-daemon emacs theme concern error: {error}");
        }
    }
}

impl EmacsThemeConcern {
    async fn apply(&self, mode: ThemeMode) -> Result<()> {
        let Some(emacsclient) = &self.adapters.emacsclient else {
            return Ok(());
        };
        let theme = if mode == ThemeMode::Dark { "ignis-dark" } else { "ignis-light" };
        let expression = format!(
            "(progn (add-to-list 'custom-theme-load-path \"$HOME/.config/emacs-ignis-themes\") \
             (mapc #'disable-theme custom-enabled-themes) (load-theme '{theme} t))"
        );
        let mut child = tokio::process::Command::new(emacsclient)
            .args(["--eval", &expression])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        match timeout(Duration::from_secs(2), child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => {
                let _ = child.kill().await;
                Err(Error::Daemon { message: "emacsclient theme update timed out".into() })
            }
        }
    }
}

fn state_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from) {
        return Ok(path);
    }
    Ok(home_directory()?.join(".local/state"))
}

fn config_home() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Ok(path);
    }
    Ok(home_directory()?.join(".config"))
}

fn home_directory() -> Result<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| Error::Config { message: "HOME is not set".into() })
}
