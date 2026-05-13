//! Tests for axis schedules and the top-level `Config`.

use chroma::{
    BrightnessAxis, BrightnessLevel, BrightnessSchedule, BrightnessWaypoint, Config, LocalHour, LocalMinute, Location,
    RampDuration, RampTrigger, SchedulePlan, ScheduledBrightness, ScheduledWarmth, SignedMinutes, ThemeAdapters,
    ThemeAxis, ThemeConcern, ThemeMode, ThemePalette, ThemePalettes, ThemeSchedule, ThemeWaypoint, WarmthAxis,
    WarmthLevel, WarmthSchedule, WarmthWaypoint,
};
use chrono::{Local, TimeZone};

fn dawn(offset: i16) -> RampTrigger {
    RampTrigger::CivilDawn(SignedMinutes::new(offset))
}

fn dusk(offset: i16) -> RampTrigger {
    RampTrigger::CivilDusk(SignedMinutes::new(offset))
}

fn at(hour: u8, minute: u8) -> RampTrigger {
    RampTrigger::TimeOfDay(LocalHour::new(hour), LocalMinute::new(minute))
}

fn test_palettes() -> ThemePalettes {
    let palette = ThemePalette::from_base16_slots([
        "#000000", "#111111", "#222222", "#333333", "#444444", "#555555", "#666666", "#777777", "#888888", "#999999",
        "#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd", "#eeeeee", "#ffffff",
    ]);
    ThemePalettes { dark: palette.clone(), light: palette }
}

fn test_theme_axis(schedule: ThemeSchedule) -> ThemeAxis {
    ThemeAxis {
        concerns: vec![ThemeConcern::Terminal],
        palettes: test_palettes(),
        adapters: ThemeAdapters::default(),
        font_point_size: 12,
        ghostty_config_templates: None,
        schedule,
    }
}

fn local_time(hour: u32, minute: u32) -> chrono::DateTime<Local> {
    Local.with_ymd_and_hms(2026, 5, 12, hour, minute, 0).single().expect("test local time is valid")
}

fn config_with_schedules(warmth: WarmthSchedule, brightness: BrightnessSchedule) -> Config {
    Config {
        theme: test_theme_axis(ThemeSchedule::Manual(ThemeMode::Light)),
        warmth: WarmthAxis { schedule: warmth },
        brightness: BrightnessAxis { schedule: brightness },
    }
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
    let theme_only_clock = test_theme_axis(ThemeSchedule::Manual(ThemeMode::Light));
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
        theme: test_theme_axis(ThemeSchedule::Scheduled {
            waypoints: vec![
                ThemeWaypoint { trigger: at(7, 0), mode: ThemeMode::Light },
                ThemeWaypoint { trigger: at(20, 0), mode: ThemeMode::Dark },
            ],
            default: ThemeMode::Dark,
        }),
        warmth: WarmthAxis { schedule: WarmthSchedule::Manual(WarmthLevel::Neutral) },
        brightness: BrightnessAxis { schedule: BrightnessSchedule::Manual(BrightnessLevel::Bright) },
    };
    assert!(!config.needs_geolocation());
}

#[test]
fn civil_warmth_axis_waits_without_location_instead_of_applying_default() {
    let config = config_with_schedules(
        WarmthSchedule::Scheduled {
            waypoints: vec![
                WarmthWaypoint {
                    trigger: dawn(-30),
                    target: WarmthLevel::Cold,
                    ramp_duration: RampDuration::from_minutes(30),
                },
                WarmthWaypoint {
                    trigger: dusk(-60),
                    target: WarmthLevel::Warmest,
                    ramp_duration: RampDuration::from_minutes(60),
                },
            ],
            default: WarmthLevel::Neutral,
        },
        BrightnessSchedule::Manual(BrightnessLevel::Bright),
    );

    let values = SchedulePlan::from_config(&config, None, local_time(12, 0)).values();

    assert!(values.warmth().is_none());
    assert_eq!(
        values.brightness().unwrap(),
        ScheduledBrightness::Settled { percent: BrightnessLevel::Bright.percent() }
    );
}

