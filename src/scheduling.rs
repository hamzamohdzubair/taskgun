use anyhow::Result;
use chrono::{DateTime, Duration, Local, Timelike};

/// Configuration for task scheduling
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    pub base_time: DateTime<Local>,
    pub offset: u32,
    pub interval: u32,
    pub use_hours: bool,
}

/// A scheduled time with both logical and resolved timestamps
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScheduledTime {
    pub logical: DateTime<Local>,
    pub resolved: DateTime<Local>,
    pub was_adjusted: bool,
}

impl ScheduledTime {
    fn new(time: DateTime<Local>) -> Self {
        let resolved = resolve_quiet_window(time);
        Self {
            logical: time,
            was_adjusted: time != resolved,
            resolved,
        }
    }
}

/// Push timestamps from 22:00-06:00 window to 06:00
/// - If hour >= 22: push to 06:00 next day
/// - If hour < 6: push to 06:00 same day
/// - Otherwise: no change
fn resolve_quiet_window(time: DateTime<Local>) -> DateTime<Local> {
    let hour = time.hour();

    if hour >= 22 {
        // Push to 06:00 next day
        let next_day = time.date_naive() + Duration::days(1);
        next_day
            .and_hms_opt(6, 0, 0)
            .expect("Valid time")
            .and_local_timezone(Local)
            .single()
            .expect("Valid local time")
    } else if hour < 6 {
        // Push to 06:00 same day
        time.date_naive()
            .and_hms_opt(6, 0, 0)
            .expect("Valid time")
            .and_local_timezone(Local)
            .single()
            .expect("Valid local time")
    } else {
        // Valid time, no change
        time
    }
}

impl ScheduleConfig {
    /// Schedule a series of tasks according to the configuration
    ///
    /// CRITICAL: In hour mode, each task is scheduled from the RESOLVED time
    /// of the previous task, not the logical time. This ensures:
    /// - No two tasks share the same timestamp
    /// - The interval between consecutive tasks is always honored
    /// - Tasks don't pile up at 06:00
    pub fn schedule_tasks(&self, count: usize) -> Result<Vec<ScheduledTime>> {
        let mut scheduled = Vec::with_capacity(count);

        if self.use_hours {
            // Hour mode: sequential calculation from resolved times
            let first_logical = self.base_time + Duration::hours(self.offset as i64);
            let first = ScheduledTime::new(first_logical);
            scheduled.push(first);

            for _ in 1..count {
                let prev_resolved = scheduled.last().unwrap().resolved;
                let next_logical = prev_resolved + Duration::hours(self.interval as i64);
                let next = ScheduledTime::new(next_logical);
                scheduled.push(next);
            }
        } else {
            // Day mode: independent calculation from base time
            for i in 0..count {
                let days_offset = self.offset as i64 + (i as i64 * self.interval as i64);
                let logical = self.base_time + Duration::days(days_offset);
                // No quiet window resolution in day mode
                scheduled.push(ScheduledTime {
                    logical,
                    resolved: logical,
                    was_adjusted: false,
                });
            }
        }

        Ok(scheduled)
    }
}

/// Format a datetime for Taskwarrior's due field
/// Format: YYYY-MM-DDTHH:MM (no seconds in hour mode)
/// Format: YYYY-MM-DD (day mode, but we use relative format)
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
    use chrono::NaiveDate;

    fn make_time(hour: u32, minute: u32) -> DateTime<Local> {
        NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single()
            .unwrap()
    }

    #[test]
    fn test_quiet_window_boundaries() {
        // Before window (valid)
        assert_eq!(resolve_quiet_window(make_time(5, 59)).hour(), 6);
        assert_eq!(resolve_quiet_window(make_time(6, 0)).hour(), 6);
        assert_eq!(resolve_quiet_window(make_time(6, 1)).hour(), 6);

        // During day (valid)
        assert_eq!(resolve_quiet_window(make_time(12, 0)).hour(), 12);
        assert_eq!(resolve_quiet_window(make_time(21, 59)).hour(), 21);

        // After window start (push to next day)
        assert_eq!(resolve_quiet_window(make_time(22, 0)).hour(), 6);
        assert_eq!(resolve_quiet_window(make_time(23, 59)).hour(), 6);

        // Midnight (push to same day 06:00)
        assert_eq!(resolve_quiet_window(make_time(0, 0)).hour(), 6);
    }

    #[test]
    fn test_quiet_window_next_day() {
        let time_22 = make_time(22, 0);
        let resolved = resolve_quiet_window(time_22);

        // Should be pushed to 06:00 next day
        assert_eq!(resolved.hour(), 6);
        assert_eq!(resolved.minute(), 0);
        assert_eq!(
            resolved.date_naive(),
            time_22.date_naive() + Duration::days(1)
        );
    }

    #[test]
    fn test_quiet_window_same_day() {
        let time_03 = make_time(3, 0);
        let resolved = resolve_quiet_window(time_03);

        // Should be pushed to 06:00 same day
        assert_eq!(resolved.hour(), 6);
        assert_eq!(resolved.minute(), 0);
        assert_eq!(resolved.date_naive(), time_03.date_naive());
    }

    #[test]
    fn test_day_mode_scheduling() {
        let base = make_time(10, 0);
        let config = ScheduleConfig {
            base_time: base,
            offset: 5,
            interval: 7,
            use_hours: false,
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

        // No adjustments in day mode
        assert!(!scheduled[0].was_adjusted);
        assert!(!scheduled[1].was_adjusted);
        assert!(!scheduled[2].was_adjusted);
    }

    #[test]
    fn test_hour_mode_with_quiet_window() {
        // Base: 20:00, offset: 1h, interval: 3h
        let base = make_time(20, 0);
        let config = ScheduleConfig {
            base_time: base,
            offset: 1,
            interval: 3,
            use_hours: true,
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
    fn test_hour_mode_chains_from_resolved_times() {
        // Verify that tasks chain from resolved times, not logical times
        let base = make_time(21, 0);
        let config = ScheduleConfig {
            base_time: base,
            offset: 2,
            interval: 1,
            use_hours: true,
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
    fn test_format_for_taskwarrior_hour_mode() {
        let time = make_time(14, 30);
        let formatted = format_for_taskwarrior(&time, true);
        assert!(formatted.ends_with("T14:30"));
    }

    #[test]
    fn test_format_relative_days() {
        let base = make_time(10, 0);
        let same_day = make_time(15, 0);
        let next_day = base + Duration::days(1);
        let five_days = base + Duration::days(5);

        assert_eq!(format_relative_days(base, same_day), "today");
        assert_eq!(format_relative_days(base, next_day), "today+1d");
        assert_eq!(format_relative_days(base, five_days), "today+5d");
    }
}
