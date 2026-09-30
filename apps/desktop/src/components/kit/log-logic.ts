/** Characters a typed log line is revealed over: the printed line, capped so long details stay quick. */
export function typedChars(time: string, label: string, detail: string, cap = 64): number {
  return Math.max(1, Math.min(`${time}  ${label.padEnd(7)}  ${detail}`.length, cap))
}
