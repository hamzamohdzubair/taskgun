use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// A skip rule that can be applied to task scheduling
#[derive(Debug, Clone)]
pub enum SkipRule {
    /// Skip a time range (e.g., 2200-0600) - only for hour-based scheduling
    TimeRange { start_hour: u32, end_hour: u32 },
    /// Skip specific days of the week
    DaysOfWeek(Vec<Weekday>),
}

impl SkipRule {
    /// Check if a given datetime should be skipped
    pub fn should_skip(&self, time: DateTime<Local>, is_hour_mode: bool) -> bool {
        match self {
            SkipRule::TimeRange {
                start_hour,
                end_hour,
            } => {
                if !is_hour_mode {
                    return false; // Time ranges only apply in hour mode
                }
                let hour = time.hour();
                if start_hour < end_hour {
                    hour >= *start_hour && hour < *end_hour
                } else {
                    hour >= *start_hour || hour < *end_hour
                }
            }
            SkipRule::DaysOfWeek(days) => days.contains(&time.weekday()),
        }
    }

    /// Resolve a time that should be skipped to the next valid time
    pub fn resolve(&self, time: DateTime<Local>) -> DateTime<Local> {
        match self {
            SkipRule::TimeRange {
                start_hour,
                end_hour,
            } => {
                let hour = time.hour();

                // Check if we're in the skip window
                let in_skip_window = if start_hour < end_hour {
                    hour >= *start_hour && hour < *end_hour
                } else {
                    hour >= *start_hour || hour < *end_hour
                };

                if !in_skip_window {
                    return time;
                }

                // Push to end_hour
                if hour >= *start_hour {
                    // In first part of skip window (e.g., 22:00-23:59), push to end_hour next day
                    let next_day = time.date_naive() + chrono::Duration::days(1);
                    next_day
                        .and_hms_opt(*end_hour, 0, 0)
                        .expect("Valid time")
                        .and_local_timezone(Local)
                        .single()
                        .expect("Valid local time")
                } else {
                    // In second part of skip window (e.g., 00:00-05:59), push to end_hour same day
                    time.date_naive()
                        .and_hms_opt(*end_hour, 0, 0)
                        .expect("Valid time")
                        .and_local_timezone(Local)
                        .single()
                        .expect("Valid local time")
                }
            }
            SkipRule::DaysOfWeek(days) => {
                let mut current = time;
                while days.contains(&current.weekday()) {
                    current += chrono::Duration::days(1);
                }
                current
            }
        }
    }
}

/// Manager for skip presets (built-in and user-defined)
#[derive(Debug, Clone)]
pub struct SkipPresets {
    presets: HashMap<String, Vec<SkipRule>>,
}

impl SkipPresets {
    /// Create with built-in defaults
    pub fn with_defaults() -> Self {
        let mut presets = HashMap::new();

        // Built-in: bedtime (2200-0600)
        presets.insert(
            "bedtime".to_string(),
            vec![SkipRule::TimeRange {
                start_hour: 22,
                end_hour: 6,
            }],
        );

        // Built-in: weekend (Saturday, Sunday)
        presets.insert(
            "weekend".to_string(),
            vec![SkipRule::DaysOfWeek(vec![Weekday::Sat, Weekday::Sun])],
        );

        Self { presets }
    }

