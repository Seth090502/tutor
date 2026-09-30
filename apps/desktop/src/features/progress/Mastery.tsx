// Mastery: a sunburst of subject > topic > skill with its table view, then the topics list with due reviews.
import { useMemo } from "react"
import { Clock } from "lucide-react"

import { ChartFrame, StackedBar, Sunburst, sunburstTable } from "@/components/charts"
import { CountUp, Eyebrow, Panel } from "@/components/kit"
import { fmtInt, masterySegments, masteryTree, sortTopics, topicCounts, topicDue, totalCounts, type Topic } from "./model"
import { Quiet } from "./parts"

export function MasteryMap({ topics }: { topics: Topic[] }) {
  const tree = useMemo(() => masteryTree(topics), [topics])
  const totals = useMemo(() => totalCounts(topics), [topics])
  const segments = useMemo(() => masterySegments(totals), [totals])
  const pct = totals.total > 0 ? Math.round((100 * totals.mastered) / totals.total) : 0

  return (
    <ChartFrame
      title="Mastery map"
      subtitle="Angle is the number of skills, tint is mastery. Click an arc to zoom in; Escape or the centre zooms out."
      table={sunburstTable(tree, "Skills", "Mastery by subject, topic and skill")}
    >
      {tree.length === 0 ? (
        <Quiet>No skills to map yet. Topics appear here once they have objectives.</Quiet>
      ) : (
        <div className="grid items-center gap-x-10 gap-y-8 lg:grid-cols-[minmax(0,460px)_minmax(0,1fr)]">
          <Sunburst name="Mastery by subject, topic and skill" data={tree} size={460} unit="skills" className="mx-auto w-full" />
          <div className="flex min-w-0 flex-col gap-6">
            <div className="flex items-baseline gap-3">
              <span className="text-[56px] leading-none font-extralight tracking-[-0.04em] tabular-nums">
                <CountUp value={pct} />
                <span className="text-ink-2 ml-0.5 text-[28px] font-light">%</span>
              </span>
              <Eyebrow>Skills mastered</Eyebrow>
            </div>
            <StackedBar
              name="Skills by status across all topics"
              segments={segments}
              unit="skills"
              height={18}
            />
            <p className="text-ink-3 font-mono text-[11px] leading-snug">
              {fmtInt(totals.total)} skills across {fmtInt(topics.length)} {topics.length === 1 ? "topic" : "topics"}. The map centre shows average mastery, which also counts partial progress.
            </p>
          </div>
        </div>
      )}
    </ChartFrame>
  )
}


function TopicMeter({ topic }: { topic: Topic }) {
  const c = topicCounts(topic)
  const segs = masterySegments(c)
  return (
    <div className="min-w-0">
      {c.total > 0 ? (
        <>
          <div aria-hidden className="bg-hairline flex h-1.5 gap-px overflow-hidden rounded-full">
            {segs.map((s) =>
              s.value > 0 ? <span key={s.id} style={{ flexGrow: s.value, background: s.color }} className="min-w-0 first:rounded-l-full last:rounded-r-full" /> : null,
            )}
          </div>
          <p className="text-ink-2 mt-2 font-mono text-[11px] tabular-nums">
            {c.mastered} mastered · {c.learning} learning · {c.new} new
          </p>
        </>
      ) : (
        <p className="text-ink-3 font-mono text-[11px]">No objectives yet</p>
      )}
    </div>
  )
}

export function TopicList({ topics }: { topics: Topic[] }) {
  const sorted = useMemo(() => sortTopics(topics), [topics])
  return (
    <Panel eyebrow="Topics" title="Due reviews by topic">
      {sorted.length === 0 ? (
        <Quiet>No mastery topics yet. Start one from the tutor and it will be tracked here.</Quiet>
      ) : (
        <ul className="m-0 list-none p-0">
          {sorted.map((t) => {
            const due = topicDue(t)
            return (
              <li
                key={t.path_id}
                className="border-hairline grid gap-x-8 gap-y-2 border-b py-4 first:pt-0 last:border-b-0 last:pb-0 md:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)_6rem] md:items-center"
              >
                <span className="min-w-0 text-[15px] font-medium text-balance break-words">{t.name}</span>
                <TopicMeter topic={t} />
                <span className="flex items-center gap-2 text-[13px] md:justify-end">
                  <Clock aria-hidden className={due > 0 ? "text-warn size-3.5" : "text-ink-3 size-3.5"} />
                  <span className={due > 0 ? "text-foreground tabular-nums" : "text-ink-2"}>{due > 0 ? `${fmtInt(due)} due` : "None due"}</span>
                </span>
              </li>
            )
          })}
        </ul>
      )}
    </Panel>
  )
}
