# horizon

A tiny landscape painting that lives in your Claude Code statusline.

```
sunrise   ▃▄▆▆●▃▂▁▁▂▂▁ ▁▃▆▆▅  statusline · ⎇ main · ✦ Fable 5 · ◔ 25% · $0.87 · 47m
night     ▃▄▆▆▅▃▂▁▁▂▂▁·▁▃▆○▅  statusline · ⎇ main · ✦ Fable 5 · ● 95% · $0.87 · 47m
```

(render it in color: `cargo run --release -- --gallery`)

## What you're looking at

- **The sky is your clock.** The palette interpolates through ten keyframes of
  your real local time — indigo night, violet pre-dawn, orange sunrise, blue
  noon, amber golden hour, crimson sunset, purple dusk. Work at 6 a.m. and
  you'll watch the dawn come up in your terminal.
- **The moon is real.** Night scenes show the actual current lunar phase
  (○ ◔ ◐ ◕ ●), computed astronomically. Stars only come out when the sky is
  dark enough, and they resettle each day.
- **The mountains are yours.** The ridge is grown deterministically from a
  hash of your project path and git branch. Every project — and every branch —
  has its own skyline. You'll learn to recognize where you are by the shape of
  the land.
- **The sun is your context window.** The sun (or moon) travels across the
  ridge as your context fills, trailing a bloom of light that tints the sky
  around it. When it sets behind the right edge, your day is done: time to
  `/compact`.
- The quiet part on the right: directory, branch, model, context %, session
  cost, session time, lines added/removed — in colors matched to the hour.

## Install

```sh
cargo build --release
```

Then in `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/path/to/statusline/target/release/statusline"
  }
}
```

Renders in under a millisecond. Requires a terminal with 24-bit color
(anything modern).

## Context sources

Context usage is read from the statusline JSON's `context_window` object
(Claude Code ≥ 2.1: `current_usage` over `context_window_size`, so 1M-context
sessions are measured correctly, with `used_percentage` as a backstop).
On older versions it falls back to the last usage entry in the session
transcript against a 200k window.

## Knobs (mostly for previews)

| env | effect |
| --- | --- |
| `HORIZON_HOUR=19.2` | override the local time |
| `HORIZON_CTX=85` | override context % |
| `HORIZON_SEED=proj:branch` | override the gallery's ridge seed |
| `HORIZON_WIDTH=120` | override the terminal width |
| `HORIZON_DUMP=1` | dump the input JSON to `$TMPDIR/horizon-input.json` (or set a path) |

The scene stretches to fill the terminal — width is read from the
controlling tty (`/dev/tty`), since Claude Code pipes stdout. Wide
terminal, panoramic landscape.
