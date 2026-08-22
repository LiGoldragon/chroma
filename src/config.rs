//! [`Config`] — top-level chroma configuration.
//!
//! Chroma's configuration input is DOTOS only. Removed theme apply
//! command records are rejected instead of migrated or interpreted.

use core::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::brightness::{BrightnessAxis, BrightnessLevel, BrightnessSchedule, BrightnessWaypoint};
use crate::error::{Error, Result};
use crate::theme::{
    GhosttyConfigTemplates, PiThemeControl, PiThemeControlRegistryDirectory, ThemeAdapters, ThemeAxis, ThemeConcern,
    ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule, ThemeWaypoint,
};
use crate::time::{LocalHour, LocalMinute, RampDuration, RampTrigger, RelativeSolarOffset, SignedMinutes};
use crate::warmth::{WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint};
use dotos::{Block, Delimiter, Document, DotosBlock};

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
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).map(|path| path.join("chroma/config.dotos"))
        {
            return Ok(Self { path });
        }
        if let Some(path) = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/chroma/config.dotos"))
        {
            return Ok(Self { path });
        }
        Err(Error::Config { message: "neither CHROMA_CONFIG, XDG_CONFIG_HOME, nor HOME locates config.dotos".into() })
    }

    /// Extract the theme axis from the DOTOS config.
    pub fn theme_axis(&self) -> Result<ThemeAxis> {
        Self::decode_theme_axis(&std::fs::read_to_string(&self.path)?)
    }

    /// Decode the full Chroma config.
    pub fn config(&self) -> Result<Config> {
        Self::decode_config(&std::fs::read_to_string(&self.path)?)
    }

    /// Decode the full Chroma config without blocking the async runtime on file I/O.
    pub async fn config_async(&self) -> Result<Config> {
        Self::decode_config(&tokio::fs::read_to_string(&self.path).await?)
    }

    /// The concrete path watched by the daemon.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn decode_theme_axis(text: &str) -> Result<ThemeAxis> {
        ConfigText::new(text).theme_axis()
    }

    fn decode_config(text: &str) -> Result<Config> {
        ConfigText::new(text).config()
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
        Ok(self.config()?.theme)
    }

    fn config(&self) -> Result<Config> {
        let document = ConfigDocument::parse(self.text)?;
        document.reject_removed_or_non_dotos_inputs()?;
        document.config()
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

#[derive(Debug, Clone, PartialEq)]
enum ConfigNode {
    Record { head: String, body: Vec<ConfigNode> },
    Ident(String),
    Str(String),
    Int(i128),
}

impl ConfigNode {
    fn from_block(block: &Block) -> Result<Self> {
        if let Some(children) = block.as_delimited(Delimiter::Parenthesis) {
            return Self::from_parenthesis(children);
        }
        if block.is_square_bracket() || block.is_pipe_text() || block.as_application().is_some() {
            return Ok(Self::Str(DotosBlock::new(block).parse_string()?));
        }
        if let Some(value) = block.demote_to_string() {
            if let Ok(integer) = value.parse::<i128>() {
                return Ok(Self::Int(integer));
            }
            return Ok(Self::Ident(value.to_owned()));
        }
        Err(Error::Config { message: format!("unsupported block in Chroma config: {block:?}") })
    }

    fn from_parenthesis(children: &[Block]) -> Result<Self> {
        let Some(head_block) = children.first() else {
            return Err(Error::Config { message: "record ended before head".into() });
        };
        let head = Self::from_block(head_block)?.atom_name("record head")?.to_owned();
        let body = children[1..].iter().map(Self::from_block).collect::<Result<Vec<_>>>()?;
        Ok(Self::Record { head, body })
    }

    fn reject_removed_or_non_dotos_inputs(&self) -> Result<()> {
        match self {
            Self::Record { head, body } => {
                Self::reject_value(head)?;
                for node in body {
                    node.reject_removed_or_non_dotos_inputs()?;
                }
                Ok(())
            }
            Self::Ident(value) | Self::Str(value) => Self::reject_value(value),
            Self::Int(_) => Ok(()),
        }
    }

    fn reject_value(value: &str) -> Result<()> {
        if matches!(value, "ApplyCommand" | "ApplyTargets" | "ThemeApplyTarget" | "Legacy") {
            return Err(Error::Config { message: format!("{value} belongs to the removed shell-apply architecture") });
        }
        let lower = value.to_ascii_lowercase();
        if lower.contains(".yaml") || lower.contains(".yml") {
            return Err(Error::Config { message: "YAML inputs are forbidden; use DOTOS".into() });
        }
        Ok(())
    }

    fn atom_name(&self, label: &str) -> Result<&str> {
        match self {
            ConfigNode::Ident(value) | ConfigNode::Str(value) => Ok(value),
            other => Err(Error::Config { message: format!("{label} expected an identifier or string, got {other:?}") }),
        }
    }

    fn atom_int(&self, label: &str) -> Result<i128> {
        match self {
            ConfigNode::Int(value) => Ok(*value),
            other => Err(Error::Config { message: format!("{label} expected an integer, got {other:?}") }),
        }
    }

    fn head(&self) -> Option<&str> {
        match self {
            ConfigNode::Record { head, .. } => Some(head),
            _ => None,
        }
    }

    fn body(&self, label: &str) -> Result<&[ConfigNode]> {
        match self {
            ConfigNode::Record { body, .. } => Ok(body),
            other => Err(Error::Config { message: format!("{label} expected a record, got {other:?}") }),
        }
    }
}

struct ConfigDocument {
    roots: Vec<ConfigNode>,
}

impl ConfigDocument {
    fn parse(text: &str) -> Result<Self> {
        let document = Document::parse(text).map_err(dotos::DotosDecodeError::from)?;
        let roots = document.root_objects().iter().map(ConfigNode::from_block).collect::<Result<Vec<_>>>()?;
        Ok(Self { roots })
    }

    fn reject_removed_or_non_dotos_inputs(&self) -> Result<()> {
        for root in &self.roots {
            root.reject_removed_or_non_dotos_inputs()?;
        }
        Ok(())
    }

    fn config(&self) -> Result<Config> {
        let root = required_record(&self.roots, "Config")?;
        Ok(Config {
            theme: parse_theme_axis(required_node(root, "Theme")?.body("Theme")?)?,
            warmth: parse_warmth_axis(required_node(root, "Warmth")?.body("Warmth")?)?,
            brightness: parse_brightness_axis(required_node(root, "Brightness")?.body("Brightness")?)?,
        })
    }
}

fn parse_theme_axis(nodes: &[ConfigNode]) -> Result<ThemeAxis> {
    let concerns = parse_theme_concerns(required_record(nodes, "Concerns")?)?;
    let ghostty_config_templates = parse_ghostty_config_templates(optional_record(nodes, "GhosttyConfigTemplates")?)?;
    if concerns.contains(&ThemeConcern::Ghostty) && ghostty_config_templates.is_none() {
        return Err(Error::Config {
            message: "Ghostty concern requires a GhosttyConfigTemplates record with Dark and Light paths".into(),
        });
    }
    Ok(ThemeAxis {
        concerns,
        palettes: parse_theme_palettes(required_record(nodes, "Palettes")?)?,
        adapters: parse_theme_adapters(optional_record(nodes, "Adapters")?)?,
        font_point_size: parse_font_point_size(optional_record(nodes, "FontPointSize")?)?,
        ghostty_config_templates,
        pi_theme_control: parse_pi_theme_control(optional_record(nodes, "PiThemeControl")?)?,
        schedule: parse_theme_schedule_ast(required_record(nodes, "Schedule")?)?,
    })
}

fn parse_warmth_axis(nodes: &[ConfigNode]) -> Result<WarmthAxis> {
    Ok(WarmthAxis { schedule: parse_warmth_schedule(required_record(nodes, "Schedule")?)? })
}

fn parse_brightness_axis(nodes: &[ConfigNode]) -> Result<BrightnessAxis> {
    Ok(BrightnessAxis { schedule: parse_brightness_schedule(required_record(nodes, "Schedule")?)? })
}

fn parse_theme_concerns(nodes: &[ConfigNode]) -> Result<Vec<ThemeConcern>> {
    if nodes.is_empty() {
        return Err(Error::Config { message: "Concerns must name at least one concern".into() });
    }
    nodes.iter().map(|node| ThemeConcern::from_config_name(node.atom_name("Concerns")?)).collect()
}

fn parse_theme_palettes(nodes: &[ConfigNode]) -> Result<ThemePalettes> {
    Ok(ThemePalettes {
        dark: parse_theme_palette(required_record(nodes, "Dark")?, "Dark")?,
        light: parse_theme_palette(required_record(nodes, "Light")?, "Light")?,
    })
}

fn parse_theme_palette(nodes: &[ConfigNode], name: &str) -> Result<ThemePalette> {
    let mut slots: [Option<String>; 16] = std::array::from_fn(|_| None);
    for node in nodes {
        let ConfigNode::Record { head, body } = node else {
            return Err(Error::Config { message: format!("{name} palette expected BaseXX records, got {node:?}") });
        };
        let index = base16_slot_index(head)?;
        let color = required_atom(body, head)?.to_string();
        slots[index] = Some(color);
    }
    palette_from_slots(name, slots)
}

fn parse_theme_adapters(nodes: Option<&[ConfigNode]>) -> Result<ThemeAdapters> {
    let Some(nodes) = nodes else {
        return Ok(ThemeAdapters::default());
    };
    let mut adapters = ThemeAdapters::default();
    for node in nodes {
        let ConfigNode::Record { head, body } = node else {
            return Err(Error::Config { message: format!("Adapters expected records, got {node:?}") });
        };
        let path = PathBuf::from(required_atom(body, head)?);
        match head.as_str() {
            "Dconf" => adapters.dconf = Some(path),
            _ => return Err(Error::Config { message: format!("unknown adapter {head}") }),
        }
    }
    Ok(adapters)
}

fn parse_ghostty_config_templates(nodes: Option<&[ConfigNode]>) -> Result<Option<GhosttyConfigTemplates>> {
    let Some(nodes) = nodes else {
        return Ok(None);
    };
    Ok(Some(GhosttyConfigTemplates {
        dark: PathBuf::from(required_atom(required_record(nodes, "Dark")?, "Dark")?),
        light: PathBuf::from(required_atom(required_record(nodes, "Light")?, "Light")?),
    }))
}

fn parse_pi_theme_control(nodes: Option<&[ConfigNode]>) -> Result<Option<PiThemeControl>> {
    let Some(nodes) = nodes else {
        return Ok(None);
    };
    let registry_directory = parse_pi_theme_control_registry_directory(required_record(nodes, "RegistryDirectory")?)?;
    let connect_timeout = parse_optional_milliseconds(nodes, "ConnectTimeoutMillis", 100)?;
    let write_timeout = parse_optional_milliseconds(nodes, "WriteTimeoutMillis", 100)?;
    Ok(Some(PiThemeControl { registry_directory, connect_timeout, write_timeout }))
}

fn parse_pi_theme_control_registry_directory(nodes: &[ConfigNode]) -> Result<PiThemeControlRegistryDirectory> {
    if let Some(runtime_relative) = optional_record(nodes, "RuntimeRelative")? {
        return Ok(PiThemeControlRegistryDirectory::runtime_relative(PathBuf::from(required_atom(
            runtime_relative,
            "RuntimeRelative",
        )?)));
    }
    Ok(PiThemeControlRegistryDirectory::absolute(PathBuf::from(required_atom(nodes, "RegistryDirectory")?)))
}

fn parse_optional_milliseconds(nodes: &[ConfigNode], name: &str, default: u64) -> Result<Duration> {
    let Some(record) = optional_record(nodes, name)? else {
        return Ok(Duration::from_millis(default));
    };
    let value = required_int(record, name)?;
    if value > 0 && value <= u64::MAX as i128 {
        Ok(Duration::from_millis(value as u64))
    } else {
        Err(Error::Config { message: format!("{name} expected a positive u64 millisecond count, got {value}") })
    }
}

fn parse_font_point_size(nodes: Option<&[ConfigNode]>) -> Result<u8> {
    let Some(nodes) = nodes else {
        return Ok(12);
    };
    let value = required_int(nodes, "FontPointSize")?;
    if value > 0 && value <= u8::MAX as i128 {
        Ok(value as u8)
    } else {
        Err(Error::Config { message: format!("FontPointSize expected a positive u8, got {value}") })
    }
}

fn parse_theme_schedule_ast(nodes: &[ConfigNode]) -> Result<ThemeSchedule> {
    if let Some(manual) = optional_record(nodes, "Manual")? {
        return Ok(ThemeSchedule::Manual(parse_theme_mode_atom(required_atom(manual, "Manual")?, "Manual")?));
    }
    let mut waypoints = Vec::new();
    let mut default = ThemeMode::Dark;
    for node in nodes {
        let ConfigNode::Record { head, body } = node else {
            return Err(Error::Config { message: format!("Schedule expected records, got {node:?}") });
        };
        match head.as_str() {
            "Waypoint" => {
                let trigger = parse_trigger(
                    body.first().ok_or_else(|| Error::Config { message: "Waypoint ended before trigger".into() })?,
                )?;
                let mode = parse_theme_mode_atom(
                    body.get(1)
                        .ok_or_else(|| Error::Config { message: "Waypoint ended before mode".into() })?
                        .atom_name("Waypoint")?,
                    "Waypoint",
                )?;
                waypoints.push(ThemeWaypoint { trigger, mode });
            }
            "Default" => default = parse_theme_mode_atom(required_atom(body, "Default")?, "Default")?,
            "Manual" => {}
            other => return Err(Error::Config { message: format!("unknown Schedule record {other}") }),
        }
    }
    if waypoints.is_empty() {
        Err(Error::Config { message: "Schedule must contain Manual or Waypoint records".into() })
    } else {
        Ok(ThemeSchedule::Scheduled { waypoints, default })
    }
}

fn parse_warmth_schedule(nodes: &[ConfigNode]) -> Result<WarmthSchedule> {
    if let Some(manual) = optional_record(nodes, "Manual")? {
        return Ok(WarmthSchedule::Manual(parse_warmth_level(required_atom(manual, "Manual")?)?));
    }
    let mut waypoints = Vec::new();
    let mut default = WarmthLevel::Neutral;
    for node in nodes {
        let ConfigNode::Record { head, body } = node else {
            return Err(Error::Config { message: format!("Warmth Schedule expected records, got {node:?}") });
        };
        match head.as_str() {
            "Waypoint" => waypoints.push(WarmthWaypoint {
                trigger: parse_trigger(required_positional(body, 0, "warmth waypoint trigger")?)?,
                target: parse_warmth_level(required_atom(required_record(body, "Level")?, "Level")?)?,
                ramp_duration: parse_ramp_duration(required_record(body, "Ramp")?)?,
            }),
            "Default" => default = parse_warmth_level(required_atom(body, "Default")?)?,
            "Manual" => {}
            other => return Err(Error::Config { message: format!("unknown Warmth Schedule record {other}") }),
        }
    }
    if waypoints.is_empty() {
        Err(Error::Config { message: "Warmth Schedule must contain Manual or Waypoint records".into() })
    } else {
        Ok(WarmthSchedule::Scheduled { waypoints, default })
    }
}

fn parse_brightness_schedule(nodes: &[ConfigNode]) -> Result<BrightnessSchedule> {
    if let Some(manual) = optional_record(nodes, "Manual")? {
        return Ok(BrightnessSchedule::Manual(parse_brightness_level(required_atom(manual, "Manual")?)?));
    }
    let mut waypoints = Vec::new();
    let mut default = BrightnessLevel::Bright;
    for node in nodes {
        let ConfigNode::Record { head, body } = node else {
            return Err(Error::Config { message: format!("Brightness Schedule expected records, got {node:?}") });
        };
        match head.as_str() {
            "Waypoint" => waypoints.push(BrightnessWaypoint {
                trigger: parse_trigger(required_positional(body, 0, "brightness waypoint trigger")?)?,
                target: parse_brightness_level(required_atom(required_record(body, "Level")?, "Level")?)?,
                ramp_duration: parse_ramp_duration(required_record(body, "Ramp")?)?,
            }),
            "Default" => default = parse_brightness_level(required_atom(body, "Default")?)?,
            "Manual" => {}
            other => return Err(Error::Config { message: format!("unknown Brightness Schedule record {other}") }),
        }
    }
    if waypoints.is_empty() {
        Err(Error::Config { message: "Brightness Schedule must contain Manual or Waypoint records".into() })
    } else {
        Ok(BrightnessSchedule::Scheduled { waypoints, default })
    }
}

fn parse_trigger(node: &ConfigNode) -> Result<RampTrigger> {
    let ConfigNode::Record { head, body } = node else {
        return Err(Error::Config { message: format!("trigger expected record, got {node:?}") });
    };
    match head.as_str() {
        "Sunrise" => Ok(RampTrigger::Sunrise(parse_solar_offset(body, "Sunrise")?)),
        "Sunset" => Ok(RampTrigger::Sunset(parse_solar_offset(body, "Sunset")?)),
        "CivilDawn" => Ok(RampTrigger::CivilDawn(parse_solar_offset(body, "CivilDawn")?)),
        "CivilDusk" => Ok(RampTrigger::CivilDusk(parse_solar_offset(body, "CivilDusk")?)),
        "TimeOfDay" => {
            let hour = required_positional(body, 0, "TimeOfDay hour")?.atom_int("TimeOfDay hour")?;
            let minute = required_positional(body, 1, "TimeOfDay minute")?.atom_int("TimeOfDay minute")?;
            if hour < 0 || minute < 0 {
                return Err(Error::Config { message: "TimeOfDay values must be non-negative".into() });
            }
            Ok(RampTrigger::TimeOfDay(LocalHour::new(hour as u8), LocalMinute::new(minute as u8)))
        }
        other => Err(Error::Config { message: format!("unknown trigger {other}") }),
    }
}

fn parse_solar_offset(nodes: &[ConfigNode], label: &str) -> Result<SignedMinutes> {
    if nodes.is_empty() {
        return Ok(SignedMinutes::ZERO);
    }
    if let Some(signed_minutes) = optional_record(nodes, "SignedMinutes")? {
        return parse_signed_minutes(signed_minutes);
    }
    if nodes.len() == 1 {
        let name = required_atom(nodes, label)?;
        return Ok(RelativeSolarOffset::from_config_name(name)?.signed_minutes());
    }
    Err(Error::Config { message: format!("{label} expected SignedMinutes or a relative solar offset label") })
}

fn parse_signed_minutes(nodes: &[ConfigNode]) -> Result<SignedMinutes> {
    let value = required_int(nodes, "SignedMinutes")?;
    Ok(SignedMinutes::new(value as i16))
}

fn parse_ramp_duration(nodes: &[ConfigNode]) -> Result<RampDuration> {
    if let Some(minutes) = optional_record(nodes, "Minutes")? {
        let value = required_int(minutes, "Minutes")?;
        if value < 0 {
            return Err(Error::Config { message: "Ramp Minutes must be non-negative".into() });
        }
        return Ok(RampDuration::from_minutes(value as u32));
    }
    if let Some(seconds) = optional_record(nodes, "Seconds")? {
        let value = required_int(seconds, "Seconds")?;
        if value < 0 {
            return Err(Error::Config { message: "Ramp Seconds must be non-negative".into() });
        }
        return Ok(RampDuration::from_seconds(value as u64));
    }
    Err(Error::Config { message: "Ramp expected Minutes or Seconds".into() })
}

fn parse_theme_mode_atom(value: &str, label: &str) -> Result<ThemeMode> {
    match value {
        "Dark" => Ok(ThemeMode::Dark),
        "Light" => Ok(ThemeMode::Light),
        _ => Err(Error::Config { message: format!("{label} expected Dark or Light, got {value:?}") }),
    }
}

fn parse_warmth_level(value: &str) -> Result<WarmthLevel> {
    match value {
        "Cold" => Ok(WarmthLevel::Cold),
        "Cool" => Ok(WarmthLevel::Cool),
        "Neutral" => Ok(WarmthLevel::Neutral),
        "Warm" => Ok(WarmthLevel::Warm),
        "Warmer" => Ok(WarmthLevel::Warmer),
        "Warmest" => Ok(WarmthLevel::Warmest),
        _ => Err(Error::Config { message: format!("unknown warmth level {value:?}") }),
    }
}

fn parse_brightness_level(value: &str) -> Result<BrightnessLevel> {
    match value {
        "Dim" => Ok(BrightnessLevel::Dim),
        "Dimmer" => Ok(BrightnessLevel::Dimmer),
        "Mid" => Ok(BrightnessLevel::Mid),
        "Bright" => Ok(BrightnessLevel::Bright),
        "Brighter" => Ok(BrightnessLevel::Brighter),
        "Brightest" => Ok(BrightnessLevel::Brightest),
        _ => Err(Error::Config { message: format!("unknown brightness level {value:?}") }),
    }
}

fn required_node<'a>(nodes: &'a [ConfigNode], head: &str) -> Result<&'a ConfigNode> {
    nodes
        .iter()
        .find(|node| node.head() == Some(head))
        .ok_or_else(|| Error::Config { message: format!("Config must contain a {head} record") })
}

fn required_record<'a>(nodes: &'a [ConfigNode], head: &str) -> Result<&'a [ConfigNode]> {
    required_node(nodes, head)?.body(head)
}

fn optional_record<'a>(nodes: &'a [ConfigNode], head: &str) -> Result<Option<&'a [ConfigNode]>> {
    nodes.iter().find(|node| node.head() == Some(head)).map(|node| node.body(head)).transpose()
}

fn required_positional<'a>(nodes: &'a [ConfigNode], index: usize, label: &str) -> Result<&'a ConfigNode> {
    nodes.get(index).ok_or_else(|| Error::Config { message: format!("{label} is missing") })
}

fn required_atom<'a>(nodes: &'a [ConfigNode], label: &str) -> Result<&'a str> {
    required_positional(nodes, 0, label)?.atom_name(label)
}

fn required_int(nodes: &[ConfigNode], label: &str) -> Result<i128> {
    required_positional(nodes, 0, label)?.atom_int(label)
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
