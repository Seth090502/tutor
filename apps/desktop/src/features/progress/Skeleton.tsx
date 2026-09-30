// Loading state: the page's own layout drawn as still, quiet blocks. Nothing animates, so a slow
// backend costs no frames.
import { Panel } from "@/components/kit"

const block = "bg-muted rounded-md"

export function ProgressSkeleton() {
  return (
    <div role="status" aria-busy="true" aria-label="Loading progress" className="flex flex-col gap-5">
      <span className="sr-only">Loading progress</span>
      <div aria-hidden className="grid gap-4 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)]">
        <Panel eyebrow="Reviews" className="flex h-60 flex-col items-center justify-center gap-4">
          <div className={`${block} h-3 w-40`} />
          <div className={`${block} h-11 w-20`} />
          <div className={`${block} h-3 w-24`} />
        </Panel>
        <div className="border-hairline bg-hairline grid grid-cols-2 gap-px overflow-hidden rounded-2xl border">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="bg-card flex flex-col gap-4 px-6 py-5">
              <div className={`${block} h-3 w-24`} />
              <div className={`${block} h-9 w-16`} />
            </div>
          ))}
        </div>
      </div>
      {[0, 1].map((i) => (
        <Panel key={i} aria-hidden className="h-56">
          <div className={`${block} h-3 w-32`} />
          <div className={`${block} mt-6 h-28 w-full opacity-60`} />
        </Panel>
      ))}
    </div>
  )
}
