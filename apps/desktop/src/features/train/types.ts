// Dual N-Back session record, exactly as contract C6 (docs/tutor/CONTRACTS.md). Spec: docs/tutor/NBACK.md.
// In "transfer" mode, `position` holds the shape index (0-7) and `letter` the tone label.

export type Metrics = {
  hits: number
  misses: number
  false_alarms: number
  correct_rejections: number
  d_prime: number
  c: number
}

export type Trial = {
  i: number
  position: number | null
  letter: string | null
  pos_match: boolean
  let_match: boolean
  lure: boolean
  pos_response: boolean
  let_response: boolean
  pos_rt_ms: number | null
  let_rt_ms: number | null
  intended_onset: number // ms on the session clock
  actual_onset: number // ms on the session clock
}

export type Block = {
  index: number
  n: number
  void: boolean
  void_reason: string | null
  trials: Trial[]
  metrics: { position: Metrics | null; letter: Metrics | null }
  next_n: number
}

export type Mode = "dual" | "position" | "letter" | "transfer"

export type Settings = {
  trial_ms: number
  stimulus_ms: number
  blocks: number
  up_max_errors: number
  down_min_errors: number
  lures: boolean
  in_trial_feedback: boolean
  keys: { position: string; letter: string }
}

export type Checkin = {
  expectation: number | null
  sleep_hours: number | null
  exercise: boolean | null
  fatigue: number | null
  strategy: string | null
}

export type NBackSession = {
  id: string
  started_at: number // epoch seconds
  ended_at: number
  mode: Mode
  seed: number
  settings: Settings
  blocks: Block[]
  comparable: boolean
  timing: { output_latency: number | null; base_latency: number | null; calibrated: boolean }
  checkin: Checkin
}

export type NBackSummary = {
  id: string
  started_at: number
  mode: string
  blocks: number
  void_blocks: number
  start_n: number
  max_n: number
  mean_n: number
  d_prime: { position: number | null; letter: number | null }
  comparable: boolean
}
