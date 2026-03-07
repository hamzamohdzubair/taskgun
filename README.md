# taskgun — A gun to shoot tasks for Taskwarrior

A Rust CLI that extends [Taskwarrior](https://taskwarrior.org/) with power-user workflows: bulk task generation, smart scheduling, and more.

## Features

- **Bulk task generation** — Create numbered task series with a single command
- **Flexible skip system** — Skip time windows and days using presets or custom rules
- **Smart scheduling** — Day-based or hour-based scheduling
- **Hierarchical tasks** — Support for subsections (e.g., Video 1.1, 1.2, 2.1)
- **Configurable presets** — Define custom skip windows in .taskrc
- **Zero runtime dependencies** — Only requires Taskwarrior itself

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

- Taskwarrior 2.6.0 or later
- Rust 1.70+ (for building from source)

## Usage

### Basic task generation

Create 5 tasks without due dates:

```bash
taskgun create "Deep Learning" -n 5
```

Creates:
- Video 1
- Video 2
- Video 3
- Video 4
- Video 5

### Day-based scheduling

Create 5 tasks with the first due in 5 days, then every 7 days:

```bash
taskgun create "Deep Learning" -n 5 --offset 5d --interval 7d
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
taskgun create "Deep Learning" -n 5 --offset 2h --interval 6h
```

### Skip windows (bedtime, weekends, custom)

Skip specific time windows or days using the `--skip` option (can be used multiple times):

```bash
# Skip bedtime (22:00-06:00 by default)
taskgun create "Deep Learning" -n 5 --offset 2h --interval 6h --skip bedtime

# Skip weekends (Saturday and Sunday)
taskgun create "Deep Learning" -n 5 --offset 1d --interval 1d --skip weekend

# Skip custom time range
taskgun create "Deep Learning" -n 5 --offset 2h --interval 3h --skip 2100-0600

# Skip specific days
taskgun create "Deep Learning" -n 5 --offset 1d --interval 1d --skip fri,sat,sun

# Combine multiple skip rules
taskgun create "Deep Learning" -n 5 --offset 2h --interval 4h --skip bedtime --skip weekend
```

**Skip behavior:**
- Tasks landing in skip windows are pushed forward to the next valid time
- In hour mode, subsequent tasks chain from the pushed time
- In day mode, each task is calculated independently

**Built-in presets:**
- `bedtime`: 22:00-06:00 (nighttime hours)
- `weekend`: Saturday and Sunday

### Hierarchical tasks with subsections

Create tasks with subsections (2 sections in chapter 1, 3 in chapter 2, 1 in chapter 3):

```bash
taskgun create "Deep Learning" -s "2,3,1" --offset 5d --interval 7d
```

Creates:
- Video 1.1 (today+5d)
- Video 1.2 (today+12d)
- Video 2.1 (today+19d)
- Video 2.2 (today+26d)
- Video 2.3 (today+33d)
- Video 3.1 (today+40d)

### Custom unit names

Use a different prefix instead of "Video":

```bash
taskgun create "Deep Learning" -s "2,3,2" -u "Lecture" --offset 3d --interval 4d
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

Generate a series of numbered Taskwarrior tasks.

**Syntax:**
```bash
taskgun create <PROJECT> [OPTIONS]
```

**Arguments:**

| Argument | Type | Description |
|----------|------|-------------|
| `<PROJECT>` | String | Taskwarrior project name (required, positional) |

**Options:**

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--count` | `-n` | u32 | 10 | Number of chapters |
| `--unit` | `-u` | String | "Video" | Task name prefix |
| `--offset` | `-o` | String | — | Time until first task (e.g., "5d", "2h") |
| `--interval` | `-i` | String | — | Time between tasks (e.g., "7d", "6h") |
| `--skip` | | String | — | Skip window (can be used multiple times) |
| `--subsections` | `-s` | String | — | Comma-separated subsection counts |

**Skip values:**
- Named presets: `bedtime`, `weekend`, or custom from .taskrc
- Time ranges: `2200-0600`, `21:00-06:00`, `1200-1400`
- Day names: `sat,sun`, `friday,saturday,sunday`, `mon,tue,wed,thu,fri`

**Notes:**
- `--offset` and `--interval` must be provided together
- Both must use the same unit (either "d" for days or "h" for hours)
- When `--subsections` is given, `--count` is inferred from the number of chapters
- `--skip` can be used multiple times to apply multiple skip rules

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

taskgun shells out to Taskwarrior's `task add` command for each task. It:

1. Validates that Taskwarrior is installed
2. Generates task names (simple or hierarchical)
3. Calculates due dates if scheduling is requested
4. Creates tasks one by one using `task add`

All tasks are created in your Taskwarrior database and follow your configured workflows, hooks, and settings.

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
- [ ] v0.2.0: Search subcommand with filters
- [ ] v0.3.0: Bulk modify subcommand
- [ ] v1.0.0: Stable release with comprehensive testing

## Links

- [Repository](https://github.com/hamzamohdzubair/taskgun)
- [Issue Tracker](https://github.com/hamzamohdzubair/taskgun/issues)
- [Taskwarrior](https://taskwarrior.org/)
