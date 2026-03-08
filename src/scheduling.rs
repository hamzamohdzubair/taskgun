use anyhow::Result;
use chrono::{DateTime, Duration, Local};

use crate::skip::SkipRule;

/// Configuration for task scheduling
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    pub base_time: DateTime<Local>,
    pub offset: u32,
    pub interval: u32,
    pub use_hours: bool,
    pub use_minutes: bool,
    pub skip_rules: Vec<SkipRule>,
}

/// A scheduled time with both logical and resolved timestamps
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScheduledTime {
    pub logical: DateTime<Local>,
    pub resolved: DateTime<Local>,
    pub was_adjusted: bool,
}

/// Apply skip rules to a datetime
fn apply_skip_rules(time: DateTime<Local>, rules: &[SkipRule], is_hour_mode: bool) -> DateTime<Local> {
    let mut current = time;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops

    loop {
        if iterations >= MAX_ITERATIONS {
            // Safety break
            return current;
        }
        iterations += 1;

        let mut any_skip = false;
        for rule in rules {
            if rule.should_skip(current, is_hour_mode) {
                current = rule.resolve(current);
                any_skip = true;
                break; // Re-check all rules after adjustment
            }
        }

        if !any_skip {
            break;
        }
    }

    current
}

impl ScheduledTime {
    fn new(time: DateTime<Local>, skip_rules: &[SkipRule], is_hour_mode: bool) -> Self {
        let resolved = apply_skip_rules(time, skip_rules, is_hour_mode);

        Self {
            logical: time,
            was_adjusted: time != resolved,
            resolved,
        }
    }
}

impl ScheduleConfig {
    /// Schedule a series of tasks according to the configuration
    ///
    /// CRITICAL: In hour/minute mode, each task is scheduled from the RESOLVED time
    /// of the previous task, not the logical time. This ensures:
    /// - No two tasks share the same timestamp
    /// - The interval between consecutive tasks is always honored
    /// - Tasks don't pile up at the quiet hour boundary
    pub fn schedule_tasks(&self, count: usize) -> Result<Vec<ScheduledTime>> {
        let mut scheduled = Vec::with_capacity(count);

        if self.use_minutes {
            // Minute mode: sequential calculation from resolved times
            let current = self.base_time + Duration::minutes(self.offset as i64);
            let first = ScheduledTime::new(current, &self.skip_rules, true);
            scheduled.push(first);

            for _ in 1..count {
                let prev_resolved = scheduled.last().unwrap().resolved;
                let next_logical = prev_resolved + Duration::minutes(self.interval as i64);
                let next = ScheduledTime::new(next_logical, &self.skip_rules, true);
                scheduled.push(next);
            }
        } else if self.use_hours {
            // Hour mode: sequential calculation from resolved times
            let current = self.base_time + Duration::hours(self.offset as i64);
            let first = ScheduledTime::new(current, &self.skip_rules, true);
            scheduled.push(first);

            for _ in 1..count {
                let prev_resolved = scheduled.last().unwrap().resolved;
                let next_logical = prev_resolved + Duration::hours(self.interval as i64);
                let next = ScheduledTime::new(next_logical, &self.skip_rules, true);
                scheduled.push(next);
            }
        } else {
            // Day mode: independent calculation from base time
            for i in 0..count {
                let days_offset = self.offset as i64 + (i as i64 * self.interval as i64);
                let logical = self.base_time + Duration::days(days_offset);
                let resolved = ScheduledTime::new(logical, &self.skip_rules, false);
                scheduled.push(resolved);
            }
        }

        Ok(scheduled)
    }
}

/// Format a datetime for Taskwarrior's due field
/// Format: YYYY-MM-DDTHH:MM (in hour mode)
/// Format: YYYY-MM-DD (in day mode, but we use relative format)
pub fn format_for_taskwarrior(time: &DateTime<Local>, use_hours: bool) -> String {
    if use_hours {
        time.format("%Y-%m-%dT%H:%M").to_string()
    } else {
        time.format("%Y-%m-%d").to_string()
    }
}

