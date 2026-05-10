//! `ThemeMode` — the desktop's colour-scheme axis.
//!
//! Theme switches are accepted instantly. The daemon fans the
//! requested mode out to independent native concern actors. There is
//! no shell-script apply boundary and no legacy apply-command schema.

use core::fmt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use nota_codec::NotaEnum;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::AbortHandle;
use tokio::time::{Duration, timeout};

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

    async fn apply(self, mode: ThemeMode, context: Arc<ThemeApplyContext>) -> Result<()> {
        let palette = context.palettes.for_mode(mode);
        match self {
            Self::Terminal => TerminalThemeConcern::new(mode, palette.clone()).apply().await,
            Self::Desktop => DesktopThemeConcern::new(mode, &context.adapters).apply().await,
            Self::Ghostty => GhosttyThemeConcern::new(palette.clone(), context.font_point_size).apply().await,
            Self::Emacs => EmacsThemeConcern::new(mode, &context.adapters).apply().await,
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
    pub schedule: ThemeSchedule,
}

#[derive(Clone)]
struct ThemeApplyContext {
    palettes: ThemePalettes,
    adapters: ThemeAdapters,
    font_point_size: u8,
}

/// Applies theme changes through independent native concern actors.
#[derive(Clone)]
pub struct ThemeApplier {
    actors: Vec<ThemeActor>,
}

impl ThemeApplier {
    pub fn from_axis(axis: ThemeAxis) -> Self {
        let context = Arc::new(ThemeApplyContext {
            palettes: axis.palettes,
            adapters: axis.adapters,
            font_point_size: axis.font_point_size,
        });
        Self {
            actors: axis.concerns.into_iter().map(|concern| ThemeActor::spawn(concern, Arc::clone(&context))).collect(),
        }
    }

    pub fn apply(&self, mode: ThemeMode) -> Result<()> {
        for actor in &self.actors {
            actor.apply(mode)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ThemeActor {
    concern: ThemeConcern,
    sender: UnboundedSender<ThemeMode>,
}

impl ThemeActor {
    fn spawn(concern: ThemeConcern, context: Arc<ThemeApplyContext>) -> Self {
        let (sender, receiver) = unbounded_channel();
        tokio::spawn(ThemeConcernActor::new(concern, context, receiver).run());
        Self { concern, sender }
    }

    fn apply(&self, mode: ThemeMode) -> Result<()> {
        self.sender.send(mode).map_err(|_| Error::Daemon { message: format!("{} theme actor is closed", self.concern) })
    }
}

struct ThemeConcernActor {
    concern: ThemeConcern,
    context: Arc<ThemeApplyContext>,
    receiver: UnboundedReceiver<ThemeMode>,
    active: Option<AbortHandle>,
}

impl ThemeConcernActor {
    fn new(concern: ThemeConcern, context: Arc<ThemeApplyContext>, receiver: UnboundedReceiver<ThemeMode>) -> Self {
        Self { concern, context, receiver, active: None }
    }

    async fn run(mut self) {
        while let Some(mut mode) = self.receiver.recv().await {
            while let Ok(next) = self.receiver.try_recv() {
                mode = next;
            }
            self.start(mode);
        }

        if let Some(handle) = self.active.take() {
            handle.abort();
        }
    }

    fn start(&mut self, mode: ThemeMode) {
        if let Some(handle) = self.active.take() {
            handle.abort();
        }

        let concern = self.concern;
        let context = Arc::clone(&self.context);
        let handle = tokio::spawn(async move {
            if let Err(error) = concern.apply(mode, context).await {
                eprintln!("chroma-daemon {concern} theme concern error: {error}");
            }
        });
        self.active = Some(handle.abort_handle());
    }
}

struct TerminalThemeConcern {
    mode: ThemeMode,
    palette: ThemePalette,
}

impl TerminalThemeConcern {
    fn new(mode: ThemeMode, palette: ThemePalette) -> Self {
        Self { mode, palette }
    }

    async fn apply(self) -> Result<()> {
        let state_dir = state_home()?.join("chroma");
        tokio::fs::create_dir_all(&state_dir).await?;
        tokio::fs::write(state_dir.join("current-mode"), format!("{}\n", self.mode)).await?;
        tokio::fs::write(
            state_dir.join("fzf-theme.sh"),
            format!("export FZF_DEFAULT_OPTS=\"$FZF_DEFAULT_OPTS {}\"\n", self.palette.fzf_options()),
        )
        .await?;
        Ok(())
    }
}

struct DesktopThemeConcern<'a> {
    mode: ThemeMode,
    adapters: &'a ThemeAdapters,
}

impl<'a> DesktopThemeConcern<'a> {
    fn new(mode: ThemeMode, adapters: &'a ThemeAdapters) -> Self {
        Self { mode, adapters }
    }

    async fn apply(self) -> Result<()> {
        if let Some(dconf) = &self.adapters.dconf {
            self.write_dconf(dconf).await?;
        }
        self.write_gtk_settings().await
    }

    async fn write_dconf(&self, dconf: &Path) -> Result<()> {
        let color_scheme = if self.mode == ThemeMode::Dark { "'prefer-dark'" } else { "'prefer-light'" };
        let gtk_theme = if self.mode == ThemeMode::Dark { "'adw-gtk3-dark'" } else { "'adw-gtk3'" };
        let icon_theme = if self.mode == ThemeMode::Dark { "'Papirus-Dark'" } else { "'Papirus-Light'" };
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

    async fn write_gtk_settings(&self) -> Result<()> {
        let config_home = config_home()?;
        let gtk_theme = if self.mode == ThemeMode::Dark { "adw-gtk3-dark" } else { "adw-gtk3" };
        let icon_theme = if self.mode == ThemeMode::Dark { "Papirus-Dark" } else { "Papirus-Light" };
        let prefer_dark = if self.mode == ThemeMode::Dark { "true" } else { "false" };
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
    palette: ThemePalette,
    font_point_size: u8,
}

impl GhosttyThemeConcern {
    fn new(palette: ThemePalette, font_point_size: u8) -> Self {
        Self { palette, font_point_size }
    }

    async fn apply(self) -> Result<()> {
        let directory = config_home()?.join("ghostty");
        tokio::fs::create_dir_all(&directory).await?;
        let config = format!(
            "font-family = IosevkaTerm Nerd Font\n\
             font-size = {}\n\
             window-decoration = false\n\
             gtk-titlebar = false\n\
             window-theme = ghostty\n\
             background = {}\n\
             foreground = {}\n",
            self.font_point_size, self.palette.base00, self.palette.base05
        );
        tokio::fs::write(directory.join("config"), config).await?;
        Ok(())
    }
}

struct EmacsThemeConcern<'a> {
    mode: ThemeMode,
    adapters: &'a ThemeAdapters,
}

impl<'a> EmacsThemeConcern<'a> {
    fn new(mode: ThemeMode, adapters: &'a ThemeAdapters) -> Self {
        Self { mode, adapters }
    }

    async fn apply(self) -> Result<()> {
        let Some(emacsclient) = &self.adapters.emacsclient else {
            return Ok(());
        };
        let theme = if self.mode == ThemeMode::Dark { "ignis-dark" } else { "ignis-light" };
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
