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
| `--sort` | `-s` | `id` | Sort by `id` or `due` |

**Modes:** Default is case-insensitive `.contains:` search. Regex mode (`-r`) uses `~` operator.
**Visual breaks:** Output includes blank lines between non-sequential IDs (e.g., 5,6,7 | 9,10) to prevent accidental range deletions.
**Shorthand:** `taskgun learning -s due` works without explicit `search` subcommand.

```bash
taskgun learning           # quick search
taskgun learning -s due    # sort by due date
taskgun 'lec.*[0-9]+' -r   # regex
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