/// Format a relative due date for day mode (e.g., "today+5d")
pub fn format_relative_days(base_time: DateTime<Local>, target_time: DateTime<Local>) -> String {
    let days = (target_time.date_naive() - base_time.date_naive()).num_days();
    if days == 0 {
        "today".to_string()
    } else {
        format!("today+{}d", days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skip::SkipRule;
    use chrono::{Datelike, NaiveDate, Timelike, Weekday};

    fn make_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single()
            .unwrap()
    }

    #[test]
    fn test_day_mode_scheduling() {
        let base = make_time(2025, 1, 15, 10, 0);
        let config = ScheduleConfig {
            base_time: base,
            offset: 5,
            interval: 7,
            use_hours: false,
            use_minutes: false,
            skip_rules: vec![],
        };

        let scheduled = config.schedule_tasks(3).unwrap();

        // Task 1: today+5d
        assert_eq!(
            (scheduled[0].resolved.date_naive() - base.date_naive()).num_days(),
            5
        );

        // Task 2: today+12d (5 + 1*7)
        assert_eq!(
            (scheduled[1].resolved.date_naive() - base.date_naive()).num_days(),
            12
        );

        // Task 3: today+19d (5 + 2*7)
        assert_eq!(
            (scheduled[2].resolved.date_naive() - base.date_naive()).num_days(),
            19
        );
    }

    #[test]
    fn test_hour_mode_with_skip_time_range() {
        // Base: 20:00, offset: 1h, interval: 3h
        let base = make_time(2025, 1, 15, 20, 0);
        let skip_rules = vec![SkipRule::TimeRange {
            start_hour: 22,
            end_hour: 6,
        }];
        let config = ScheduleConfig {
            base_time: base,
            offset: 1,
            interval: 3,
            use_hours: true,
            use_minutes: false,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(4).unwrap();

        // Task 1: 20:00+1h = 21:00 (valid)
        assert_eq!(scheduled[0].resolved.hour(), 21);
        assert!(!scheduled[0].was_adjusted);

        // Task 2: 21:00+3h = 00:00 → pushed to 06:00 next day
        assert_eq!(scheduled[1].resolved.hour(), 6);
        assert!(scheduled[1].was_adjusted);

        // Task 3: 06:00+3h = 09:00 (valid, uses resolved time from task 2)
        assert_eq!(scheduled[2].resolved.hour(), 9);
        assert!(!scheduled[2].was_adjusted);

        // Task 4: 09:00+3h = 12:00 (valid)
        assert_eq!(scheduled[3].resolved.hour(), 12);
        assert!(!scheduled[3].was_adjusted);
    }

    #[test]
    fn test_skip_weekends_day_mode() {
        // Start on Thursday 2025-01-16
        let base = make_time(2025, 1, 16, 10, 0);
        let skip_rules = vec![SkipRule::DaysOfWeek(vec![Weekday::Sat, Weekday::Sun])];
        let config = ScheduleConfig {
            base_time: base,
            offset: 1,
            interval: 1,
            use_hours: false,
            use_minutes: false,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(5).unwrap();

        // In day mode, each task is calculated independently from base time
        // Task 0: Thu (base) + 1d = Fri (Jan 17) - no skip
        assert_eq!(scheduled[0].resolved.weekday(), Weekday::Fri);
        assert_eq!(scheduled[0].resolved.day(), 17);

        // Task 1: Thu (base) + 2d = Sat (Jan 18) → skip to Mon (Jan 20)
        assert_eq!(scheduled[1].resolved.weekday(), Weekday::Mon);
        assert_eq!(scheduled[1].resolved.day(), 20);

        // Task 2: Thu (base) + 3d = Sun (Jan 19) → skip to Mon (Jan 20)
        assert_eq!(scheduled[2].resolved.weekday(), Weekday::Mon);
        assert_eq!(scheduled[2].resolved.day(), 20);

        // Task 3: Thu (base) + 4d = Mon (Jan 20) - no skip
        assert_eq!(scheduled[3].resolved.weekday(), Weekday::Mon);
        assert_eq!(scheduled[3].resolved.day(), 20);

        // Task 4: Thu (base) + 5d = Tue (Jan 21) - no skip
        assert_eq!(scheduled[4].resolved.weekday(), Weekday::Tue);
        assert_eq!(scheduled[4].resolved.day(), 21);
    }

    #[test]
    fn test_hour_mode_chains_from_resolved_times() {
        // Verify that tasks chain from resolved times, not logical times
        let base = make_time(2025, 1, 15, 21, 0);
        let skip_rules = vec![SkipRule::TimeRange {
            start_hour: 22,
            end_hour: 6,
        }];
        let config = ScheduleConfig {
            base_time: base,
            offset: 2,
            interval: 1,
            use_hours: true,
            use_minutes: false,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(3).unwrap();

        // Task 1: 21:00+2h = 23:00 → pushed to 06:00 next day
        assert_eq!(scheduled[0].resolved.hour(), 6);

        // Task 2: 06:00+1h = 07:00 (from resolved task 1, not from 23:00+1h=00:00)
        assert_eq!(scheduled[1].resolved.hour(), 7);

        // Task 3: 07:00+1h = 08:00
        assert_eq!(scheduled[2].resolved.hour(), 8);
    }

    #[test]
    fn test_multiple_skip_rules() {
        // Skip both bedtime and weekends
        let base = make_time(2025, 1, 17, 21, 0); // Friday 21:00
        let skip_rules = vec![
            SkipRule::TimeRange {
                start_hour: 22,
                end_hour: 6,
            },
            SkipRule::DaysOfWeek(vec![Weekday::Sat, Weekday::Sun]),
        ];
        let config = ScheduleConfig {
            base_time: base,
            offset: 2,
            interval: 12,
            use_hours: true,
            use_minutes: false,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(3).unwrap();

        // Task 1: Fri 21:00+2h = Fri 23:00 → Sat 06:00 → Mon 06:00 (skip weekend)
        assert_eq!(scheduled[0].resolved.weekday(), Weekday::Mon);
        assert_eq!(scheduled[0].resolved.hour(), 6);

        // Task 2: Mon 06:00+12h = Mon 18:00 (valid)
        assert_eq!(scheduled[1].resolved.weekday(), Weekday::Mon);
        assert_eq!(scheduled[1].resolved.hour(), 18);

        // Task 3: Mon 18:00+12h = Tue 06:00 (skips bedtime)
        assert_eq!(scheduled[2].resolved.weekday(), Weekday::Tue);
        assert_eq!(scheduled[2].resolved.hour(), 6);
    }

    #[test]
    fn test_minute_mode_scheduling() {
        // Base: 10:00, offset: 30min, interval: 45min
        let base = make_time(2025, 1, 15, 10, 0);
        let config = ScheduleConfig {
            base_time: base,
            offset: 30,
            interval: 45,
            use_hours: false,
            use_minutes: true,
            skip_rules: vec![],
        };

        let scheduled = config.schedule_tasks(4).unwrap();

        // Task 1: 10:00+30min = 10:30
        assert_eq!(scheduled[0].resolved.hour(), 10);
        assert_eq!(scheduled[0].resolved.minute(), 30);

        // Task 2: 10:30+45min = 11:15
        assert_eq!(scheduled[1].resolved.hour(), 11);
        assert_eq!(scheduled[1].resolved.minute(), 15);

        // Task 3: 11:15+45min = 12:00
        assert_eq!(scheduled[2].resolved.hour(), 12);
        assert_eq!(scheduled[2].resolved.minute(), 0);

        // Task 4: 12:00+45min = 12:45
        assert_eq!(scheduled[3].resolved.hour(), 12);
        assert_eq!(scheduled[3].resolved.minute(), 45);
    }

    #[test]
    fn test_minute_mode_with_skip_time_range() {
        // Base: 21:30, offset: 15min, interval: 20min
        let base = make_time(2025, 1, 15, 21, 30);
        let skip_rules = vec![SkipRule::TimeRange {
            start_hour: 22,
            end_hour: 6,
        }];
        let config = ScheduleConfig {
            base_time: base,
            offset: 15,
            interval: 20,
            use_hours: false,
            use_minutes: true,
            skip_rules,
        };

        let scheduled = config.schedule_tasks(3).unwrap();

        // Task 1: 21:30+15min = 21:45 (valid)
        assert_eq!(scheduled[0].resolved.hour(), 21);
        assert_eq!(scheduled[0].resolved.minute(), 45);

        // Task 2: 21:45+20min = 22:05 → pushed to 06:00 next day
        assert_eq!(scheduled[1].resolved.hour(), 6);
        assert_eq!(scheduled[1].resolved.minute(), 0);

        // Task 3: 06:00+20min = 06:20 (valid, uses resolved time from task 2)
        assert_eq!(scheduled[2].resolved.hour(), 6);
        assert_eq!(scheduled[2].resolved.minute(), 20);
    }
}
