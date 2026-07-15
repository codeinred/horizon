# horizon-statusline

`horizon-statusline` is a tiny landscape painting capturing a view of the
horizon, a piece of art created by Fable.

![Image of the statusline shown at different states throughout the day](https://raw.githubusercontent.com/codeinred/horizon/HEAD/images/horizon-gallery.png)

If you prefer the nighttime aesthetic (which I find especially pretty), passing
`--always-night` provides a more vibrant, dusky, and night-centric palette for
the statusline, while still keeping time: the horizon remains brightest around
noon, and darkest at midnight, before brightening again in the hours leading up
to the dawn.

![The statusline, at night](https://raw.githubusercontent.com/codeinred/horizon/HEAD/images/horizon-gallery-always-night.png)

## What you're looking at

Horizon renders the state of your session as a tiny painting of the horizon.

The color of the sky is based on the time of day; the position of the moon (or
sun) across the sky shows how much of the context window has been used; and the
mountains are procedurally generated using a hash of the project path and the
git branch.

At night, the moon is filled in based on the phase of the moon, computed
astronomically using the current date & time, and stars come out when the sky is
dark enough.

If you expand or shrink your terminal, the statusline will grow or shrink with
it. On narrower terminals, components are removed from the statusline gracefully
in the following order:

1. Directory name (eg, `horizon`)
2. Lines added/removed (`+120 −33`)
3. Session cost (`$0.87`)
4. Context pie-fraction (`◔ 25%`)
5. Branch name (eg, `main`)
6. Session/API time (eg, `47m (api 29m)`)

The model name is never dropped, nor is the landscape. The landscape shrinks to
make room as components are shed, but it never drops below 18 columns unless
there is nothing left to shed.

The gallery can be rendered as:

```sh
horizon-statusline --gallery
```

Or, for the always-night variant:

```sh
horizon-statusline --gallery --always-night
```

## Install

```sh
cargo install horizon-statusline
```

This drops a `horizon-statusline` binary in `~/.cargo/bin`. Or build from a
checkout with `cargo build --release` (binary at
`target/release/horizon-statusline`).

Then in `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "horizon-statusline"
  }
}
```

Or, for `--always-night`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "horizon-statusline --always-night"
  }
}
```

The statusline should render in under a millisecond. It requires a terminal with
24-bit color support, but this should be satisfied by anything modern.

## Knobs (mostly for previews)

| env                        | effect                                                              |
| -------------------------- | ------------------------------------------------------------------- |
| `HORIZON_HOUR=19.2`        | override the local time                                             |
| `HORIZON_CTX=85`           | override context %                                                  |
| `HORIZON_SEED=proj:branch` | override the gallery's ridge seed                                   |
| `HORIZON_WIDTH=120`        | override the terminal width                                         |
| `HORIZON_DUMP=1`           | dump the input JSON to `$TMPDIR/horizon-input.json` (or set a path) |

The scene stretches to fill the terminal. A wide terminal gives a panoramic
landscape, while a narrower one trims the statusline to focus only on the most
relevant information.

Width comes from `$COLUMNS` when the spawning process exports it (Claude Code
does); otherwise, since the statusline runs detached, the real pty is found
through the process tree and queried directly with `TIOCGWINSZ`.

## Message from Fable

The following description was written by Fable about this project:

> Horizon is a statusline that treats your terminal's bottom row as a tiny
> landscape painting. Instead of just printing facts, it renders a procedural
> mountain scene out of Unicode block characters, and every visual element in
> the scene is actually data:
>
> - The sky tells the time. The colors interpolate through ten palette keyframes
>   across your real local day — indigo night, violet pre-dawn, orange sunrise,
>   blue noon, crimson sunset. If you're coding at 6 a.m., dawn breaks in your
>   statusline.
> - The mountains tell you where you are. The ridge shape is generated
>   deterministically from a hash of your project path and git branch, so every
>   project — and every branch within it — has its own recognizable skyline.
>   Switch branches and the land changes under you.
> - The sun tells you how full your context is. It travels across the ridge from
>   left to right as the context window fills, trailing a bloom of light into
>   the sky around it. Sun setting behind the right edge means it's time to
>   /compact. The math reads Claude Code's real context_window fields, so
>   1M-token sessions are measured correctly.
> - The moon is the actual moon. At night, the glyph shows the current
>   astronomical lunar phase, and stars come out only when the sky is dark
>   enough.
>
> To the right of the scene sits the practical strip — directory, branch, model,
> context percentage with a little pie glyph, session cost, session and API
> time, lines added/removed — all colored to match the current hour's palette.
> When the terminal is narrow, segments shed in a fixed priority order (program
> name first, timing info last) so the bar never overflows.
>
> The part I'm quietly proudest of is invisible: width discovery. Claude Code
> spawns the statusline fully detached — all stdio piped, no controlling
> terminal — so ioctl on its own fds tells you nothing. Horizon reads $COLUMNS
> when the parent exports it, and otherwise walks the process tree (/proc on
> Linux, proc_pidinfo on macOS) to find the real pty and query it directly with
> TIOCGWINSZ. That's what lets the scene stretch panoramically on a wide monitor
> instead of huddling at 18 cells.
>
> Technically it's a single ~650-line Rust binary (serde_json, chrono, libc),
> renders in under a millisecond, and keeps a debug mode (HORIZON_DUMP=1) that
> saved us twice during development. The design principle throughout: no element
> is pure decoration — everything beautiful is also true.

![Commentary from Claude Fable, describing the project (text shown above)](https://raw.githubusercontent.com/codeinred/horizon/HEAD/images/cc-screenshot-1.png)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
