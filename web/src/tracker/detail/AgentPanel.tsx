import { useState } from 'react'
import { Bot, ChevronRight } from 'lucide-react'
import { trackerApi } from '../api'
import { fullDate, relativeTime } from '../format'
import { useTracker } from '../store'
import type { DelegateInput, IssueDetail, LaneAction, LaneStatus } from '../types'
import { LANE_STATUS_LABELS } from '../types'
import { AgentGlyph, Button, toast } from '../ui'
import { errorMessage } from './shared'
import { SessionAgentPanel } from './SessionAgentPanel'

const LANE_COLORS: Record<LaneStatus, string> = {
  queued: '#e2e2e2',
  working: '#f2c94c',
  blocked: '#eb5757',
  done_claimed: '#4cb782',
  not_done: '#fc7840',
  verified_done: '#5e6ad2',
  closed: '#5e6ad2',
  failed: '#95a2b3',
}

const ALLOWED: Record<LaneStatus, LaneAction[]> = {
  queued: ['send', 'close'],
  working: ['send', 'close'],
  blocked: ['send', 'close'],
  done_claimed: ['object', 'send'],
  not_done: ['send', 'close'],
  verified_done: ['close', 'object'],
  closed: ['reopen'],
  failed: ['reopen'],
}

const ACTION_LABELS: Record<LaneAction, string> = {
  send: 'Send message',
  object: 'Object',
  close: 'Close lane',
  reopen: 'Reopen',
}

const TEXT_ACTIONS: LaneAction[] = ['send', 'object', 'reopen']

export function LaneBadge({ status }: { status: LaneStatus }) {
  return (
    <span className="ld-lane-badge">
      <span className="ld-lane-dot" style={{ background: LANE_COLORS[status] }} />
      {LANE_STATUS_LABELS[status]}
    </span>
  )
}

export function AgentPanel({
  detail,
  onDetail,
  compact = false,
}: {
  detail: IssueDetail
  onDetail: (detail: IssueDetail) => void
  compact?: boolean
}) {
  const issue = detail.issue
  if (detail.session_detail) {
    return <SessionAgentPanel detail={detail} onDetail={onDetail} compact={compact} />
  }
  if (issue.lane_key || detail.lane_detail) {
    return <LanePanel detail={detail} onDetail={onDetail} compact={compact} />
  }
  return <DelegatePanel detail={detail} onDetail={onDetail} />
}

