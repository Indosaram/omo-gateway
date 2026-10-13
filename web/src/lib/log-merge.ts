import type { LogEntry } from '../api'

const MAX_LOGS = 400

/** Appends `incoming` to `prev`, skipping entries whose id is already present. */
export function appendLogs(prev: LogEntry[], incoming: LogEntry[]): LogEntry[] {
  const seen = new Set<number>()
  for (const entry of prev) if (typeof entry.id === 'number') seen.add(entry.id)
  const fresh: LogEntry[] = []
  for (const entry of incoming) {
    if (typeof entry.id === 'number') {
      if (seen.has(entry.id)) continue
      seen.add(entry.id)
    }
    fresh.push(entry)
  }
  return [...prev, ...fresh].slice(-MAX_LOGS)
}
