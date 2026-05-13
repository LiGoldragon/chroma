//! Wall-clock schedule projection for the visual axes.
//!
//! A schedule reconciliation answers "what should be true now?",
//! not "which waypoint most recently fired?". Ramped axes therefore
//! project elapsed wall-clock time into the current transition: before
//! a ramp they hold the previous value, during a ramp they produce the
//! interpolated value plus the remaining duration, and after a ramp they
//! produce the target as a settled value.

use std::time::Duration;

use chrono::{DateTime, Local, LocalResult, NaiveDate, TimeZone};
use sunrise::{Coordinates, DawnType, SolarDay, SolarEvent};

use crate::brightness::{BrightnessPercent, BrightnessSchedule};
use crate::config::Config;
use crate::state::StoredLocation;
use crate::theme::{ThemeMode, ThemeSchedule};
use crate::time::{LocalHour, LocalMinute, RampDuration, RampTrigger, SignedMinutes};
use crate::warmth::{KelvinTemperature, WarmthSchedule};

pub type Location = StoredLocation;

#[derive(Debug, Clone, Copy)]
pub struct ScheduledValues {
    pub(crate) theme: Option<ThemeMode>,
    pub(crate) warmth: Option<ScheduledWarmth>,
    pub(crate) brightness: Option<ScheduledBrightness>,
}

impl ScheduledValues {
    pub fn theme(self) -> Option<ThemeMode> {
        self.theme
    }

    pub fn warmth(self) -> Option<ScheduledWarmth> {
        self.warmth
    }

