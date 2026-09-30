# Dual N-Back

Evidence: `docs/tutor/research/nback.md`.
- Training reliably improves the n-back task and similar n-back tasks (g 0.62 on untrained n-back, Soveri
  2017).
- Transfer to other working-memory tests is small (g 0.24).
- Transfer to reasoning or academic outcomes is not established once active controls and bias are
  handled.

The module is therefore a well-built, optional, honestly labelled skill practice with a personal experiment
that tests any broader benefit on this learner.

## Rules

- Copy says: "This trains the n-back task. Transfer to other skills is small or unproven; the Experiment
  tab tests it on you."
- It never claims IQ, "brain power", working-memory age, learning speed or grades.
- n-back never drives FSRS, question difficulty, lesson gating or curriculum. The tutor may mention it only
  as an optional warm-up after due reviews.
- Dose default: 15-20 minutes, up to 3 sessions a week, a soft 60 min/week reminder. Adjustable. No streaks,
  XP, badges, leaderboards or multipliers.
- Adaptation is always visible: the level and rule are shown on screen and logged per session.

## Task

- **Modes:** dual (default), single position, single letter. Start at n = 1.
- **Stimuli:**
  - Position: 8 cells of a 3x3 grid (centre unused).
  - Letters: 8 non-rhyming consonants C H K L Q R S T, as pre-rendered clips.
- **Timing:** 500 ms stimulus, 3,000 ms trial, 20 + n trials per block.
- **Targets:** about 25% matches per modality (about 6 per stream), including dual matches.
- **Lures:** off by default; optional, and tagged in the log.
- **Generation:** a seeded generator; the seed is stored.
- **Adaptation (Jaeggi/Thompson, per block):** n goes up when each stream has 2 or fewer errors (misses
  plus false alarms) and down when either stream has more than 5. Thresholds are configurable.
- **Session:** 12-20 blocks (default 15) with 10 s rests.
- **Keys:** A = position match, L = letter match. Remappable. Buttons are shown too.
- **Feedback:** a block-end summary by default. In-trial feedback is opt-in and marks the session
  non-comparable.
- **Void rule:** a block is void on focus loss, the window hiding, or an audio device change.
- **After the session:** one question, "Which strategy did you use?".

## Scoring

- **Per modality:** hits, misses, false alarms, correct rejections.
- **d'** and criterion **c**, with the log-linear correction (add 0.5 to each count, 1 to each total).
- **Raw counts** are always shown next to d' and c.
- **Response time** is logged as a within-person trend only.

## Timing engineering

- One AudioContext, created on the Start click.
- Letter clips are pre-rendered offline, committed as small WAVs, decoded at startup, trimmed of silence
  under 450 ms and loudness-normalised. They are generated once with Windows SAPI (offline, free) or later
  with ElevenLabs. There are no live TTS calls and no `speechSynthesis` in the trial loop.
- Lookahead scheduling on the audio clock; visual onset on the frame nearest the scheduled time.
- Each trial logs intended vs actual onsets, plus `outputLatency` and `baseLatency`.
- A calibration screen (audio/visual sync check) is shown; timing is labelled "unmeasured" until it has run.

## Measurement

- **Training curve:** trained task only. Each session is a point; rolling median with an uncertainty band;
  level, hit and false-alarm rates, and d' per modality. It carries the line: "n-back scores are noisy and
  do not measure intelligence."
- **Transfer check:** every 4 weeks, labelled "measurement". A single n-back with different stimuli
  (shapes instead of grid positions, tones instead of letters), fixed n (the current level minus 1) and
  fixed settings. Ungamified.
- **Experiment card (opt-in, later phase):**
  - 3-5 baseline probe sessions: backward digit span, a short operation span, and open reasoning items once
    their licences are checked (ICAR, MaRs-IB).
  - A locked plan, an active control (adaptive visual search), and a reliable-change analysis.
  - The minimum detectable change is shown first (about 18 IQ points for one matrix probe), with a
    statement that a null result is expected.
- **Log:** expectation (one rating per session), sleep hours, exercise and fatigue.

## Architecture

- `apps/desktop/src/features/train/`:
  - React screen: task, settings, calibration, history.
  - TypeScript engine: generator, scoring, adaptation, audio scheduler. All pure functions are unit-tested.
- Storage in the Rust shell (contract C6): SQLite in the app data directory, per-trial rows, CSV export.
- Sidebar section "Train", between Progress and Plan.

## Not building

- IQ, brain-age or composite scores.
- Triple, quad or arithmetic modes.
- Streaks.
- Stimulation.
- n-back-styled subject content.
- Any code or assets from Brain Workshop (GPL-2.0, ideas only).
