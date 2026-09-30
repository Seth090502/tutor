export type { MarkerShape, SunburstDatum } from "./geometry"

/** The data-table alternative every chart can produce; ChartFrame renders it as a real <table>. */
export interface TableData {
  caption?: string
  columns: string[]
  rows: (string | number)[][]
  /** Per-column alignment. Defaults to left for the first column and right for the rest. */
  align?: ("left" | "right")[]
}
