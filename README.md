# taskgun — A gun to shoot tasks for Taskwarrior

A Rust CLI that extends [Taskwarrior](https://taskwarrior.org/) with power-user workflows: bulk task generation, smart scheduling, and more.

## Features

- **Bulk task generation** — Create numbered task series with a single command
- **Smart scheduling** — Day-based or hour-based scheduling with quiet window support
- **Hierarchical tasks** — Support for subsections (e.g., Video 1.1, 1.2, 2.1)
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
taskgun create -p "Deep Learning" -n 5
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
taskgun create -p "Deep Learning" -n 5 --offset 5 --interval 7
```

Creates tasks due on:
- today+5d
- today+12d
- today+19d
- today+26d
- today+33d

### Hour-based scheduling with quiet window

Create 5 tasks with the first due in 2 hours, then every 6 hours, avoiding 22:00-06:00:

```bash
taskgun create -p "Deep Learning" -n 5 --offset 2 --interval 6 --hours
```

**Quiet window behavior:**
- Any task landing between 22:00-06:00 is pushed to 06:00
- Subsequent tasks are scheduled from the pushed time, not the original time
- This ensures no two tasks share a timestamp and intervals are preserved

### Hierarchical tasks with subsections

Create tasks with subsections (2 sections in chapter 1, 3 in chapter 2, 1 in chapter 3):

```bash
taskgun create -p "Deep Learning" -s "2,3,1" --offset 5 --interval 7
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
taskgun create -p "Deep Learning" -s "2,3,2" -u "Lecture" --offset 3 --interval 4
```

Creates:
- Lecture 1.1, Lecture 1.2, Lecture 2.1, etc.

## Command Reference

### `taskgun create`

Generate a series of numbered Taskwarrior tasks.

**Options:**

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--project` | `-p` | String | required | Taskwarrior project name |
| `--count` | `-n` | u32 | 10 | Number of chapters |
| `--unit` | `-u` | String | "Video" | Task name prefix |
| `--offset` | `-o` | u32 | — | Delay before first task is due |
| `--interval` | `-i` | u32 | — | Gap between consecutive tasks |
| `--hours` | | flag | false | Treat offset/interval as hours (not days) |
| `--subsections` | `-s` | String | — | Comma-separated subsection counts |

**Notes:**
- `--offset` and `--interval` must be provided together
- When `--subsections` is given, `--count` is inferred from the number of chapters

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
RUST_LOG=debug cargo run -- create -p "Test" -n 3

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