function LanePanel({
  detail,
  onDetail,
  compact,
}: {
  detail: IssueDetail
  onDetail: (detail: IssueDetail) => void
  compact: boolean
}) {
  const t = useTracker()
  const lane = detail.lane_detail?.lane
  const summary = detail.issue.lane
  const status = (lane?.status ?? summary?.status ?? 'queued') as LaneStatus
  const [pending, setPending] = useState<LaneAction | null>(null)
  const [text, setText] = useState('')
  const [force, setForce] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const run = async (action: LaneAction) => {
    setBusy(true)
    setError(null)
    try {
      const opts: { text?: string; force?: boolean } = {}
      if (text.trim()) opts.text = text.trim()
      if (action === 'close' && force) opts.force = true
      const next = await trackerApi.laneAction(detail.issue.id, action, opts)
      onDetail(next)
      setPending(null)
      setText('')
      setForce(false)
      toast(`${ACTION_LABELS[action]}: done`)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  const transcript = detail.lane_detail?.transcript ?? []
  const history = detail.lane_detail?.history ?? []
  const lastTranscript = transcript.slice(compact ? -3 : -12)
  const model = lane?.model ?? summary?.model
  const agentUser = detail.issue.assignee_id ? t.usersById.get(detail.issue.assignee_id) : undefined
  const needsText = pending !== null && (pending === 'send' || pending === 'object')

  return (
    <section className="ld-agent" aria-label="Agent lane">
      <div className="ld-agent-head">
        <AgentGlyph />
        <span className="ld-agent-title">Agent lane</span>
        <LaneBadge status={status} />
        <span style={{ flex: 1 }} />
        <span className="ld-comment-time mono">{lane?.key ?? summary?.key}</span>
      </div>
      <dl className="ld-agent-grid">
        <dt>Agent</dt>
        <dd>{agentUser?.name ?? model ?? 'Auto'}</dd>
        {model && (
          <>
            <dt>Model</dt>
            <dd className="mono">{model}</dd>
          </>
        )}
        <dt>Turns</dt>
        <dd>{lane?.turns_run ?? summary?.turns_run ?? 0}</dd>
        <dt>Last activity</dt>
        <dd title={fullDate(lane?.last_activity_at ?? summary?.last_activity_at)}>
          {relativeTime(lane?.last_activity_at ?? summary?.last_activity_at) || '—'}
        </dd>
        {lane?.cwd && (
          <>
            <dt>Working dir</dt>
            <dd className="mono">{lane.cwd}</dd>
          </>
        )}
        {lane?.last_turn_outcome && (
          <>
            <dt>Last outcome</dt>
            <dd>{lane.last_turn_outcome}</dd>
          </>
        )}
        {lane?.done_signals && !compact && (
          <>
            <dt>Done when</dt>
            <dd>{lane.done_signals}</dd>
          </>
        )}
        {lane?.objection && (
          <>
            <dt>Objection</dt>
            <dd>{lane.objection}</dd>
          </>
        )}
      </dl>
      {lane?.last_report && (
        <details className="ld-agent-block" open={!compact}>
          <summary>
            <ChevronRight size={12} />
            Last report
          </summary>
          <pre className="ld-agent-pre">{lane.last_report}</pre>
        </details>
      )}
      {lane?.claim_evidence && (
        <details className="ld-agent-block">
          <summary>
            <ChevronRight size={12} />
            Claim evidence
          </summary>
          <pre className="ld-agent-pre">{lane.claim_evidence}</pre>
        </details>
      )}
      {lane?.brief_path && !compact && (
        <details className="ld-agent-block">
          <summary>
            <ChevronRight size={12} />
            Brief
          </summary>
          <pre className="ld-agent-pre">{lane.brief_path}</pre>
        </details>
      )}
      {lastTranscript.length > 0 && (
        <details className="ld-agent-block">
          <summary>
            <ChevronRight size={12} />
            Transcript ({transcript.length})
          </summary>
          <div className="ld-agent-pre" style={{ fontFamily: 'inherit' }}>
            {lastTranscript.map((line) => (
              <div key={line.id} className="ld-transcript-item">
                <div className="ld-transcript-role">
                  {line.role} · {relativeTime(line.at)}
                </div>
                <div style={{ whiteSpace: 'pre-wrap' }}>{line.text}</div>
              </div>
            ))}
          </div>
        </details>
      )}
      {history.length > 0 && !compact && (
        <details className="ld-agent-block">
          <summary>
            <ChevronRight size={12} />
            Status history ({history.length})
          </summary>
          <div className="ld-agent-pre" style={{ fontFamily: 'inherit' }}>
            {history
              .slice()
              .reverse()
              .map((h) => (
                <div key={h.id} className="ld-transcript-item">
                  <span className="ld-transcript-role">{relativeTime(h.at)}</span>{' '}
                  {h.from_status ? `${LANE_STATUS_LABELS[h.from_status as LaneStatus] ?? h.from_status} → ` : ''}
                  {LANE_STATUS_LABELS[h.to_status as LaneStatus] ?? h.to_status}
                  {h.reason ? ` — ${h.reason}` : ''}
                </div>
              ))}
          </div>
        </details>
      )}

      {pending === null ? (
        <div className="ld-agent-actions">
          {ALLOWED[status].map((action) => (
            <Button
              key={action}
              variant={action === 'close' ? 'secondary' : action === 'send' ? 'primary' : 'secondary'}
              size="sm"
              onClick={() => {
                setError(null)
                setPending(action)
              }}
            >
              {ACTION_LABELS[action]}
            </Button>
          ))}
          {error && <span className="ld-error">{error}</span>}
        </div>
      ) : (
        <form
          className="ld-agent-form"
          onSubmit={(e) => {
            e.preventDefault()
            void run(pending)
          }}
        >
          {TEXT_ACTIONS.includes(pending) && (
            <label>
              {pending === 'object' ? 'Objection' : pending === 'reopen' ? 'Instructions (optional)' : 'Message to agent'}
              <textarea
                autoFocus
                className="ld-field"
                rows={3}
                value={text}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                    e.preventDefault()
                    if (!needsText || text.trim()) void run(pending)
                  } else if (e.key === 'Escape') {
                    e.stopPropagation()
                    setPending(null)
                  }
                }}
              />
            </label>
          )}
          {pending === 'close' && status !== 'verified_done' && (
            <>
              <label style={{ flexDirection: 'row', alignItems: 'center', gap: 8, color: 'var(--text-2)' }}>
                <input type="checkbox" checked={force} onChange={(e) => setForce(e.target.checked)} />
                Force close (lane is not verified)
              </label>
              {force && (
                <label>
                  Reason
                  <input className="ld-field" value={text} onChange={(e) => setText(e.target.value)} />
                </label>
              )}
            </>
          )}
          {error && <span className="ld-error">{error}</span>}
          <div className="ld-agent-form-row">
            <Button variant="ghost" size="sm" type="button" onClick={() => setPending(null)}>
              Cancel
            </Button>
            <Button
              variant={pending === 'close' ? 'danger' : 'primary'}
              size="sm"
              type="submit"
              disabled={busy || (needsText && !text.trim())}
            >
              {busy ? 'Working…' : ACTION_LABELS[pending]}
            </Button>
          </div>
        </form>
      )}
    </section>
  )
}

