//! Tests for axis schedules and the top-level `Config`.

use chroma::{
    ApplyCommand, BrightnessAxis, BrightnessLevel, BrightnessSchedule, BrightnessWaypoint, Config, LocalHour,
    LocalMinute, RampDuration, RampTrigger, SignedMinutes, ThemeAxis, ThemeMode, ThemeSchedule, ThemeWaypoint,
    WarmthAxis, WarmthLevel, WarmthSchedule, WarmthWaypoint,
};
use std::path::PathBuf;

fn dawn(offset: i16) -> RampTrigger {
    RampTrigger::CivilDawn(SignedMinutes::new(offset))
}

fn dusk(offset: i16) -> RampTrigger {
    RampTrigger::CivilDusk(SignedMinutes::new(offset))
}

fn at(hour: u8, minute: u8) -> RampTrigger {
    RampTrigger::TimeOfDay(LocalHour::new(hour), LocalMinute::new(minute))
}

#[test]
fn theme_manual_default_is_the_value_itself() {
    let schedule = ThemeSchedule::Manual(ThemeMode::Light);
    assert_eq!(schedule.default_mode(), ThemeMode::Light);
}

#[test]
fn theme_scheduled_default_is_the_default_field() {
    let schedule = ThemeSchedule::Scheduled {
        waypoints: vec![ThemeWaypoint { trigger: at(20, 0), mode: ThemeMode::Dark }],
        default: ThemeMode::Light,
    };
    assert_eq!(schedule.default_mode(), ThemeMode::Light);
}

#[test]
fn theme_manual_does_not_need_geolocation() {
    assert!(!ThemeSchedule::Manual(ThemeMode::Dark).needs_geolocation());
}

#[test]
fn theme_scheduled_with_only_clock_triggers_does_not_need_geolocation() {
    let schedule = ThemeSchedule::Scheduled {
        waypoints: vec![
            ThemeWaypoint { trigger: at(7, 0), mode: ThemeMode::Light },
            ThemeWaypoint { trigger: at(20, 0), mode: ThemeMode::Dark },
        ],
        default: ThemeMode::Dark,
    };
    assert!(!schedule.needs_geolocation());
}

#[test]
fn theme_scheduled_with_civil_trigger_needs_geolocation() {
    let schedule = ThemeSchedule::Scheduled {
        waypoints: vec![
            ThemeWaypoint { trigger: dawn(0), mode: ThemeMode::Light },
            ThemeWaypoint { trigger: dusk(0), mode: ThemeMode::Dark },
        ],
        default: ThemeMode::Dark,
    };
    assert!(schedule.needs_geolocation());
}

#[test]
fn warmth_manual_does_not_need_geolocation() {
    assert!(!WarmthSchedule::Manual(WarmthLevel::Neutral).needs_geolocation());
}

#[test]
fn warmth_scheduled_with_civil_trigger_needs_geolocation() {
    let schedule = WarmthSchedule::Scheduled {
        waypoints: vec![WarmthWaypoint {
            trigger: dusk(-60),
            target: WarmthLevel::Warmest,
            ramp_duration: RampDuration::from_minutes(60),
        }],
        default: WarmthLevel::Neutral,
    };
    assert!(schedule.needs_geolocation());
}

#[test]
fn brightness_scheduled_with_clock_trigger_does_not_need_geolocation() {
    let schedule = BrightnessSchedule::Scheduled {
        waypoints: vec![BrightnessWaypoint {
            trigger: at(22, 0),
            target: BrightnessLevel::Dim,
            ramp_duration: RampDuration::from_minutes(30),
        }],
        default: BrightnessLevel::Bright,
    };
    assert!(!schedule.needs_geolocation());
}

#[test]
fn config_aggregates_axis_geolocation_need_disjunctively() {
    let theme_only_clock = ThemeAxis {
        apply_command: ApplyCommand::new("/run/current-system/sw/bin/chroma-apply-theme"),
        schedule: ThemeSchedule::Manual(ThemeMode::Light),
    };
    let warmth_with_civil = WarmthAxis {
        schedule: WarmthSchedule::Scheduled {
            waypoints: vec![WarmthWaypoint {
                trigger: dusk(-60),
                target: WarmthLevel::Warmest,
                ramp_duration: RampDuration::from_minutes(60),
            }],
            default: WarmthLevel::Neutral,
        },
    };
    let brightness_manual = BrightnessAxis { schedule: BrightnessSchedule::Manual(BrightnessLevel::Bright) };

    let config = Config { theme: theme_only_clock, warmth: warmth_with_civil, brightness: brightness_manual };
    assert!(config.needs_geolocation(), "warmth axis carries the civil trigger; whole config needs geo");
}

#[test]
fn config_without_civil_triggers_does_not_need_geolocation() {
    let config = Config {
        theme: ThemeAxis {
            apply_command: ApplyCommand::new("/run/current-system/sw/bin/chroma-apply-theme"),
            schedule: ThemeSchedule::Scheduled {
                waypoints: vec![
                    ThemeWaypoint { trigger: at(7, 0), mode: ThemeMode::Light },
                    ThemeWaypoint { trigger: at(20, 0), mode: ThemeMode::Dark },
                ],
                default: ThemeMode::Dark,
            },
        },
        warmth: WarmthAxis { schedule: WarmthSchedule::Manual(WarmthLevel::Neutral) },
        brightness: BrightnessAxis { schedule: BrightnessSchedule::Manual(BrightnessLevel::Bright) },
    };
    assert!(!config.needs_geolocation());
}

#[test]
fn apply_command_round_trips_through_pathbuf() {
    let original = PathBuf::from("/run/current-system/sw/bin/chroma-apply-theme");
    let command = ApplyCommand::new(original.clone());
    assert_eq!(command.as_path(), original.as_path());
    assert_eq!(format!("{}", command), original.display().to_string());
}
