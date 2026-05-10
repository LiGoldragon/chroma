//! [`Config`] — top-level chroma configuration.
//!
//! Chroma's configuration input is NOTA only. Removed theme apply
//! command records are rejected instead of migrated or interpreted.

use core::fmt;
use std::path::{Path, PathBuf};

use crate::brightness::BrightnessAxis;
use crate::error::{Error, Result};
use crate::theme::{
    ThemeAdapters, ThemeAxis, ThemeConcern, ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule, ThemeWaypoint,
};
use crate::time::{LocalHour, LocalMinute, RampTrigger, SignedMinutes};
use crate::warmth::WarmthAxis;
use nota_codec::{Lexer, Token};

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

    /// Locate Chroma's config using the normal user config search path.
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

    /// Extract the theme axis from the NOTA config.
    pub fn theme_axis(&self) -> Result<ThemeAxis> {
        let text = std::fs::read_to_string(&self.path)?;
        ConfigText::new(&text).theme_axis()
    }
}

struct ConfigText<'input> {
    text: &'input str,
}

impl<'input> ConfigText<'input> {
    fn new(text: &'input str) -> Self {
        Self { text }
    }

    fn theme_axis(&self) -> Result<ThemeAxis> {
        self.reject_removed_or_non_nota_inputs()?;
        Ok(ThemeAxis {
            concerns: self.theme_concerns()?,
            palettes: self.theme_palettes()?,
            adapters: self.theme_adapters()?,
            font_point_size: self.theme_font_point_size()?,
            schedule: self.theme_schedule()?,
        })
    }