function DelegatePanel({ detail, onDetail }: { detail: IssueDetail; onDetail: (detail: IssueDetail) => void }) {
  const [open, setOpen] = useState(false)
  const [cwd, setCwd] = useState(() => {
    try {
      return localStorage.getItem('tracker.delegate.cwd') ?? ''
    } catch {
      return ''
    }
  })
  const [done, setDone] = useState('')
  const [model, setModel] = useState<NonNullable<DelegateInput['model']>>('auto')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const submit = async () => {
    if (!cwd.trim() || !done.trim()) return
    setBusy(true)
    setError(null)
    try {
      const next = await trackerApi.delegate(detail.issue.id, { cwd: cwd.trim(), done: done.trim(), model })
      try {
        localStorage.setItem('tracker.delegate.cwd', cwd.trim())
      } catch (storageErr) {
        console.warn('tracker: could not remember delegate cwd', storageErr)
      }
      onDetail(next)
      setOpen(false)
      toast(`${detail.issue.identifier} delegated to agent`)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  if (!open) {
    return (
      <section className="ld-agent" aria-label="Delegate">
        <div className="ld-agent-head">
          <Bot size={14} color="var(--text-3)" />
          <span className="ld-comment-time" style={{ flex: 1 }}>
            Hand this issue to an agent work lane.
          </span>
          <Button variant="secondary" size="sm" onClick={() => setOpen(true)}>
            Delegate to agent
          </Button>
        </div>
      </section>
    )
  }

  return (
    <section className="ld-agent" aria-label="Delegate">
      <div className="ld-agent-head">
        <AgentGlyph />
        <span className="ld-agent-title">Delegate to agent</span>
      </div>
      <form
        className="ld-agent-form"
        style={{ borderTop: 0 }}
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.stopPropagation()
            setOpen(false)
          } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault()
            void submit()
          }
        }}
      >
        <label>
          Working directory
          <input
            autoFocus
            className="ld-field mono"
            placeholder="/Users/me/code/project"
            value={cwd}
            onChange={(e) => setCwd(e.target.value)}
          />
        </label>
        <label>
          Done when
          <textarea
            className="ld-field"
            rows={3}
            placeholder="Describe how the agent proves the work is finished"
            value={done}
            onChange={(e) => setDone(e.target.value)}
          />
        </label>
        <label>
          Model
          <select
            className="ld-field"
            value={model}
            onChange={(e) => setModel(e.target.value as NonNullable<DelegateInput['model']>)}
          >
            <option value="auto">Auto</option>
            <option value="small">Small</option>
            <option value="large">Large</option>
          </select>
        </label>
        {error && <span className="ld-error">{error}</span>}
        <div className="ld-agent-form-row">
          <Button variant="ghost" size="sm" type="button" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" type="submit" disabled={busy || !cwd.trim() || !done.trim()}>
            {busy ? 'Delegating…' : 'Delegate'}
          </Button>
        </div>
      </form>
    </section>
  )
}