    pub fn brightness(self) -> Option<ScheduledBrightness> {
        self.brightness
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledWarmth {
    Settled { kelvin: KelvinTemperature },
    Transition { current_kelvin: KelvinTemperature, target_kelvin: KelvinTemperature, remaining_duration: RampDuration },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledBrightness {
    Settled {
        percent: BrightnessPercent,
    },
    Transition {
        current_percent: BrightnessPercent,
        target_percent: BrightnessPercent,
        remaining_duration: RampDuration,
    },
}

pub struct SchedulePlan {
    values: ScheduledValues,
    next_at: Option<DateTime<Local>>,
}

impl SchedulePlan {
    pub fn from_config(config: &Config, location: Option<Location>, now: DateTime<Local>) -> Self {
        let theme = ThemeProjection::from_schedule(&config.theme.schedule, location).axis_schedule_at(now);
        let warmth = WarmthProjection::from_schedule(&config.warmth.schedule, location).axis_schedule_at(now);
        let brightness =
            BrightnessProjection::from_schedule(&config.brightness.schedule, location).axis_schedule_at(now);
        Self {
            values: ScheduledValues { theme: theme.value, warmth: warmth.value, brightness: brightness.value },
            next_at: [theme.next_at, warmth.next_at, brightness.next_at].into_iter().flatten().min(),
        }
    }

    pub fn values(&self) -> ScheduledValues {
        self.values
    }

    pub fn next_delay_from(&self, now: DateTime<Local>) -> Option<Duration> {
        let next = self.next_at?;
        next.signed_duration_since(now).to_std().ok().map(|duration| duration.max(Duration::from_secs(1)))
    }
}

struct AxisSchedule<T> {
    value: Option<T>,
    next_at: Option<DateTime<Local>>,
}

impl<T> AxisSchedule<T> {
    fn ready(value: T, next_at: Option<DateTime<Local>>) -> Self {
        Self { value: Some(value), next_at }
    }

    fn waiting() -> Self {
        Self { value: None, next_at: None }
    }
}

struct ThemeProjection {
    schedule: ThemeSchedule,
    location: Option<Location>,
}

impl ThemeProjection {
    fn from_schedule(schedule: &ThemeSchedule, location: Option<Location>) -> Self {
        Self { schedule: schedule.clone(), location }
    }

    fn axis_schedule_at(&self, now: DateTime<Local>) -> AxisSchedule<ThemeMode> {
        match &self.schedule {
            ThemeSchedule::Manual(mode) => AxisSchedule::ready(*mode, None),
            ThemeSchedule::Scheduled { waypoints, default } => {
                if self.schedule.needs_geolocation() && self.location.is_none() {
                    return AxisSchedule::waiting();
                }
                let events = waypoints.iter().flat_map(|waypoint| {
                    trigger_datetimes(waypoint.trigger, self.location, now)
                        .map(move |time| TimedEvent { time, value: waypoint.mode })
                });
                TimedTimeline::new(*default, events).axis_schedule_at(now)
            }
        }
    }
}

struct WarmthProjection {
    schedule: WarmthSchedule,
    location: Option<Location>,
}

impl WarmthProjection {
    fn from_schedule(schedule: &WarmthSchedule, location: Option<Location>) -> Self {
        Self { schedule: schedule.clone(), location }
    }

    fn axis_schedule_at(&self, now: DateTime<Local>) -> AxisSchedule<ScheduledWarmth> {
        match &self.schedule {
            WarmthSchedule::Manual(level) => {
                AxisSchedule::ready(ScheduledWarmth::Settled { kelvin: level.kelvin() }, None)
            }
            WarmthSchedule::Scheduled { waypoints, default } => {
                if self.schedule.needs_geolocation() && self.location.is_none() {
                    return AxisSchedule::waiting();
                }
                let events = waypoints.iter().flat_map(|waypoint| {
                    trigger_datetimes(waypoint.trigger, self.location, now).map(move |time| RampedEvent {
                        time,
                        target: waypoint.target.kelvin(),
                        duration: waypoint.ramp_duration,
                    })
                });
                let schedule = RampedTimeline::new(default.kelvin(), events).axis_schedule_at(now);
                AxisSchedule { value: schedule.value.map(ScheduledWarmth::from), next_at: schedule.next_at }
            }
        }
    }
}

struct BrightnessProjection {
    schedule: BrightnessSchedule,
    location: Option<Location>,
}

impl BrightnessProjection {
    fn from_schedule(schedule: &BrightnessSchedule, location: Option<Location>) -> Self {
        Self { schedule: schedule.clone(), location }
    }

    fn axis_schedule_at(&self, now: DateTime<Local>) -> AxisSchedule<ScheduledBrightness> {
        match &self.schedule {
            BrightnessSchedule::Manual(level) => {
                AxisSchedule::ready(ScheduledBrightness::Settled { percent: level.percent() }, None)
            }
            BrightnessSchedule::Scheduled { waypoints, default } => {
                if self.schedule.needs_geolocation() && self.location.is_none() {
                    return AxisSchedule::waiting();
                }
                let events = waypoints.iter().flat_map(|waypoint| {
                    trigger_datetimes(waypoint.trigger, self.location, now).map(move |time| RampedEvent {
                        time,
                        target: waypoint.target.percent(),
                        duration: waypoint.ramp_duration,
                    })
                });
                let schedule = RampedTimeline::new(default.percent(), events).axis_schedule_at(now);
                AxisSchedule { value: schedule.value.map(ScheduledBrightness::from), next_at: schedule.next_at }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TimedEvent<Value> {
    time: DateTime<Local>,
    value: Value,
}

struct TimedTimeline<Value> {
    default: Value,
    events: Vec<TimedEvent<Value>>,
}

impl<Value> TimedTimeline<Value>
where
    Value: Copy,
{
    fn new(default: Value, events: impl IntoIterator<Item = TimedEvent<Value>>) -> Self {
        let mut events = events.into_iter().collect::<Vec<_>>();
        events.sort_by(|left, right| left.time.cmp(&right.time));
        Self { default, events }
    }

    fn axis_schedule_at(&self, now: DateTime<Local>) -> AxisSchedule<Value> {
        let next_at = self.events.iter().find(|event| event.time > now).map(|event| event.time);
        let value =
            self.events.iter().rev().find(|event| event.time <= now).map(|event| event.value).unwrap_or(self.default);
        AxisSchedule::ready(value, next_at)
    }
}

#[derive(Debug, Clone, Copy)]
struct RampedEvent<Value> {
    time: DateTime<Local>,
    target: Value,
    duration: RampDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RampedScheduleState<Value> {
    Settled { value: Value },
    Transition { current: Value, target: Value, remaining_duration: RampDuration },
}

struct RampedTimeline<Value> {
    default: Value,
    events: Vec<RampedEvent<Value>>,
}

impl<Value> RampedTimeline<Value>
where
    Value: Copy + PartialEq + LinearInterpolate,
{
    fn new(default: Value, events: impl IntoIterator<Item = RampedEvent<Value>>) -> Self {
        let mut events = events.into_iter().collect::<Vec<_>>();
        events.sort_by(|left, right| left.time.cmp(&right.time));
        Self { default, events }
    }

    fn axis_schedule_at(&self, now: DateTime<Local>) -> AxisSchedule<RampedScheduleState<Value>> {
        let next_at = self.events.iter().find(|event| event.time > now).map(|event| event.time);
        let Some((index, event)) = self.events.iter().enumerate().rev().find(|(_, event)| event.time <= now) else {
            return AxisSchedule::ready(RampedScheduleState::Settled { value: self.default }, next_at);
        };
        let source = if index == 0 { self.default } else { self.events[index - 1].target };
        let value = self.project_event(source, *event, now);
        AxisSchedule::ready(value, next_at)
    }

    fn project_event(
        &self,
        source: Value,
        event: RampedEvent<Value>,
        now: DateTime<Local>,
    ) -> RampedScheduleState<Value> {
        if source == event.target {
            return RampedScheduleState::Settled { value: event.target };
        }
        let Some(end) = event.end_time() else {
            return RampedScheduleState::Settled { value: event.target };
        };
        if now >= end {
            return RampedScheduleState::Settled { value: event.target };
        }
        let Some(remaining_duration) = event.remaining_duration_at(now) else {
            return RampedScheduleState::Settled { value: event.target };
        };
        let elapsed = now.signed_duration_since(event.time).to_std().unwrap_or(Duration::ZERO);
        let fraction = elapsed.as_secs_f64() / event.duration.as_duration().as_secs_f64();
        let current = source.interpolate_toward(event.target, fraction);
        if current == event.target {
            RampedScheduleState::Settled { value: event.target }
        } else {
            RampedScheduleState::Transition { current, target: event.target, remaining_duration }
        }
    }
}

impl<Value> RampedEvent<Value> {
    fn end_time(&self) -> Option<DateTime<Local>> {
        let duration = chrono::Duration::from_std(self.duration.as_duration()).ok()?;
        Some(self.time + duration)
    }

    fn remaining_duration_at(&self, now: DateTime<Local>) -> Option<RampDuration> {
        let remaining = self.end_time()?.signed_duration_since(now).to_std().ok()?;
        if remaining.is_zero() { None } else { Some(RampDuration::from_seconds(ceil_seconds(remaining))) }
    }
}

trait LinearInterpolate {
    fn interpolate_toward(self, target: Self, fraction: f64) -> Self;
}

impl LinearInterpolate for KelvinTemperature {
    fn interpolate_toward(self, target: Self, fraction: f64) -> Self {
        self.lerp_to(target, fraction)
    }
}

impl LinearInterpolate for BrightnessPercent {
    fn interpolate_toward(self, target: Self, fraction: f64) -> Self {
        self.lerp_to(target, fraction)
    }
}

impl From<RampedScheduleState<KelvinTemperature>> for ScheduledWarmth {
    fn from(value: RampedScheduleState<KelvinTemperature>) -> Self {
        match value {
            RampedScheduleState::Settled { value } => Self::Settled { kelvin: value },
            RampedScheduleState::Transition { current, target, remaining_duration } => {
                Self::Transition { current_kelvin: current, target_kelvin: target, remaining_duration }
            }
        }
    }
}

impl From<RampedScheduleState<BrightnessPercent>> for ScheduledBrightness {
    fn from(value: RampedScheduleState<BrightnessPercent>) -> Self {
        match value {
            RampedScheduleState::Settled { value } => Self::Settled { percent: value },
            RampedScheduleState::Transition { current, target, remaining_duration } => {
                Self::Transition { current_percent: current, target_percent: target, remaining_duration }
            }
        }
    }
}

fn ceil_seconds(duration: Duration) -> u64 {
    duration.as_secs() + u64::from(duration.subsec_nanos() > 0)
}

fn trigger_datetimes(
    trigger: RampTrigger,
    location: Option<Location>,
    now: DateTime<Local>,
) -> impl Iterator<Item = DateTime<Local>> {
    [-1_i64, 0, 1]
        .into_iter()
        .filter_map(move |offset| now.date_naive().checked_add_signed(chrono::Duration::days(offset)))
        .filter_map(move |date| trigger_datetime(trigger, location, date))
}

fn trigger_datetime(trigger: RampTrigger, location: Option<Location>, date: NaiveDate) -> Option<DateTime<Local>> {
    match trigger {
        RampTrigger::TimeOfDay(hour, minute) => local_time_on(date, hour, minute),
        RampTrigger::CivilDawn(offset) => civil_time_on(date, location?, SolarEvent::Dawn(DawnType::Civil), offset),
        RampTrigger::CivilDusk(offset) => civil_time_on(date, location?, SolarEvent::Dusk(DawnType::Civil), offset),
    }
}

fn local_time_on(date: NaiveDate, hour: LocalHour, minute: LocalMinute) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(hour.as_u8() as u32, minute.as_u8() as u32, 0)?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(datetime) => Some(datetime),
        LocalResult::Ambiguous(first, _) => Some(first),
        LocalResult::None => None,
    }
}

fn civil_time_on(
    date: NaiveDate,
    location: Location,
    event: SolarEvent,
    offset: SignedMinutes,
) -> Option<DateTime<Local>> {
    let coordinates = Coordinates::new(location.latitude, location.longitude)?;
    let utc = SolarDay::new(coordinates, date).event_time(event)?;
    Some(utc.with_timezone(&Local) + chrono::Duration::minutes(offset.as_i16() as i64))
}
