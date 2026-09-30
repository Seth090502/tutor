import { describe, expect, it } from "vitest"

import { appendLog, domainColor, ratio, sortedRows, type Job } from "./logic"

const job = (over: Partial<Job> = {}): Job => ({
  job_id: "3f2a91c4-0000-4000-8000-000000000000",
  kind: "ingest",
  state: "queued",
  progress: { done: 0, total: 0 },
  started_at: null,
  finished_at: null,
  error: null,
  ...over,
})

describe("appendLog", () => {
  it("adds one line per state change and ignores repeated polls", () => {
    let log = appendLog(null, job(), 1)
    log = appendLog(log, job(), 2)
    log = appendLog(log, job({ state: "running" }), 3)
    log = appendLog(log, job({ state: "running", progress: { done: 4, total: 9 } }), 4)
    expect(log.lines.map((l) => l.state)).toEqual(["queued", "running"])
    expect(log.lines[0].detail).toBe("ingest 3f2a91c4")
  })

  it("starts a fresh log for a different job", () => {
    const first = appendLog(null, job({ state: "done" }), 1)
    const next = appendLog(first, job({ job_id: "other", state: "queued" }), 2)
    expect(next.jobId).toBe("other")
    expect(next.lines).toHaveLength(1)
  })

  it("reports the error on failure and elapsed time on completion", () => {
    const failed = appendLog(null, job({ state: "failed", error: "disk full" }), 1)
    expect(failed.lines[0].detail).toBe("disk full")
    const done = appendLog(null, job({ state: "done", started_at: 100, finished_at: 112.4 }), 1)
    expect(done.lines[0].detail).toBe("finished in 12s")
  })

  it("leaves out an implausible elapsed time", () => {
    const done = appendLog(null, job({ state: "done", started_at: 1, finished_at: 1_000_000 }), 1)
    expect(done.lines[0].detail).toBe("ingest 3f2a91c4")
  })
})

describe("helpers", () => {
  it("sorts rows by value, then label", () => {
    expect(sortedRows({ b: 2, a: 2, c: 5 }).map((r) => r.label)).toEqual(["c", "a", "b"])
    expect(sortedRows(undefined)).toEqual([])
  })

  it("maps subjects to accents and leaves other shelves neutral", () => {
    expect(domainColor("Math")).toBe("var(--acc-math)")
    expect(domainColor("unsorted")).toBeUndefined()
  })

  it("clamps the ratio", () => {
    expect(ratio(5, 0)).toBe(0)
    expect(ratio(5, 10)).toBe(0.5)
    expect(ratio(12, 10)).toBe(1)
  })
})
