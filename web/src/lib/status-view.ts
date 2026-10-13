export interface ApprovalSession {
  platform: string
  channel_id: string
  thread_id?: string | null
  user_id: string
}

export function approvalSessionLabel(session: ApprovalSession | null | undefined): string {
  if (!session) return 'unknown session'
  const thread = session.thread_id ? `/${session.thread_id}` : ''
  return `${session.platform}:${session.channel_id}${thread} (${session.user_id})`
}

/** Disk usage in bytes, or null when the host did not report a usable sample. */
export function diskUsage(disk: {
  workspace_total_bytes: number | null
  workspace_available_bytes: number | null
}): { used: number; total: number; percent: number } | null {
  const total = disk.workspace_total_bytes
  const available = disk.workspace_available_bytes
  if (total == null || available == null || total <= 0) return null
  const used = Math.max(0, total - available)
  return { used, total, percent: Math.round((used / total) * 100) }
}
