# HTML views — visual system

Scope: `rmap render --html` and `--html --multi` only. The root `DESIGN.md` is the schema/CLI contract; this file owns the look of the static pages. Product truth for these views lives in `PRODUCT.md`.

## World

A T-card planning board. Each task is a card-stock T-card slotted into an aluminium lane; the card's **colour is its lane**. Labels are engraved black plates. HOLD and VOID are red rubber stamps. A pulled card shows its ruled index-card back (the spec).

Refuses: the dark neon kanban dashboard (the pre-0.7 "blueprint console" look).

## Tokens (`_styles.css :root`)

| Role | Light | Notes |
|---|---|---|
| Board metal | `--alu #d5d3cd`, `--alu-hi`, `--alu-lo`, `--alu-groove` | brushed grain via repeating gradient |
| Plates | `--plate #232428`, `--plate-ink`, `--plate-dim` | masthead, lane counts, phase numbers, wiring panel |
| Text on metal | `--on-board`, `-2`, `-3` | flips light in dark mode |
| Text on stock/paper | `--ink`, `-2`, `-3` | never flips — card stock stays light |
| Card stock | `--stock-ready` canary, `-active` sky, `-hold` salmon, `-waiting` buff, `-done` green, `-void` grey | set `--stock` via `.lane-*` |
| Stamp | `--stamp #b3261e` | HOLD / VOID stamps, blocks cords |
| Index card | `--paper`, `--rule-blue`, `--rule-red` | detail panel back; 26px rule pitch, every line box in `.spec` is 26px |

Dark mode (`prefers-color-scheme`) is the same board after hours: dark anodized metal, stocks keep their hue.

## Type

- Labels / plates: `--f-label` (DIN Condensed → Bahnschrift → Arial Narrow), uppercase, tracked 0.06–0.16em.
- Numerals: `--f-num` (DIN Alternate), tabular. IDs, Eff, counters.
- Reading text: system UI stack. Code, paths, assignees: `--f-code`.
- No web fonts (offline, single-file constraint).

## Components

- **T-card** (`.task-card` on boards, `.rack-card` in racks): id numeral, Eff box (tier: `hi` filled ink, `mid` bold, `lo` plain, `low` dashed), `+N` unlocks, 3-line title, one lane-specific note (hold reason / waits-on / landing ref), marker tags + assignee, stem via `::after`.
- **Slot rail**: recessed channel with two lips; empty rail prints a lane-specific empty line.
- **Counter window**: black window, tabular digits, lane-colour strip under it.
- **Switch**: physical key with a lamp in the lane colour; `aria-pressed` is the state.
- **Composition strip**: lane segments sized by count (masthead, phase header, repo rail).
- **Detail panel**: pulled card — stock-coloured head, index-card back; slides in with `--ease-out`, the source card lifts and gets an ink outline (`.is-pulled`). The one authored motion.

## Layout rules

- Dispatch racks (Ready / Active / Hold) always come before phase boards.
- Phase boards show only non-empty lanes; a Done lane with >6 cards spans the row and files cards in a grid. Phases with nothing open start folded.
- ≤760px: everything single-column, racks show top 4, panel goes full-screen.

## Invariants (agent contract — do not restyle away)

`rmap-data` / `rmap-relations` islands; `.task-card` with `data-id` `data-status` `data-eff` `data-markers` `data-depends-on` `data-phase`; `.dag-node[data-id]`; `.phase[data-phase-status]`; `.repo-row[data-slug][data-name][data-has-rel]`. Rack cards must NOT carry `.task-card`.
