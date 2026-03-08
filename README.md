# taskgun — A rusty gun for our [taskwarrior](https://taskwarrior.org)

A rusty gun for our [taskwarrior](https://taskwarrior.org) - bulk task generation, smart scheduling, and deadline-driven productivity.

## Why do you need taskgun?

Ever found a YouTube lecture series you want to complete but never quite finish? Started reading a technical book but lost momentum halfway through? **The problem is simple: no deadlines.**

Large sequential projects need structure and accountability. taskgun lets you break down big goals into smaller tasks with automatic deadlines, creating the external pressure you need to maintain momentum.

### Real-world Examples

#### 1. **YouTube Lecture Series** — 10 lectures, one per day

You discover a 10-part lecture series on Machine Learning. Schedule one lecture per day starting in 2 days:

```bash
taskgun create "ML Course" -p 10 -u Lecture --offset 2d --interval 1d
```

Creates:
- Lecture 1 (due in 2 days)
- Lecture 2 (due in 3 days)
- Lecture 3 (due in 4 days)
- ... and so on

#### 2. **Technical Book** — 12 chapters, one per week

Reading "Introduction to Algorithms" with 12 chapters. Schedule one chapter per week starting next week:

```bash
taskgun create CLRS -p 12 -u Chapter --offset 7d --interval 7d --skip weekend
```

Creates:
- Chapter 1 (due in 7 days, skips weekends)
- Chapter 2 (due in 14 days, skips weekends)
- Chapter 3 (due in 21 days, skips weekends)
- ... completing in 12 weeks

#### 3. **Book with Large Chapters** — Breaking down into sections

Reading "Design Patterns" with 3 chapters, but each chapter is too long. Chapter 1 has 3 sections, Chapter 2 has 4 sections, Chapter 3 has 2 sections. Schedule one section every 3 days:

```bash
taskgun create "Design Patterns" -p 3,4,2 -u Section --offset 3d --interval 3d
```

Creates:
- Section 1.1 (due in 3 days)
- Section 1.2 (due in 6 days)
- Section 1.3 (due in 9 days)
- Section 2.1 (due in 12 days)
- Section 2.2 (due in 15 days)
- Section 2.3 (due in 18 days)
- Section 2.4 (due in 21 days)
- Section 3.1 (due in 24 days)
- Section 3.2 (due in 27 days)

#### 4. **Quick Revision** — 30 lectures in 2 days

You have an exam in 2 days and need to revise 30 lectures quickly. Schedule one lecture every 2 hours, skipping bedtime:

```bash
taskgun create "Exam Prep" -p 30 -u Lecture --offset 2h --interval 2h --skip bedtime
```

Creates 30 lectures scheduled every 2 hours starting in 2 hours, automatically skipping 22:00-06:00 (bedtime). With ~16 waking hours per day, you'll complete all 30 lectures in approximately 2 days.

#### 5. **Rapid Practice** — 20 exercises with minute-level precision

You want to practice 20 short coding exercises, dedicating 30 minutes to each, with 15-minute breaks:

```bash
taskgun create "Coding Practice" -p 20 -u Exercise --offset 30m --interval 45min --skip bedtime
```

Creates 20 exercises scheduled every 45 minutes (30 min work + 15 min break), starting in 30 minutes, automatically skipping bedtime hours.

**Result:** What seemed like an overwhelming task is now a structured schedule you can follow.

---

## Features

- **Bulk task generation** — Create numbered task series with a single command
- **Instant keyword search** — Search tasks by typing `taskgun keyword` (searches projects and descriptions)
- **Flexible skip system** — Skip time windows and days using presets or custom rules
- **Smart scheduling** — Day-based, hour-based, or minute-based scheduling with mixed unit support
- **Hierarchical tasks** — Support for subsections (e.g., Part 1.1, 1.2, 2.1)
- **Configurable presets** — Define custom skip windows in .taskrc
- **Zero runtime dependencies** — Only requires [Taskwarrior](https://taskwarrior.org) itself

## Installation

### From crates.io

```bash
cargo install taskgun
```

### From source

```bash
git clone https://github.com/hamzamohdzubair/taskgun
cd taskgun
cargo install --path .
```

### Requirements

- [Taskwarrior](https://taskwarrior.org) 2.6.0 or later
- Rust 1.70+ (for building from source)

### Configure taskgun (Optional)

taskgun comes with built-in presets (`bedtime` and `weekend`), but you can customize them or add your own in `~/.taskrc`:

```bash
# Add this to your ~/.taskrc file

# === taskgun Configuration ===

# Customize bedtime hours (default: 2200-0600)
taskgun.skip.bedtime=2100-0500

# Customize weekend (default: Saturday, Sunday)
taskgun.skip.weekend=sat,sun

# Add custom skip windows
taskgun.skip.lunch=1200-1400
taskgun.skip.meetings=0900-1000,1400-1500
taskgun.skip.longweekend=fri,sat,sun
```

After adding these to your `.taskrc`, use them with the `--skip` option:

```bash
taskgun create Project --offset 2h --interval 3h --skip lunch --skip bedtime
```

**Note:** If you don't add anything to `.taskrc`, the built-in defaults work perfectly:
- `bedtime`: 22:00-06:00
- `weekend`: Saturday and Sunday

## Usage

### Keyword search

Search for tasks by keyword — searches in both project names and descriptions:

```bash
# Case-insensitive search (default)
taskgun search learning
taskgun search VIDEO          # Finds "Video 1", "Video 2", etc.

# Search by project name
taskgun search "Deep Learning"

# Shorthand syntax (same as above)
taskgun learning
taskgun "Deep Learning"
```

### Regex search

Use the `-r` or `--regex` flag for case-sensitive regex search:

```bash
# Find tasks matching regex pattern
taskgun search 'sec [0-9]' -r
taskgun search 'lec [0-9]+' --regex

# Match specific chapters/sections
taskgun search 'Video [12]' -r         # Only Video 1 and Video 2

# Shorthand syntax also works
taskgun 'sec [0-9]' -r
```

**Note:** Regex mode is case-sensitive by default. Use standard regex patterns supported by Taskwarrior.

### Basic task generation

Create 5 tasks without due dates:

```bash
taskgun create "Deep Learning" -p 5
```

Creates:
- Part 1
- Part 2
- Part 3
- Part 4
- Part 5

### Day-based scheduling

Create 5 tasks with the first due in 5 days, then every 7 days:

```bash
taskgun create "Deep Learning" -p 5 --offset 5d --interval 7d
```

Creates tasks due on:
- today+5d
- today+12d
- today+19d
- today+26d
- today+33d

### Hour-based scheduling

Create 5 tasks with the first due in 2 hours, then every 6 hours:

```bash
taskgun create "Deep Learning" -p 5 --offset 2h --interval 6h
```

### Minute-based scheduling

Create 10 tasks with the first due in 30 minutes, then every 45 minutes:

```bash
taskgun create "Practice" -p 10 --offset 30m --interval 45min
```

Both `m` and `min` suffixes are supported for minutes.

### Mixed unit scheduling

Mix any time units — taskgun automatically handles the conversion:

```bash
# Offset in hours, interval in minutes
taskgun create "Sprint Tasks" -p 10 --offset 2h --interval 30m

# Offset in days, interval in hours
taskgun create "Daily Reviews" -p 10 --offset 1d --interval 6h

# Any combination works!
taskgun create "Mixed" -p 5 --offset 3d --interval 90min
```

### Skip windows (bedtime, weekends, custom)

Skip specific time windows or days using the `--skip` option (can be used multiple times):

```bash
# Skip bedtime (22:00-06:00 by default)
taskgun create "Deep Learning" -p 5 --offset 2h --interval 6h --skip bedtime

# Skip weekends (Saturday and Sunday)
taskgun create "Deep Learning" -p 5 --offset 1d --interval 1d --skip weekend

# Skip custom time range
taskgun create "Deep Learning" -p 5 --offset 2h --interval 3h --skip 2100-0600

# Skip specific days
taskgun create "Deep Learning" -p 5 --offset 1d --interval 1d --skip fri,sat,sun

# Combine multiple skip rules
taskgun create "Deep Learning" -p 5 --offset 2h --interval 4h --skip bedtime --skip weekend
```

**Skip behavior:**
- Tasks landing in skip windows are pushed forward to the next valid time
- In hour/minute mode, subsequent tasks chain from the pushed time (maintains intervals)
- In day mode, each task is calculated independently from the base time

**Built-in presets:**
- `bedtime`: 22:00-06:00 (nighttime hours)
- `weekend`: Saturday and Sunday

### Hierarchical tasks with subsections

Create tasks with subsections (2 sections in chapter 1, 3 in chapter 2, 1 in chapter 3):

```bash
taskgun create "Deep Learning" -p 2,3,1 --offset 5d --interval 7d
```

Creates:
- Part 1.1 (today+5d)
- Part 1.2 (today+12d)
- Part 2.1 (today+19d)
- Part 2.2 (today+26d)
- Part 2.3 (today+33d)
- Part 3.1 (today+40d)

### Custom unit names

Use a different prefix instead of Part:

```bash
taskgun create "Deep Learning" -p 2,3,2 -u Lecture --offset 3d --interval 4d
```

Creates:
- Lecture 1.1, Lecture 1.2, Lecture 2.1, etc.

### Custom skip windows in .taskrc

Define your own skip windows in `~/.taskrc`:

```ini
# Define a lunch break
taskgun.skip.lunch=1200-1400

# Define a long weekend
taskgun.skip.longweekend=fri,sat,sun

# Override the bedtime preset
taskgun.skip.bedtime=2100-0500
```

Then use them:

```bash
taskgun create "Deep Learning" --offset 2h --interval 3h --skip lunch --skip bedtime
```

## Command Reference

### `taskgun create`

Generate a series of numbered [Taskwarrior](https://taskwarrior.org) tasks.

**Syntax:**
```bash
taskgun create <PROJECT> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `<PROJECT>` | String | [Taskwarrior](https://taskwarrior.org) project name (required, positional) |

**Options:**

| Flag | Short | Type | Required | Description |
|------|-------|------|----------|-------------|
| `--parts` | `-p` | String | Yes | Number of tasks (e.g., 10) or subsection structure (e.g., 2,3,1) |
| `--unit` | `-u` | String | No | Task name prefix (default: Part) |
| `--offset` | `-o` | String | No | Time until first task (e.g., 5d, 2h, 30m, 45min) — units can be mixed with interval |
| `--interval` | `-i` | String | No | Time between tasks (e.g., 7d, 6h, 15m, 20min) — units can be mixed with offset |
| `--skip` | | String | No | Skip window (can be used multiple times) |

**Parts format:**
- Simple number: `10`, `30`, `12` (creates numbered tasks: Video 1, Video 2, ...)
- Subsections: `2,3,1`, `3,4,2` (creates hierarchical: Video 1.1, 1.2, 2.1, ...)

**Skip values:**
- Named presets: `bedtime`, `weekend`, or custom from .taskrc
- Time ranges: `2200-0600`, `21:00-06:00`, `1200-1400`
- Day names: `sat,sun`, `friday,saturday,sunday`, `mon,tue,wed,thu,fri`

**Time units:**
- Days: `d` or `D` (e.g., `5d`, `10D`)
- Hours: `h` or `H` (e.g., `2h`, `24H`)
- Minutes: `m`, `M`, `min`, `MIN`, or `Min` (e.g., `30m`, `45min`, `90MIN`)
- Units can be mixed freely between offset and interval

**Notes:**
- `--parts` is required
- `--offset` and `--interval` must be provided together
- `--skip` can be used multiple times

### `taskgun search`

Search tasks by keyword or regex pattern.

**Syntax:**
```bash
taskgun search <KEYWORD> [OPTIONS]
# or shorthand:
taskgun <KEYWORD> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `<KEYWORD>` | String | Search keyword or regex pattern (required) |

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `--regex` | `-r` | Use regex mode (case-sensitive) |

**Examples:**

```bash
# Case-insensitive keyword search
taskgun search learning
taskgun learning                    # shorthand

# Regex search
taskgun search 'lec.*[0-9]+' -r
taskgun 'video [12]' -r            # shorthand
```

**Search behavior:**
- Default mode: case-insensitive search in project names and descriptions
- Regex mode (`-r`/`--regex`): case-sensitive regex matching
- Searches both project and description fields
- Uses Taskwarrior's native filtering
- Color-coded output with alternating row backgrounds
- Flexible column widths that adapt to terminal width

### `taskgun completions`

Generate shell completion scripts:

```bash
# Bash
taskgun completions bash > /usr/local/etc/bash_completion.d/taskgun

# Zsh
taskgun completions zsh > ~/.zfunc/_taskgun

# Fish
taskgun completions fish > ~/.config/fish/completions/taskgun.fish
```

## How It Works

taskgun shells out to [Taskwarrior](https://taskwarrior.org)'s `task add` command for each task. It:

1. Validates that [Taskwarrior](https://taskwarrior.org) is installed
2. Generates task names (simple or hierarchical)
3. Calculates due dates if scheduling is requested
4. Creates tasks one by one using `task add`

All tasks are created in your [Taskwarrior](https://taskwarrior.org) database and follow your configured workflows, hooks, and settings.

## Development

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- create "Test" -n 3

# Check code
cargo clippy -- -D warnings
cargo fmt --check

# Build release
cargo build --release
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Roadmap

- [x] v0.1.0: Create subcommand with smart scheduling
- [x] v0.2.0: Skip system with presets and custom rules
- [x] v0.3.0: Mixed unit support and refined defaults
- [x] v0.4.0: Keyword search functionality
- [ ] v0.5.0: Bulk modify subcommand
- [ ] v1.0.0: Stable release with comprehensive testing

## Links

- [Repository](https://github.com/hamzamohdzubair/taskgun)
- [Issue Tracker](https://github.com/hamzamohdzubair/taskgun/issues)
- [Taskwarrior](https://taskwarrior.org/)
