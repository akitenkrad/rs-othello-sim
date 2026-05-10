# docs/assets/

This directory holds GIF demos referenced from the root README and from
`docs/tui-guide.md`. Three GIFs are expected:

- `tui-play.gif` — Play mode (Human vs Human)
- `tui-replay.gif` — Replay mode (record playback)
- `tui-observe.gif` — Observe mode (AI vs AI with Evaluator overlay)

## How to capture

### macOS

Record a 5-second TUI session with `Cmd+Shift+5`, save as `out.mov`,
then convert:

```bash
ffmpeg -i out.mov -vf "fps=10,scale=800:-1" -loop 0 tui-observe.gif
```

### Cross-platform (asciinema + agg)

```bash
asciinema rec out.cast
agg --font-family "Menlo" --rows 24 --cols 80 out.cast tui-observe.gif
```

Aim for ≤5 s and ≤1 MB per GIF.
