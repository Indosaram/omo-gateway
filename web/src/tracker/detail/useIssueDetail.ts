import { useCallback, useEffect, useRef, useState } from 'react'
import { trackerApi } from '../api'
import { useTracker } from '../store'
import type { Issue, IssueDetail } from '../types'
import { errorMessage } from './shared'

export interface IssueDetailHandle {
  detail: IssueDetail | null
  issue: Issue | null
  loading: boolean
  error: string | null
  notFound: boolean
  reload: () => Promise<void>
  setDetail: (detail: IssueDetail) => void
}

export function useIssueDetail(idOrIdentifier: string | null): IssueDetailHandle {
  const t = useTracker()
  const [detail, setDetailState] = useState<IssueDetail | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notFound, setNotFound] = useState(false)
  const requestId = useRef(0)
  const upsertIssue = t.upsertIssue

  const reload = useCallback(async () => {
    if (!idOrIdentifier) return
    const current = ++requestId.current
    setLoading(true)
    try {
      const next = await trackerApi.issue(idOrIdentifier)
      if (current !== requestId.current) return
      setDetailState(next)
      setError(null)
      setNotFound(false)
      upsertIssue(next.issue)
    } catch (err) {
      if (current !== requestId.current) return
      const status = (err as { status?: number }).status
      setNotFound(status === 404)
      setError(errorMessage(err))
    } finally {
      if (current === requestId.current) setLoading(false)
    }
  }, [idOrIdentifier, upsertIssue])

  useEffect(() => {
    setDetailState(null)
    setNotFound(false)
    setError(null)
    void reload()
  }, [reload])

  const storeIssue =
    (detail && t.issuesById.get(detail.issue.id)) ||
    (idOrIdentifier ? t.issuesById.get(idOrIdentifier) ?? t.issuesByIdentifier.get(idOrIdentifier) : undefined) ||
    null

  // The store polls every 10s; when it sees a newer revision, pull the full detail (comments, activity) too.
  const storeStamp = storeIssue?.updated_at
  const detailStamp = detail?.issue.updated_at
  const laneStamp = storeIssue?.lane ? `${storeIssue.lane.status}:${storeIssue.lane.last_activity_at}` : null
  const seenLane = useRef<string | null>(null)
  useEffect(() => {
    if (!detail || !storeStamp || !detailStamp) return
    const laneChanged = laneStamp !== null && seenLane.current !== null && seenLane.current !== laneStamp
    seenLane.current = laneStamp
    if (storeStamp > detailStamp || laneChanged) void reload()
  }, [storeStamp, detailStamp, laneStamp, detail, reload])

  const setDetail = useCallback(
    (next: IssueDetail) => {
      setDetailState(next)
      upsertIssue(next.issue)
    },
    [upsertIssue],
  )

  return {
    detail,
    issue: storeIssue ?? detail?.issue ?? null,
    loading,
    error,
    notFound,
    reload,
    setDetail,
  }
}