    fn reject_removed_or_non_nota_inputs(&self) -> Result<()> {
        let mut lexer = Lexer::new(self.text);
        while let Some(token) = lexer.next_token()? {
            match token {
                Token::Ident(value) | Token::Str(value) => {
                    if matches!(value.as_str(), "ApplyCommand" | "ApplyTargets" | "ThemeApplyTarget" | "Legacy") {
                        return Err(Error::Config {
                            message: format!("{value} belongs to the removed shell-apply architecture"),
                        });
                    }
                    let lower = value.to_ascii_lowercase();
                    if lower.contains(".yaml") || lower.contains(".yml") {
                        return Err(Error::Config { message: "YAML inputs are forbidden; use NOTA".into() });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn theme_concerns(&self) -> Result<Vec<ThemeConcern>> {
        let mut record = RecordSearch::new(self.text);
        while record.seek("Concerns")? {
            let mut concerns = Vec::new();
            loop {
                match record.next_token()? {
                    Some(Token::RParen) if concerns.is_empty() => {
                        return Err(Error::Config { message: "Concerns must name at least one concern".into() });
                    }
                    Some(Token::RParen) => return Ok(concerns),
                    Some(Token::Ident(name)) | Some(Token::Str(name)) => {
                        concerns.push(ThemeConcern::from_config_name(&name)?);
                    }
                    Some(token) => {
                        return Err(Error::Config {
                            message: format!("Concerns expected concern names, got {token:?}"),
                        });
                    }
                    None => return Err(Error::Config { message: "Concerns ended before closing paren".into() }),
                }
            }
        }
        Err(Error::Config { message: "Theme config must contain a Concerns record".into() })
    }

    fn theme_palettes(&self) -> Result<ThemePalettes> {
        Ok(ThemePalettes { dark: self.theme_palette("Dark")?, light: self.theme_palette("Light")? })
    }

    fn theme_palette(&self, name: &str) -> Result<ThemePalette> {
        let mut record = RecordSearch::new(self.text);
        while record.seek(name)? {
            let mut slots: [Option<String>; 16] = std::array::from_fn(|_| None);
            loop {
                match record.next_token()? {
                    Some(Token::RParen) => return palette_from_slots(name, slots),
                    Some(Token::LParen) => {
                        let slot = match record.next_token()? {
                            Some(Token::Ident(slot)) => slot,
                            Some(token) => {
                                return Err(Error::Config {
                                    message: format!("{name} palette expected a BaseXX slot, got {token:?}"),
                                });
                            }
                            None => {
                                return Err(Error::Config {
                                    message: format!("{name} palette ended after opening a slot"),
                                });
                            }
                        };
                        let index = base16_slot_index(&slot)?;
                        let color = match record.next_token()? {
                            Some(Token::Str(color)) | Some(Token::Ident(color)) => color,
                            Some(token) => {
                                return Err(Error::Config {
                                    message: format!("{slot} expected a color string, got {token:?}"),
                                });
                            }
                            None => return Err(Error::Config { message: format!("{slot} ended before color") }),
                        };
                        record.expect_rparen(&format!("{slot} palette slot"))?;
                        slots[index] = Some(color);
                    }
                    Some(token) => {
                        return Err(Error::Config {
                            message: format!("{name} palette expected BaseXX records, got {token:?}"),
                        });
                    }
                    None => {
                        return Err(Error::Config { message: format!("{name} palette ended before closing paren") });
                    }
                }
            }
        }
        Err(Error::Config { message: format!("Theme config must contain a {name} palette record") })
    }

    fn theme_adapters(&self) -> Result<ThemeAdapters> {
        let mut record = RecordSearch::new(self.text);
        while record.seek("Adapters")? {
            let mut adapters = ThemeAdapters::default();
            loop {
                match record.next_token()? {
                    Some(Token::RParen) => return Ok(adapters),
                    Some(Token::LParen) => {
                        let adapter = match record.next_token()? {
                            Some(Token::Ident(adapter)) => adapter,
                            Some(token) => {
                                return Err(Error::Config {
                                    message: format!("Adapters expected adapter name, got {token:?}"),
                                });
                            }
                            None => {
                                return Err(Error::Config { message: "Adapters ended after opening record".into() });
                            }
                        };
                        let path = match record.next_token()? {
                            Some(Token::Str(path)) | Some(Token::Ident(path)) => PathBuf::from(path),
                            Some(token) => {
                                return Err(Error::Config {
                                    message: format!("{adapter} adapter expected path, got {token:?}"),
                                });
                            }
                            None => return Err(Error::Config { message: format!("{adapter} ended before path") }),
                        };
                        record.expect_rparen(&format!("{adapter} adapter"))?;
                        match adapter.as_str() {
                            "Dconf" => adapters.dconf = Some(path),
                            "Emacsclient" => adapters.emacsclient = Some(path),
                            _ => {
                                return Err(Error::Config { message: format!("unknown adapter {adapter}") });
                            }
                        }
                    }
                    Some(token) => {
                        return Err(Error::Config { message: format!("Adapters expected records, got {token:?}") });
                    }
                    None => return Err(Error::Config { message: "Adapters ended before closing paren".into() }),
                }
            }
        }
        Ok(ThemeAdapters::default())
    }

    fn theme_font_point_size(&self) -> Result<u8> {
        let mut record = RecordSearch::new(self.text);
        while record.seek("FontPointSize")? {
            let size = match record.next_token()? {
                Some(Token::Int(size)) if size > 0 && size <= u8::MAX as i128 => size as u8,
                Some(token) => {
                    return Err(Error::Config {
                        message: format!("FontPointSize expected a positive integer, got {token:?}"),
                    });
                }
                None => return Err(Error::Config { message: "FontPointSize ended before value".into() }),
            };
            record.expect_rparen("FontPointSize")?;
            return Ok(size);
        }
        Ok(12)
    }

    fn theme_schedule(&self) -> Result<ThemeSchedule> {
        let mut record = RecordSearch::new(self.text);
        while record.seek("Schedule")? {
            return record.parse_theme_schedule();
        }
        Err(Error::Config { message: "Theme config must contain a Schedule record".into() })
    }
}

struct RecordSearch<'input> {
    lexer: Lexer<'input>,
}

impl<'input> RecordSearch<'input> {
    fn new(text: &'input str) -> Self {
        Self { lexer: Lexer::new(text) }
    }

    fn seek(&mut self, name: &str) -> Result<bool> {
        while let Some(token) = self.lexer.next_token()? {
            if token != Token::LParen {
                continue;
            }
            match self.lexer.next_token()? {
                Some(Token::Ident(head)) if head == name => return Ok(true),
                Some(_) => {}
                None => return Err(Error::Config { message: format!("{name} search hit an incomplete record") }),
            }
        }
        Ok(false)
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        Ok(self.lexer.next_token()?)
    }

    fn expect_rparen(&mut self, label: &str) -> Result<()> {
        match self.lexer.next_token()? {
            Some(Token::RParen) => Ok(()),
            Some(token) => Err(Error::Config { message: format!("{label} expected closing paren, got {token:?}") }),
            None => Err(Error::Config { message: format!("{label} ended before closing paren") }),
        }
    }

    fn parse_theme_schedule(&mut self) -> Result<ThemeSchedule> {
        let mut waypoints = Vec::new();
        let mut default = ThemeMode::Dark;
        loop {
            match self.lexer.next_token()? {
                Some(Token::RParen) if waypoints.is_empty() => {
                    return Err(Error::Config { message: "Schedule must contain Manual or Waypoint records".into() });
                }
                Some(Token::RParen) => return Ok(ThemeSchedule::Scheduled { waypoints, default }),
                Some(Token::LParen) => match self.lexer.next_token()? {
                    Some(Token::Ident(head)) if head == "Manual" => {
                        let mode = self.parse_theme_mode("Manual")?;
                        self.expect_rparen("Manual")?;
                        self.expect_rparen("Schedule")?;
                        return Ok(ThemeSchedule::Manual(mode));
                    }
                    Some(Token::Ident(head)) if head == "Waypoint" => {
                        waypoints.push(self.parse_theme_waypoint()?);
                    }
                    Some(Token::Ident(head)) if head == "Default" => {
                        default = self.parse_theme_mode("Default")?;
                        self.expect_rparen("Default")?;
                    }
                    Some(Token::Ident(head)) => {
                        return Err(Error::Config { message: format!("unknown Schedule record {head}") });
                    }
                    Some(token) => {
                        return Err(Error::Config {
                            message: format!("Schedule expected a record head, got {token:?}"),
                        });
                    }
                    None => return Err(Error::Config { message: "Schedule ended after opening a record".into() }),
                },
                Some(token) => {
                    return Err(Error::Config { message: format!("Schedule expected records, got {token:?}") });
                }
                None => return Err(Error::Config { message: "Schedule ended before closing paren".into() }),
            }
        }
    }

    fn parse_theme_waypoint(&mut self) -> Result<ThemeWaypoint> {
        let trigger = self.parse_ramp_trigger()?;
        let mode = self.parse_theme_mode("Waypoint")?;
        self.expect_rparen("Waypoint")?;
        Ok(ThemeWaypoint { trigger, mode })
    }

    fn parse_ramp_trigger(&mut self) -> Result<RampTrigger> {
        match self.lexer.next_token()? {
            Some(Token::LParen) => {}
            Some(token) => {
                return Err(Error::Config { message: format!("Waypoint expected trigger record, got {token:?}") });
            }
            None => return Err(Error::Config { message: "Waypoint ended before trigger".into() }),
        }
        match self.lexer.next_token()? {
            Some(Token::Ident(head)) if head == "CivilDawn" => {
                let offset = self.parse_signed_minutes()?;
                self.expect_rparen("CivilDawn")?;
                Ok(RampTrigger::CivilDawn(offset))
            }
            Some(Token::Ident(head)) if head == "CivilDusk" => {
                let offset = self.parse_signed_minutes()?;
                self.expect_rparen("CivilDusk")?;
                Ok(RampTrigger::CivilDusk(offset))
            }
            Some(Token::Ident(head)) if head == "TimeOfDay" => {
                let hour = match self.lexer.next_token()? {
                    Some(Token::Int(hour)) if hour >= 0 => LocalHour::new(hour as u8),
                    Some(token) => {
                        return Err(Error::Config { message: format!("TimeOfDay expected hour, got {token:?}") });
                    }
                    None => return Err(Error::Config { message: "TimeOfDay ended before hour".into() }),
                };
                let minute = match self.lexer.next_token()? {
                    Some(Token::Int(minute)) if minute >= 0 => LocalMinute::new(minute as u8),
                    Some(token) => {
                        return Err(Error::Config { message: format!("TimeOfDay expected minute, got {token:?}") });
                    }
                    None => return Err(Error::Config { message: "TimeOfDay ended before minute".into() }),
                };
                self.expect_rparen("TimeOfDay")?;
                Ok(RampTrigger::TimeOfDay(hour, minute))
            }
            Some(Token::Ident(head)) => Err(Error::Config { message: format!("unknown trigger {head}") }),
            Some(token) => Err(Error::Config { message: format!("trigger expected record head, got {token:?}") }),
            None => Err(Error::Config { message: "trigger ended before head".into() }),
        }
    }

    fn parse_signed_minutes(&mut self) -> Result<SignedMinutes> {
        match self.lexer.next_token()? {
            Some(Token::LParen) => {}
            Some(token) => {
                return Err(Error::Config {
                    message: format!("civil trigger expected SignedMinutes record, got {token:?}"),
                });
            }
            None => return Err(Error::Config { message: "civil trigger ended before SignedMinutes".into() }),
        }
        match self.lexer.next_token()? {
            Some(Token::Ident(head)) if head == "SignedMinutes" => {}
            Some(token) => {
                return Err(Error::Config { message: format!("expected SignedMinutes, got {token:?}") });
            }
            None => return Err(Error::Config { message: "SignedMinutes ended before head".into() }),
        }
        let offset = match self.lexer.next_token()? {
            Some(Token::Int(offset)) => SignedMinutes::new(offset as i16),
            Some(token) => {
                return Err(Error::Config { message: format!("SignedMinutes expected integer, got {token:?}") });
            }
            None => return Err(Error::Config { message: "SignedMinutes ended before value".into() }),
        };
        self.expect_rparen("SignedMinutes")?;
        Ok(offset)
    }

    fn parse_theme_mode(&mut self, label: &str) -> Result<ThemeMode> {
        match self.lexer.next_token()? {
            Some(Token::Ident(mode)) if mode == "Dark" => Ok(ThemeMode::Dark),
            Some(Token::Ident(mode)) if mode == "Light" => Ok(ThemeMode::Light),
            Some(token) => Err(Error::Config { message: format!("{label} expected Dark or Light, got {token:?}") }),
            None => Err(Error::Config { message: format!("{label} ended before mode") }),
        }
    }
}

fn base16_slot_index(slot: &str) -> Result<usize> {
    match slot {
        "Base00" => Ok(0),
        "Base01" => Ok(1),
        "Base02" => Ok(2),
        "Base03" => Ok(3),
        "Base04" => Ok(4),
        "Base05" => Ok(5),
        "Base06" => Ok(6),
        "Base07" => Ok(7),
        "Base08" => Ok(8),
        "Base09" => Ok(9),
        "Base0A" => Ok(10),
        "Base0B" => Ok(11),
        "Base0C" => Ok(12),
        "Base0D" => Ok(13),
        "Base0E" => Ok(14),
        "Base0F" => Ok(15),
        _ => Err(Error::Config { message: format!("unknown base16 slot {slot}") }),
    }
}

fn palette_from_slots(name: &str, slots: [Option<String>; 16]) -> Result<ThemePalette> {
    let values: Vec<String> = slots
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value.ok_or_else(|| Error::Config { message: format!("{name} palette is missing Base{index:02X}") })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ThemePalette::from_base16_slots([
        &values[0],
        &values[1],
        &values[2],
        &values[3],
        &values[4],
        &values[5],
        &values[6],
        &values[7],
        &values[8],
        &values[9],
        &values[10],
        &values[11],
        &values[12],
        &values[13],
        &values[14],
        &values[15],
    ]))
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

impl fmt::Display for ConfigFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.path.display())
    }
}

impl AsRef<Path> for ConfigFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
