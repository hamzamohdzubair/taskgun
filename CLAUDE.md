# taskgun — A gun to shoot tasks for our Taskwarrior

A Rust CLI extending Taskwarrior with bulk operations, smart scheduling, and search.

## Stack & Goals

```toml
clap = { version = "4", features = ["derive"] }   # CLI + help
chrono = "0.4"                                      # time + quiet window
anyhow = "1"                                        # errors
```

- Published on crates.io as `taskgun`
- Subcommand interface with clean `--help` output
- Zero dependencies beyond Taskwarrior

---

## Commands

### `taskgun create` — Bulk task generation ✓

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--project` | `-p` | required | Project name or comma-separated subsections (e.g., "2,3,1") |
| `--count` | `-n` | 10 | Number of tasks (inferred from subsections if provided) |
| `--unit` | `-u` | "Part" | Task name prefix |
| `--offset` | `-o` | — | Delay before first task (e.g., "5d", "2h", "30m") |
| `--interval` | `-i` | — | Gap between tasks (e.g., "7d", "6h", "15m") |

**Naming:** `Video 1`, `Video 2` or `Video 1.1`, `Video 1.2` with subsections
**Scheduling:** Day mode (7d) uses simple day offsets; Hour/minute mode (2h, 30m) respects **quiet window 22:00-06:00** (pushes to 06:00 and cascades intervals from there). Mixed units supported.

```bash
taskgun create "Deep Learning" -p 5 --offset 5d --interval 7d
taskgun create "Deep Learning" -p "2,3,1" --offset 2h --interval 30m -u "Lecture"
```

---

### `taskgun search` — Keyword search ✓

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `keyword` | — | required | Search term or regex pattern |
| `--regex` | `-r` | false | Enable case-sensitive regex mode |
| `--sort` | `-s` | `urg` | Sort by `urg` (urgency), `id`, or `due` |

**Modes:** Default is case-insensitive `.contains:` search. Regex mode (`-r`) uses `~` operator.
**Sorting:** Defaults to urgency descending (most urgent first). Can sort by ID or due date.
**Visual breaks:** Output includes blank lines between non-sequential IDs (e.g., 5,6,7 | 9,10) to prevent accidental range deletions.
**Colors & formatting:** Alternating row backgrounds, color-coded due dates, and flexible column widths (auto-detects terminal width).
**Shorthand:** `taskgun learning -s due` works without explicit `search` subcommand.

```bash
taskgun learning           # quick search (sorted by urgency)
taskgun learning -s due    # sort by due date
taskgun learning -s id     # sort by ID
taskgun 'lec.*[0-9]+' -r   # regex
```

---

### `taskgun due` — Date-based filtering ✓

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `pattern` | — | required | Date pattern (see below) |
| `--sort` | `-s` | `urg` | Sort by `urg` (urgency), `id`, or `due` |

**Patterns:**
- **Days:** `d1` (today), `d2` (tomorrow), `d3` (day after), `2d` (next 2 days), `7d` (next 7 days)
- **Weeks:** `w1` (this week), `w2` (next week only), `2w` (next 2 weeks)
- **Months:** `m1` (this month), `m2` (next month only), `2m` (next 2 months)
- **Shortcuts:** `tod` (today), `tom` (tomorrow), `week` (this week)

**Letter-first vs number-first:**
- `d1` → today only (specific day)
- `1d` → today + overdue (includes backlog)
- `d2` → tomorrow only (specific day)
- `2d` → next 2 days (range: today + tomorrow + overdue)
- `w2` → next week only (specific week)
- `2w` → next 2 weeks (range: this week + next week)

**Shorthand:** All patterns work without `due` subcommand (e.g., `taskgun d1`, `taskgun 7d`, `taskgun tod`)

**Sorting:** Defaults to urgency descending. Visual breaks and formatting same as search.

```bash
taskgun due d1             # tasks due today only
taskgun d1                 # shorthand (same as above)
taskgun 1d                 # today + overdue (includes backlog)
taskgun tod                # alias for d1
taskgun tom                # alias for d2 (tomorrow)
taskgun 7d -s due          # next 7 days + overdue, sorted by due date
taskgun week               # alias for w1 (this week)
taskgun 2w                 # next 2 weeks
```

---

### `taskgun modify` — Bulk modification (planned)

```bash
taskgun modify --project "Deep Learning" --due-shift +2d
```

---

## Implementation

**Architecture:**
- Shell out to `task` via `std::process::Command`
- `create`: one `task add` per task
- `search`: build filter + `task list` with custom sort
- Quiet window: `h >= 22 || h < 6` → push to 06:00, cascade intervals

**Structure:**
```
src/
├── main.rs           # clap dispatch
├── scheduling.rs     # time + quiet window
├── skip.rs           # skip window parsing
├── taskwarrior.rs    # task binary checks
└── commands/
    ├── create.rs     # bulk generation
    ├── search.rs     # keyword search with ID breaks
    └── modify.rs     # (planned)
```
