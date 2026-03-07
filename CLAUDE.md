# taskgun — A gun to shoot tasks for our Taskwarrior

A Rust CLI published on crates.io that extends Taskwarrior with power-user
workflows: bulk task generation, smart scheduling, search, and bulk modification.

---

## Project Goals

- Publish as `taskgun` on crates.io (`cargo install taskw`)
- Subcommand-based interface: `taskgun <subcommand> [options]`
- Zero runtime dependencies beyond Taskwarrior itself
- Clean `--help` output for every subcommand via `clap`

---

## Tech Stack

```toml
# Cargo.toml dependencies
clap = { version = "4", features = ["derive"] }   # CLI + subcommands + --help
chrono = "0.4"                                      # time arithmetic + quiet window
anyhow = "1"                                        # error handling
```

---

## Subcommands (Planned)

### `taskgun create` — Bulk task generation ← **start here**

Generates a series of numbered Taskwarrior tasks under a project.

#### Arguments

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--project` | `-p` | String | required | Taskwarrior project name |
| `--count` | `-n` | u32 | 10 | Number of chapters |
| `--unit` | `-u` | String | `"Video"` | Task name prefix |
| `--offset` | `-o` | u32 | — | Delay before first task is due |
| `--interval` | `-i` | u32 | — | Gap between consecutive tasks |
| `--hours` | | bool flag | false | Treat offset/interval as hours (not days) |
| `--subsections` | `-s` | String | — | Comma-separated subsection counts e.g. `2,3,1` |

`--offset` and `--interval` must always be provided together.

#### Task naming

- Without `--subsections`: `Video 1`, `Video 2`, ..., `Video N`
- With `--subsections "2,3,1"`: `Video 1.1`, `Video 1.2`, `Video 2.1`, `Video 2.2`, `Video 2.3`, `Video 3.1`
- Chapter count is inferred from the length of `--subsections` if `--count` is not provided

#### Due date scheduling

**Day mode** (default):
```
Task 1 → today + offset days
Task 2 → today + offset + 1×interval days
Task N → today + offset + (N-1)×interval days
```

**Hour mode** (`--hours` flag):
- Each task is `interval` hours after the *resolved* time of the previous task
- Quiet window: **22:00–06:00** — any timestamp landing in this window is
  pushed forward to **06:00** of the appropriate day, and the *next* task
  is scheduled `interval` hours after that pushed time (not the original
  logical time). This means no two tasks ever share a timestamp, and the
  interval between consecutive tasks is always honoured.

```
# Example: now = 20:00, offset = 1h, interval = 3h
Task 1 → 21:00           (valid)
Task 2 → 21:00+3h=00:00  → pushed to 06:00 next day
Task 3 → 06:00+3h=09:00  (valid)
Task 4 → 09:00+3h=12:00  (valid)
```

#### Example invocations

```bash
# Simple, no due dates
taskgun create -p "Deep Learning" -n 5

# Days-based scheduling
taskgun create -p "Deep Learning" -n 5 --offset 5 --interval 7

# Variable subsections, days-based
taskgun create -p "Deep Learning" -s "2,3,1" --offset 5 --interval 7

# Hour-based with quiet window
taskgun create -p "Deep Learning" -n 5 --offset 2 --interval 6 --hours

# Custom unit name
taskgun create -p "Deep Learning" -s "2,3,2" -u "Lecture" --offset 3 --interval 4
```

---

### `taskgun search` — Search tasks ← **planned**

Filter and display tasks by project, tag, due date range, status, etc.

```bash
taskgun search --project "Deep Learning" --tag youtube
taskgun search --due-before 2025-12-31 --status pending
```

---

### `taskgun modify` — Bulk modification ← **planned**

Modify multiple tasks matching a filter in one command.

```bash
taskgun modify --project "Deep Learning" --due-shift +2d
taskgun modify --project "Deep Learning" --tag +reviewed
```

---

## Implementation Notes

- Invoke Taskwarrior by shelling out: `std::process::Command::new("task")`
- For `create`, build and execute one `task add` call per task
- For `search`/`modify`, build the appropriate `task` filter + command
- Use `chrono::Local::now()` as the base for all time calculations
- Quiet window check: extract hour from timestamp, push to 06:00 if `h >= 22` or `h < 6`
- All errors should surface cleanly via `anyhow` with context messages

---

## Project Structure

```
taskgun/
├── Cargo.toml
├── CLAUDE.md          ← this file
├── README.md
└── src/
    ├── main.rs        # clap entry point, subcommand dispatch
    └── commands/
        ├── mod.rs
        ├── create.rs  # create subcommand logic
        ├── search.rs  # search subcommand logic (planned)
        └── modify.rs  # modify subcommand logic (planned)
```

---

## Reference: Original Bash Implementation

The bash prototype this project is based on, preserved here for logic reference:

```bash