#[test]
fn civil_warmth_axis_applies_once_location_is_known() {
    let config = config_with_schedules(
        WarmthSchedule::Scheduled {
            waypoints: vec![WarmthWaypoint {
                trigger: dawn(0),
                target: WarmthLevel::Cold,
                ramp_duration: RampDuration::from_minutes(30),
            }],
            default: WarmthLevel::Neutral,
        },
        BrightnessSchedule::Manual(BrightnessLevel::Bright),
    );
    let tirana = Location { latitude: 41.3275, longitude: 19.8189 };

    let values = SchedulePlan::from_config(&config, Some(tirana), local_time(12, 0)).values();

    assert!(values.warmth().is_some());
}

#[test]
fn warmth_schedule_after_finished_ramp_is_settled_target() {
    let config = config_with_schedules(
        WarmthSchedule::Scheduled {
            waypoints: vec![
                WarmthWaypoint {
                    trigger: at(5, 0),
                    target: WarmthLevel::Warmest,
                    ramp_duration: RampDuration::from_seconds(1),
                },
                WarmthWaypoint {
                    trigger: at(6, 0),
                    target: WarmthLevel::Cold,
                    ramp_duration: RampDuration::from_minutes(30),
                },
            ],
            default: WarmthLevel::Warmest,
        },
        BrightnessSchedule::Manual(BrightnessLevel::Bright),
    );

    let values = SchedulePlan::from_config(&config, None, local_time(10, 30)).values();

    assert_eq!(values.warmth().unwrap(), ScheduledWarmth::Settled { kelvin: WarmthLevel::Cold.kelvin() });
}

#[test]
fn warmth_schedule_inside_ramp_projects_elapsed_position() {
    let config = config_with_schedules(
        WarmthSchedule::Scheduled {
            waypoints: vec![
                WarmthWaypoint {
                    trigger: at(5, 0),
                    target: WarmthLevel::Warmest,
                    ramp_duration: RampDuration::from_seconds(1),
                },
                WarmthWaypoint {
                    trigger: at(6, 0),
                    target: WarmthLevel::Cold,
                    ramp_duration: RampDuration::from_minutes(30),
                },
            ],
            default: WarmthLevel::Warmest,
        },
        BrightnessSchedule::Manual(BrightnessLevel::Bright),
    );
    let expected_current = WarmthLevel::Warmest.kelvin().lerp_to(WarmthLevel::Cold.kelvin(), 0.5);

    let values = SchedulePlan::from_config(&config, None, local_time(6, 15)).values();

    assert_eq!(
        values.warmth().unwrap(),
        ScheduledWarmth::Transition {
            current_kelvin: expected_current,
            target_kelvin: WarmthLevel::Cold.kelvin(),
            remaining_duration: RampDuration::from_minutes(15),
        }
    );
}

#[test]
fn brightness_schedule_inside_ramp_projects_elapsed_position() {
    let config = config_with_schedules(
        WarmthSchedule::Manual(WarmthLevel::Cold),
        BrightnessSchedule::Scheduled {
            waypoints: vec![
                BrightnessWaypoint {
                    trigger: at(9, 0),
                    target: BrightnessLevel::Dim,
                    ramp_duration: RampDuration::from_seconds(1),
                },
                BrightnessWaypoint {
                    trigger: at(10, 0),
                    target: BrightnessLevel::Bright,
                    ramp_duration: RampDuration::from_minutes(60),
                },
            ],
            default: BrightnessLevel::Dim,
        },
    );
    let expected_current = BrightnessLevel::Dim.percent().lerp_to(BrightnessLevel::Bright.percent(), 0.5);

    let values = SchedulePlan::from_config(&config, None, local_time(10, 30)).values();

    assert_eq!(
        values.brightness().unwrap(),
        ScheduledBrightness::Transition {
            current_percent: expected_current,
            target_percent: BrightnessLevel::Bright.percent(),
            remaining_duration: RampDuration::from_minutes(30),
        }
    );
}
