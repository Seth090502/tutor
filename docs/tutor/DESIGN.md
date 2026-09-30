# Tutor design direction

Status: draft from the gallery study (2026-09-30). The subject worlds will be revised after the pedagogy
research lands (evidence on per-subject theming and context-dependent memory).

Source: the 100 Sonnet 5.5 HTML pieces at miaai-lab/Sonnet-5.5-100-HTML-Files, studied by seven agents. That
repository has no licence, so we take design ideas only and write our own code. Piece numbers are cited so
each decision can be traced back.

## Principles

1. **A calm instrument.** Long study sessions come first. Typography and hairlines carry the look; effects
   stay at the edges and never sit behind text at full strength (034 scrims, 017 reading type).
2. **One accent per world.** Each subject has its own accent and ambient tint. Everything else is neutral
   (002, 005, 100).
3. **Numbers are the heroes.** Large light tabular numerals over small tracked mono labels, divided by
   hairlines (024, 005, 072, 048).
4. **Motion is earned.** Motion responds to an action or a state change. Nothing loops behind reading text.
   Reduced motion stops loops rather than slowing them (002 and 014 got this wrong).
5. **The GPU is busy.** The 5090 runs other workloads, so the only live WebGL is the Orb. Everything else is
   CSS, SVG or Canvas2D that repaints only when something changes and pauses when hidden (053, 010, 098).

## Tokens

Neutral base, warm rather than grey (017, 005):

| Token | Light ("paper") | Dark ("lamplight") |
|---|---|---|
| background | `#f5f0e6` | `#1a1714` |
| card | `#fbf7ee` | `#221d18` |
| ink (text 1) | `#1a1714` | `#f1eadb` |
| text 2 / text 3 | ink at .72 / .58 | ink at .72 / .58 |
| hairline / strong hairline | ink at .12 / .28 | ink at .15 / .32 |

- Text 3 must stay at or above 4.5:1. The gallery's .46 tiers fail.
- Labels are never smaller than 11px.
- Colour never carries meaning alone: pair it with a word or a mark.

### Subject worlds

`data-world` on the root swaps a small token set. `@property`-registered colours cross-fade over 0.9 to
1.2 s (057, 005, 061). With no subject selected, the world is neutral.

| World | Accent, dark / light | Ambient | Motif (static SVG, drawn once) | Draws on |
|---|---|---|---|---|
| Math | periwinkle `#9aa6f0` / ink blue `#2f3f8f` | slate washes | times-table cardioid chords, ruler ticks | 074, 098, 025 |
| English | ember `#e4573a` / vermilion `#c8402a` | paper-and-ink washes | book spines, drop cap | 017, 081, 064 |
| Science | cyan `#56d4e0` / teal `#1a7f86` | indigo washes | orbit rings, moon phases | 014, 091, 045 |
| Chemistry | brass `#dcb86e` / bronze `#8c6218` | cream/walnut washes | hexagon lattice, molecule sprite | 084, 040 |

- The four hues are spaced around the colour wheel.
- Before shipping, check equal OKLCH lightness and AA contrast for each accent, using a 060-style check
  that stays out of the app.

### Type

- **Geist** (UI), **Geist Mono** (labels and numerals), plus one bundled OFL serif for display and reading.
  The candidate is **Newsreader** (variable, optical sizes, true italic).
- The gallery's Iowan, Palatino and Bodoni stacks don't exist on Windows, so every font is bundled.
- Display numerals: Geist 200, `tabular-nums`, tight tracking (-0.03 to -0.05em).
- Eyebrows: Geist Mono 11px, uppercase, +0.18em, paired with a 24px hairline (066).
- Reading surfaces (Plan, explanations, misconceptions): serif 17 to 18px, line-height 1.6, about 68ch,
  with `text-wrap: pretty` (017, 064).
- Math symbols and emphasis use serif italic (026).

## Components

| Component | Decision | From |
|---|---|---|
| Sidebar | Icon-and-label rail; the active item is a spring-driven pill (stiffness 260, damping 20); a subject pill switches world | 048, 065, 053 |
| Stat row | Hairline-divided cells: big light numeral, mono caps label, small delta chip | 005, 024, 048, 072 |
| Charts | Hand-built SVG kit: 2px lines, 10% area wash, labels at line ends, dim others on hover, keyboard scrubber, chart/table toggle, draw-on once (under 1.2 s). Flat fills with a darker same-hue outline. Train/test whiskers as range bars | 012, 048, 010, 053 |
| Answer keys A-D | Keycaps: contact shadow, skirt, top face, about 4px travel with a spring press. Bound to the A-D and 1-4 keys. The chosen answer stays latched with an LED dot. The result is shown by a tint plus a mark | 061, 020 |
| Question card | Hover-only holo sheen and glare: transform-only layers in static blend containers, 6 to 8 degree tilt, glare opacity at most .4. Text sits outside the blend stack. Flips to explanation and misconception. The frame ring tints by mastery | 029 |
| Orb stage | Ambient gradient blobs (transform drift, 60 to 120 s, static under reduced motion). A local scrim behind the transcript. Dark-glass state chips with a state word. The page tint follows the Orb state | 014, 002, 034, 051 |
| Progress | Due/overdue gauge (21-tick arc, spring needle), 30-day moon-phase discs (lit fraction = share of the daily goal), mastery sunburst (subject > topic > skill) with a table view | 005, 091, 040 |
| Library | Book-spine shelf (height = pages, colour = subject, 2D lean on hover). Filter chips with counts. The ingest log is typed mono lines with a spring check-stamp | 081, 071 |
| Plan | Editorial reading column, progress hairline, sticky outline with scroll-spy | 017, 064 |
| Settings | Dot-leader service rows with an LED plus a state word | 051 |
| Session chrome | Session timer. After 5 s idle, chrome dims to 40% over 1.8 s and returns on hover or focus | 079 |

## Motion and performance

- One shared spring ticker that sleeps when idle (065).
- Transforms and opacity only.
- Easing `cubic-bezier(.2,.8,.2,1)` for entrances and `cubic-bezier(.3,1.35,.4,1)` for toggles.
- A "lite by default" governor: richer ambients turn on only after 20 frames under 24 ms, and step back above
  52 ms. It is exposed as `data-quality` on the root (053).
- No `backdrop-filter` over anything that animates. No full-screen per-frame canvases or shaders besides
  the Orb. Grain is a pre-generated tile, not a filter.
- Every loop pauses on `visibilitychange`. `prefers-reduced-motion` removes ambient loops and keeps state
  transitions (0.2 to 0.5 s).
- Sound is off by default. Nothing plays during a voice call, since it would leak into the microphone
  (020, 056).

## Not adopting

- Live shaders: 002, 014, 022, 066.
- Full-screen simulations: 026, 034 live, 090.
- Custom cursors, scroll-jacking, text effects that blur or rotate, and difference-blend type.
- Skeuomorphic chrome beyond the answer keys.
