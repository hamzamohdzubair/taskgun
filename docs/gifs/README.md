# GIF Placeholders for taskgun README

This directory contains demo GIFs for the taskgun README.

## Required GIFs

### 1. `create-command.gif`
**Shows:** Bulk task creation with scheduling
- Command: `taskgun create "Deep Learning" -p 5 --offset 2d --interval 7d`
- Expected output: 5 tasks created with due dates
- Duration: ~5-10 seconds

### 2. `search-command.gif`
**Shows:** Instant keyword search with visual formatting
- Command: `taskgun learning -s urg`
- Expected output: Colored task list with urgency sorting
- Duration: ~5-10 seconds

### 3. `due-command.gif`
**Shows:** Date filtering with pattern matching
- Commands to show:
  - `taskgun d1` (today's tasks)
  - `taskgun 7d` (next 7 days)
  - `taskgun w1` (this week)
- Duration: ~10-15 seconds

### 4. `plan-command.gif`
**Shows:** Task planning sequences
- Commands to show:
  - `taskgun plan 5,9,1,3` (set plan values)
  - `taskgun plan` (display planned tasks)
- Duration: ~10-15 seconds

## Recording Tips

1. Use a terminal with good color support (e.g., iTerm2, Alacritty)
2. Set terminal size to ~100x30 for readability
3. Use a tool like `asciinema` or `terminalizer` for recording
4. Keep each GIF focused on one command/feature
5. Include clear command input and colored output
6. Add a 1-2 second pause at the end to show final state

## Converting to GIF

If using asciinema:
```bash
# Record
asciinema rec demo.cast

# Convert to GIF
agg demo.cast demo.gif
```

If using terminalizer:
```bash
# Record
terminalizer record demo

# Render
terminalizer render demo
```