taskcr() {
    local project=""
    local count=""
    local unit="Video"
    local start_offset=""
    local interval=""
    local subsections=""
    local use_hours=false

    local usage="Usage: task_series --project <name> [options]

Options:
  --project,     -p  Project name (required)
  --count,       -n  Number of chapters (default: 10)
  --unit,        -u  Task name prefix (default: 'Video')
  --offset,      -o  Days (or hours if --hours) until first task is due
  --interval,    -i  Days (or hours if --hours) between each task
  --hours            Treat --offset and --interval as hours, skipping 2200-0600
  --subsections, -s  Comma-separated subsection counts per chapter e.g. 2,3,1

Examples:
  task_series -p 'Deep Learning' -n 5
  task_series -p 'Deep Learning' -n 5 --offset 5 --interval 7
  task_series -p 'Deep Learning' -s '2,3,1' --offset 5 --interval 7
  task_series -p 'Deep Learning' -s '2,3,2' -u 'Lecture' --offset 3 --interval 4
  task_series -p 'Deep Learning' -n 5 --offset 2 --interval 6 --hours"

    [[ $# -eq 0 ]] && echo "$usage" && return 0

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --project|-p)     project="$2";      shift 2 ;;
            --count|-n)       count="$2";         shift 2 ;;
            --unit|-u)        unit="$2";          shift 2 ;;
            --offset|-o)      start_offset="$2";  shift 2 ;;
            --interval|-i)    interval="$2";      shift 2 ;;
            --subsections|-s) subsections="$2";   shift 2 ;;
            --hours)          use_hours=true;     shift 1 ;;
            --help|-h)        echo "$usage";      return 0 ;;
            *) echo "Unknown option: $1"; return 1 ;;
        esac
    done

    if [[ -z "$project" ]]; then
        echo "Error: --project is required."
        echo "$usage"
        return 1
    fi

    if [[ (-n "$start_offset" && -z "$interval") || (-z "$start_offset" && -n "$interval") ]]; then
        echo "Error: --offset and --interval must both be provided together."
        return 1
    fi

    # Push a timestamp out of the 2200-0600 quiet window if needed
    _resolve_due_ts() {
        local ts="$1"
        local hour=$(date -d "@$ts" +%-H)
        if (( hour >= 22 )); then
            local next_day=$(date -d "@$ts + 1 day" +%Y-%m-%d)
            ts=$(date -d "${next_day} 06:00" +%s)
        elif (( hour < 6 )); then
            local same_day=$(date -d "@$ts" +%Y-%m-%d)
            ts=$(date -d "${same_day} 06:00" +%s)
        fi
        echo "$ts"
    }

    # If --subsections given, infer chapter count from it
    local -a sub_array=()
    if [[ -n "$subsections" ]]; then
        IFS=',' read -ra sub_array <<< "$subsections"
        count="${count:-${#sub_array[@]}}"
    else
        count="${count:-10}"
    fi

    # Pre-compute all due timestamps upfront when using hours
    local -a due_args=()
    if [[ -n "$start_offset" ]]; then
        if $use_hours; then
            local total_tasks=0
            if [[ ${#sub_array[@]} -gt 0 ]]; then
                for s in "${sub_array[@]}"; do (( total_tasks += s )); done
            else
                total_tasks="$count"
            fi

            local current_ts=$(date -d "now + ${start_offset} hours" +%s)
            current_ts=$(_resolve_due_ts "$current_ts")

            for (( t=0; t<total_tasks; t++ )); do
                local due_str=$(date -d "@$current_ts" +%Y-%m-%dT%H:%M)
                due_args+=( "due:${due_str}" )
                # Advance by interval from the (possibly pushed) current time
                current_ts=$(( current_ts + interval * 3600 ))
                current_ts=$(_resolve_due_ts "$current_ts")
            done
        else
            local total_tasks=0
            if [[ ${#sub_array[@]} -gt 0 ]]; then
                for s in "${sub_array[@]}"; do (( total_tasks += s )); done
            else
                total_tasks="$count"
            fi
            for (( t=0; t<total_tasks; t++ )); do
                local due_days=$(( start_offset + t * interval ))
                due_args+=( "due:today+${due_days}d" )
            done
        fi
    fi

    local due_counter=0
    local total=0

    for i in $(seq 1 "$count"); do
        if [[ ${#sub_array[@]} -gt 0 ]]; then
            local subs="${sub_array[$((i-1))]}"
            for j in $(seq 1 "$subs"); do
                local due_arg="${due_args[$due_counter]:-}"
                task add "$unit $i.$j" project:"$project" $due_arg
                (( due_counter++ ))
                (( total++ ))
            done
        else
            local due_arg="${due_args[$due_counter]:-}"
            task add "$unit $i" project:"$project" $due_arg
            (( due_counter++ ))
            (( total++ ))
        fi
    done

    echo "✓ Created $total tasks under project: '$project'"
    if [[ -n "$start_offset" ]]; then
        if $use_hours; then
            echo "  Due dates: now+${start_offset}h for first, then every ${interval}h (quiet window 2200-0600 shifts schedule forward)"
        else
            echo "  Due dates: today+${start_offset}d for first, then every ${interval}d per section"
        fi
    fi
}

```

---

## Publishing Checklist

- [ ] `cargo test` passes
- [ ] `--help` output is clean for all subcommands
- [ ] Shell completions generated (clap supports bash/zsh/fish)
- [ ] `README.md` with install instructions and usage examples
- [ ] `cargo publish --dry-run` passes
- [ ] Published to crates.io as `taskgun`