    /// Load user-defined presets from .taskrc
    pub fn load_from_taskrc(&mut self) -> Result<()> {
        let taskrc_path = Self::find_taskrc()?;
        if !taskrc_path.exists() {
            return Ok(()); // No .taskrc, use defaults only
        }

        let content = fs::read_to_string(&taskrc_path)
            .context(format!("Failed to read taskrc: {:?}", taskrc_path))?;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // Look for taskgun.skip.* settings
            if line.starts_with("taskgun.skip.") {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();

                    // Extract preset name: taskgun.skip.NAME=VALUE
                    if let Some(name) = key.strip_prefix("taskgun.skip.") {
                        let rules = Self::parse_skip_value(value)?;
                        self.presets.insert(name.to_string(), rules);
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the .taskrc file
    fn find_taskrc() -> Result<PathBuf> {
        // Check TASKRC environment variable first
        if let Ok(taskrc_env) = std::env::var("TASKRC") {
            return Ok(PathBuf::from(taskrc_env));
        }

        // Default location: ~/.taskrc
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".taskrc");
            return Ok(path);
        }

        anyhow::bail!("Could not find .taskrc (no HOME or TASKRC environment variable)");
    }

    /// Get a preset by name
    pub fn get(&self, name: &str) -> Option<&Vec<SkipRule>> {
        self.presets.get(name)
    }

    /// Parse a skip value from command line or config file
    fn parse_skip_value(s: &str) -> Result<Vec<SkipRule>> {
        let s = s.trim();

        // Try parsing as time range first (HHMM-HHMM or HH:MM-HH:MM)
        if s.contains('-') && s.chars().any(|c| c.is_ascii_digit()) {
            if let Ok(rule) = Self::parse_time_range(s) {
                return Ok(vec![rule]);
            }
        }

        // Try parsing as day list (mon,tue,wed or monday,tuesday,wednesday)
        if s.contains(',') || Self::is_day_name(s) {
            if let Ok(rule) = Self::parse_days(s) {
                return Ok(vec![rule]);
            }
        }

        anyhow::bail!("Invalid skip value: '{}'. Expected time range (e.g., '2200-0600') or day list (e.g., 'sat,sun')", s);
    }

    /// Parse a time range like "2200-0600" or "22:00-06:00"
    fn parse_time_range(s: &str) -> Result<SkipRule> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            anyhow::bail!("Time range must be in format 'HHMM-HHMM' or 'HH:MM-HH:MM'");
        }

        let start_hour = Self::parse_hour(parts[0])?;
        let end_hour = Self::parse_hour(parts[1])?;

