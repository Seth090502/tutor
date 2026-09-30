/** Honest empty state for a tracker section whose data source is not built yet. */
export function PendingSection({
  tracks,
  source,
}: {
  tracks: string[]
  source: string
}) {
  return (
    <div className="mx-auto flex h-full max-w-2xl flex-col justify-center gap-6 p-8">
      <div className="bg-card rounded-xl border p-6">
        <h3 className="mb-3 text-sm font-medium">Will track</h3>
        <ul className="text-muted-foreground list-disc space-y-1 pl-5 text-sm">
          {tracks.map((t) => (
            <li key={t}>{t}</li>
          ))}
        </ul>
        <p className="text-muted-foreground mt-4 border-t pt-4 text-xs">Data source: {source}</p>
      </div>
    </div>
  )
}
