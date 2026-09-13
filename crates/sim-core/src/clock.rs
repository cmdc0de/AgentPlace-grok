//! Day/night overlay `[time]`. Hashed when enabled. Day/tod are derived from `tick`.

use serde::Deserialize;

pub const DEFAULT_TICKS_PER_DAY: u64 = 240;
const DAY_LIGHT: f32 = 1.0;
const NIGHT_LIGHT: f32 = 0.15;
const TWILIGHT_TICKS: u64 = 10;

/// Overlay `[time]`. Not on `ExperimentConfig`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeParams {
    pub enabled: bool,
    pub ticks_per_day: u64,
}

impl Default for TimeParams {
    fn default() -> Self {
        Self {
            enabled: true,
            ticks_per_day: DEFAULT_TICKS_PER_DAY,
        }
    }
}

impl TimeParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            time: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
            ticks_per_day: Option<u64>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.time.enabled.unwrap_or(true),
            ticks_per_day: slice
                .time
                .ticks_per_day
                .unwrap_or(DEFAULT_TICKS_PER_DAY)
                .max(1),
        }
    }
}

pub fn day_tod(tick: u64, ticks_per_day: u64) -> (u64, u64) {
    let tpd = ticks_per_day.max(1);
    (tick / tpd, tick % tpd)
}

pub fn is_night(tod: u64, ticks_per_day: u64) -> bool {
    let tpd = ticks_per_day.max(1);
    tod >= tpd * 3 / 4
}

pub fn is_dawn(tick: u64, ticks_per_day: u64) -> bool {
    let tpd = ticks_per_day.max(1);
    tick > 0 && tick % tpd == 0
}

/// Tiredness-scaled dawn refill. Empty leftover ⇒ 20% max; half ⇒ 40%; full ⇒ 60% then clamp.
pub fn dawn_refill(energy: u32, max: u32) -> u32 {
    if max == 0 {
        return energy;
    }
    let remaining_milli = (u64::from(energy) * 1000) / u64::from(max);
    let refill = u64::from(max) * (200 + remaining_milli * 4 / 10) / 1000;
    energy.saturating_add(refill as u32).min(max)
}

/// Hash-neutral viewer helper. Day ~1.0, night ~0.15, 10-tick twilight lerp.
pub fn light_for_tod(tod: u64, ticks_per_day: u64) -> f32 {
    let tpd = ticks_per_day.max(1);
    let night_start = tpd * 3 / 4;
    let tw = TWILIGHT_TICKS.min(night_start).min(tpd.saturating_sub(night_start)).max(1);
    if tod < tw {
        lerp(NIGHT_LIGHT, DAY_LIGHT, tod as f32 / tw as f32)
    } else if tod >= night_start {
        NIGHT_LIGHT
    } else if tod + tw > night_start {
        let into = (tod + tw - night_start) as f32 / tw as f32;
        lerp(DAY_LIGHT, NIGHT_LIGHT, into.clamp(0.0, 1.0))
    } else {
        DAY_LIGHT
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omit_time_table_is_enabled() {
        let p = TimeParams::from_config_toml("");
        assert!(p.enabled);
        assert_eq!(p.ticks_per_day, 240);
        let off = TimeParams::from_config_toml("[time]\nenabled = false\n");
        assert!(!off.enabled);
        let custom = TimeParams::from_config_toml("[time]\nticks_per_day = 120\n");
        assert!(custom.enabled);
        assert_eq!(custom.ticks_per_day, 120);
    }

    #[test]
    fn day_tod_two_ticks() {
        assert_eq!(day_tod(2, 240), (0, 2));
        assert_eq!(day_tod(240, 240), (1, 0));
        assert_eq!(day_tod(241, 240), (1, 1));
    }

    #[test]
    fn dawn_refill_table() {
        let max = 10_000u32;
        assert_eq!(dawn_refill(0, max), 2_000);
        assert_eq!(dawn_refill(5_000, max), 9_000);
        assert_eq!(dawn_refill(max, max), max);
        assert_eq!(dawn_refill(100, 0), 100);
    }

    #[test]
    fn light_day_bright_night_dim() {
        let day = light_for_tod(60, 240);
        let night = light_for_tod(200, 240);
        assert!(day > 0.9, "day={day}");
        assert!((night - 0.15).abs() < 0.01, "night={night}");
        let dusk = light_for_tod(175, 240);
        assert!(dusk < day && dusk > night, "dusk={dusk}");
        let dawn = light_for_tod(5, 240);
        assert!(dawn > night && dawn < day, "dawn_twilight={dawn}");
    }
}