        Ok(SkipRule::TimeRange {
            start_hour,
            end_hour,
        })
    }

    /// Parse an hour from "HHMM" or "HH:MM" format
    fn parse_hour(s: &str) -> Result<u32> {
        let s = s.trim();

        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid time format: '{}'", s);
            }
            let hour: u32 = parts[0].parse().context("Invalid hour")?;
            if hour > 23 {
                anyhow::bail!("Hour must be 0-23, got {}", hour);
            }
            return Ok(hour);
        }

        if s.len() == 4 {
            let hour: u32 = s[..2].parse().context("Invalid hour")?;
            if hour > 23 {
                anyhow::bail!("Hour must be 0-23, got {}", hour);
            }
            return Ok(hour);
        }

        anyhow::bail!("Time must be in format 'HHMM' or 'HH:MM', got '{}'", s);
    }

    /// Check if a string looks like a day name
    fn is_day_name(s: &str) -> bool {
        let s_lower = s.to_lowercase();
        matches!(
            s_lower.as_str(),
            "mon"
                | "monday"
                | "tue"
                | "tuesday"
                | "wed"
                | "wednesday"
                | "thu"
                | "thursday"
                | "fri"
                | "friday"
                | "sat"
                | "saturday"
                | "sun"
                | "sunday"
        )
    }

    /// Parse a comma-separated list of days like "mon,wed,fri" or "saturday,sunday"
    fn parse_days(s: &str) -> Result<SkipRule> {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        let mut days = Vec::new();

        for part in parts {
            let day = Self::parse_day(part)?;
            if !days.contains(&day) {
                days.push(day);
            }
        }

        if days.is_empty() {
            anyhow::bail!("No valid days found in: '{}'", s);
        }

        Ok(SkipRule::DaysOfWeek(days))
    }

    /// Parse a single day name (case-insensitive, supports abbreviations)
    fn parse_day(s: &str) -> Result<Weekday> {
        let s_lower = s.to_lowercase();
        match s_lower.as_str() {
            "mon" | "monday" => Ok(Weekday::Mon),
            "tue" | "tuesday" => Ok(Weekday::Tue),
            "wed" | "wednesday" => Ok(Weekday::Wed),
            "thu" | "thursday" => Ok(Weekday::Thu),
            "fri" | "friday" => Ok(Weekday::Fri),
            "sat" | "saturday" => Ok(Weekday::Sat),
            "sun" | "sunday" => Ok(Weekday::Sun),
            _ => anyhow::bail!("Invalid day name: '{}'", s),
        }
    }

    /// Parse a skip argument (could be preset name or direct value)
    pub fn parse_skip_arg(&self, s: &str) -> Result<Vec<SkipRule>> {
        // First check if it's a preset name
        if let Some(rules) = self.get(s) {
            return Ok(rules.clone());
        }

        // Otherwise try to parse as a direct value
        Self::parse_skip_value(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_range() {
        let rule = SkipPresets::parse_time_range("2200-0600").unwrap();
        match rule {
            SkipRule::TimeRange {
                start_hour,
                end_hour,
            } => {
                assert_eq!(start_hour, 22);
                assert_eq!(end_hour, 6);
            }
            _ => panic!("Expected TimeRange"),
        }

        let rule = SkipPresets::parse_time_range("22:00-06:00").unwrap();
        match rule {
            SkipRule::TimeRange {
                start_hour,
                end_hour,
            } => {
                assert_eq!(start_hour, 22);
                assert_eq!(end_hour, 6);
            }
            _ => panic!("Expected TimeRange"),
        }
    }

    #[test]
    fn test_parse_days() {
        let rule = SkipPresets::parse_days("sat,sun").unwrap();
        match rule {
            SkipRule::DaysOfWeek(days) => {
                assert_eq!(days.len(), 2);
                assert!(days.contains(&Weekday::Sat));
                assert!(days.contains(&Weekday::Sun));
            }
            _ => panic!("Expected DaysOfWeek"),
        }

        let rule = SkipPresets::parse_days("monday,wednesday,friday").unwrap();
        match rule {
            SkipRule::DaysOfWeek(days) => {
                assert_eq!(days.len(), 3);
                assert!(days.contains(&Weekday::Mon));
                assert!(days.contains(&Weekday::Wed));
                assert!(days.contains(&Weekday::Fri));
            }
            _ => panic!("Expected DaysOfWeek"),
        }
    }

    #[test]
    fn test_presets_defaults() {
        let presets = SkipPresets::with_defaults();

        // Check bedtime preset
        let bedtime = presets.get("bedtime").unwrap();
        assert_eq!(bedtime.len(), 1);
        match &bedtime[0] {
            SkipRule::TimeRange {
                start_hour,
                end_hour,
            } => {
                assert_eq!(*start_hour, 22);
                assert_eq!(*end_hour, 6);
            }
            _ => panic!("Expected TimeRange for bedtime"),
        }

        // Check weekend preset
        let weekend = presets.get("weekend").unwrap();
        assert_eq!(weekend.len(), 1);
        match &weekend[0] {
            SkipRule::DaysOfWeek(days) => {
                assert_eq!(days.len(), 2);
                assert!(days.contains(&Weekday::Sat));
                assert!(days.contains(&Weekday::Sun));
            }
            _ => panic!("Expected DaysOfWeek for weekend"),
        }
    }

    #[test]
    fn test_parse_skip_arg_preset() {
        let presets = SkipPresets::with_defaults();

        // Use preset name
        let rules = presets.parse_skip_arg("bedtime").unwrap();
        assert_eq!(rules.len(), 1);

        let rules = presets.parse_skip_arg("weekend").unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_parse_skip_arg_direct() {
        let presets = SkipPresets::with_defaults();

        // Direct time range
        let rules = presets.parse_skip_arg("2100-0500").unwrap();
        assert_eq!(rules.len(), 1);

        // Direct day list
        let rules = presets.parse_skip_arg("fri,sat,sun").unwrap();
        assert_eq!(rules.len(), 1);
    }
}
