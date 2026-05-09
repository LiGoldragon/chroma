//! [`Config`] — top-level chroma configuration.
//!
//! Parsed from a single NOTA record at
//! `~/.config/chroma/config.nota`. Re-parsed on inotify push.

use core::fmt;
use std::path::{Path, PathBuf};

use crate::brightness::BrightnessAxis;
use crate::error::{Error, Result};
use crate::theme::ThemeAxis;
use crate::warmth::WarmthAxis;
use nota_codec::{Lexer, Token};

/// Path to the home-manager-built shell script that applies a
/// theme. The daemon spawns it with one positional argument
/// (`dark` or `light`) and waits for exit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApplyCommand(PathBuf);

impl ApplyCommand {
    /// Construct from any path-like value.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Borrow as a [`Path`] for spawning.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for ApplyCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}

/// The on-disk Chroma configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    /// Construct from an explicit path, primarily for tests.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Locate Chroma's config using the normal user config
    /// search path.
    pub fn from_default_locations() -> Result<Self> {
        if let Some(path) = std::env::var_os("CHROMA_CONFIG").map(PathBuf::from) {
            return Ok(Self { path });
        }
        if let Some(path) =
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).map(|path| path.join("chroma/config.nota"))
        {
            return Ok(Self { path });
        }
        if let Some(path) = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/chroma/config.nota"))
        {
            return Ok(Self { path });
        }
        Err(Error::Config { message: "neither CHROMA_CONFIG, XDG_CONFIG_HOME, nor HOME locates config.nota".into() })
    }

    /// Extract the theme apply command from the config file.
    pub fn theme_apply_command(&self) -> Result<ApplyCommand> {
        let text = std::fs::read_to_string(&self.path)?;
        ConfigText::new(&text).theme_apply_command()
    }
}

struct ConfigText<'input> {
    text: &'input str,
}

impl<'input> ConfigText<'input> {
    fn new(text: &'input str) -> Self {
        Self { text }
    }

    fn theme_apply_command(&self) -> Result<ApplyCommand> {
        ApplyCommandRecord::from_text(self.text).apply_command()
    }
}

struct ApplyCommandRecord<'input> {
    lexer: Lexer<'input>,
}

impl<'input> ApplyCommandRecord<'input> {
    fn from_text(text: &'input str) -> Self {
        Self { lexer: Lexer::new(text) }
    }

    fn apply_command(mut self) -> Result<ApplyCommand> {
        while let Some(token) = self.lexer.next_token()? {
            if token != Token::LParen {
                continue;
            }

            let Some(Token::Ident(head)) = self.lexer.next_token()? else {
                continue;
            };

            if head != "ApplyCommand" {
                continue;
            }

            let path = match self.lexer.next_token()? {
                Some(Token::Str(path)) | Some(Token::Ident(path)) => path,
                Some(token) => {
                    return Err(Error::Config {
                        message: format!("ApplyCommand expected a path string, got {token:?}"),
                    });
                }
                None => {
                    return Err(Error::Config { message: "ApplyCommand ended before its path".into() });
                }
            };

            match self.lexer.next_token()? {
                Some(Token::RParen) => return Ok(ApplyCommand::new(path)),
                Some(token) => {
                    return Err(Error::Config {
                        message: format!("ApplyCommand expected closing paren, got {token:?}"),
                    });
                }
                None => {
                    return Err(Error::Config { message: "ApplyCommand ended before closing paren".into() });
                }
            }
        }

        Err(Error::Config { message: "Config does not contain an ApplyCommand record".into() })
    }
}

/// Top-level chroma configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeAxis,
    pub warmth: WarmthAxis,
    pub brightness: BrightnessAxis,
}

impl Config {
    /// Whether any axis schedule requires the geoclue subscription.
    pub fn needs_geolocation(&self) -> bool {
        self.theme.schedule.needs_geolocation()
            || self.warmth.schedule.needs_geolocation()
            || self.brightness.schedule.needs_geolocation()
    }
}
