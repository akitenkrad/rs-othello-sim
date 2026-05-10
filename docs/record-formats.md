# Record Formats

`rs-othello-sim` reads and writes several record formats, each
optimised for a different use case. The neutral in-memory
representation is `othello_io::GameRecord`; every reader produces and
every writer consumes that type.

| Format | Read | Write | Use case |
|---|---|---|---|
| Self-described JSON | yes | yes | Default. Carries all metadata + per-move timestamps |
| GGF (Othello subset) | yes | yes | Interop with existing Othello tools |
| WTHOR (`.wtb`) | yes | no | Importing public Othello databases |
| JSONL event log | append | append | Streaming logs (one event per line) |

The conversion subcommand `othello-cli convert` accepts any reader
format and emits any writer format; see
[`docs/cli-usage.md#convert--record-format-conversion`](cli-usage.md#convert--record-format-conversion).

## Self-described JSON

The default format is a single JSON object with `schema_version`,
`metadata`, and a `moves` array. Schema version is currently `"1.0"`.

```json
{
  "schema_version": "1.0",
  "metadata": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "started_at": "2026-05-09T15:30:00.000+09:00",
    "ended_at":   "2026-05-09T15:30:42.123+09:00",
    "board_size": { "rows": 8, "cols": 8 },
    "players": {
      "black": { "name": "MctsPlayer",   "params": { "simulations": 1000 } },
      "white": { "name": "RandomPlayer", "params": { "seed": 42 } }
    },
    "result": { "winner": "Black", "score": { "black": 38, "white": 26 } },
    "engine_version": "rs-othello-sim 0.1.0"
  },
  "moves": [
    { "n": 1, "side": "Black", "move": { "Place": { "row": 2, "col": 3 } },
      "ts": "2026-05-09T15:30:00.123+09:00" },
    { "n": 2, "side": "White", "move": { "Place": { "row": 2, "col": 2 } },
      "ts": "2026-05-09T15:30:00.456+09:00" }
  ]
}
```

Notes:

- `move` is `{ "Place": { "row": R, "col": C } }` for placements and
  `"Pass"` (a bare string) for passes.
- Timestamps follow RFC 3339 with millisecond precision and timezone
  offset.
- The reader rejects records whose major schema version differs from
  its own.

## GGF (Othello subset)

GGF (Generic Game Format) is a long-standing community format. We
implement the Othello subset:

```
(;GM[Othello]PC[rs-othello-sim]DT[2026-05-09]
 PB[MctsPlayer]PW[RandomPlayer]
 RE[+12]
 BO[8 ---------------------------O*------*O--------------------------- *]
 B[D3//1.234]W[C5//0.456]B[E3//1.111]…;)
```

Recognised tags:

| Tag | Meaning |
|---|---|
| `GM` | Game name (`Othello` only) |
| `PB` / `PW` | Black / White player name |
| `PC` | Place (the engine that produced the record) |
| `DT` | Date (free-form) |
| `RE` | Result (`+N`, `-N`, `=` for draw) |
| `BO` | Initial board (`size board-string side-to-move`) |
| `B[…]` / `W[…]` | Move (coordinate optionally followed by `//time`) |

We do not emit comments (`C[…]`) or variations. The reader silently
skips tags it does not understand.

## WTHOR (`.wtb`) — read only

WTHOR is a fixed-length binary archive used by major Othello
databases (e.g. the Frédéric Donninger collection). Layout:

- 16-byte header (year, board count, etc.).
- 68 bytes per game: 8 bytes of metadata + 60 bytes of moves.
- Each move is one byte: `(row - 1) * 10 + col`, 1-indexed; `0` is the
  null move (unused suffix of the 60-byte buffer).

The reader is read-only; convert to JSON or GGF first if you need to
edit a record. The `inspect` subcommand can summarise totals across
all games in a `.wtb` archive without unpacking it.

## JSONL event log

`othello_io::JsonlLogger` writes one JSON object per line, optimised
for streaming and tail-friendly viewing. Every batch self-play
produces one of these alongside per-game records.

```jsonl
{"event":"game_start","ts":"2026-05-09T15:30:00.000+09:00","game_id":"550e…","board_size":[8,8],"players":{"black":"Mcts","white":"Random"}}
{"event":"move","ts":"…","game_id":"550e…","n":1,"side":"Black","move":{"Place":[2,3]},"stones":{"black":4,"white":1},"legal_count":3}
{"event":"move","ts":"…","game_id":"550e…","n":2,"side":"White","move":{"Place":[2,2]},"stones":{"black":3,"white":3},"legal_count":4}
{"event":"pass","ts":"…","game_id":"550e…","n":30,"side":"Black"}
{"event":"game_end","ts":"…","game_id":"550e…","winner":"Black","stones":{"black":38,"white":26},"moves_total":60}
```

`move` events use a compact array `[row, col]` rather than the
`{Place: {row, col}}` object form found in the JSON record. The trade
is intentional: JSONL is designed to be streamed and aggregated, so
shorter line lengths matter; record JSON is designed to be archived
and round-tripped, so the schema is self-describing.

Event types: `game_start`, `move`, `pass`, `game_end`. Each line has
`event`, `ts`, and `game_id`; the rest depends on the type.

## Round-trip examples

WTHOR archive → individual JSON files:

```bash
othello-cli convert \
  --input archive.wtb --input-format wthor \
  --output-format json --output-dir converted/
```

GGF → JSON:

```bash
othello-cli convert \
  --input game.ggf --input-format ggf \
  --output-format json --output game.json
```

JSON → GGF (e.g. to feed into another Othello tool):

```bash
othello-cli convert \
  --input game.json --input-format json \
  --output-format ggf --output game.ggf
```

JSONL is not part of `convert`'s output choices because it is an
event log, not a record archive. Use `selfplay --jsonl-log PATH` to
emit one alongside individual JSON records.

## See also

- [Self-play & batch runs](self-play.md) for the directory layout
  produced by `--log-dir auto`.
- [Tools (visualize / analyze / TB)](tools-visualize.md) for the
  Python utilities that consume both formats.
- `crates/othello-io/src/` for the reader / writer source.
