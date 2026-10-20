import { useState } from 'react'
import { ChevronRight, CircleStop, MessageSquareReply, Send, SquareTerminal } from 'lucide-react'
import { trackerApi } from '../api'
import { fullDate, modKey, relativeTime } from '../format'
import { useTracker } from '../store'
import type { IssueDetail, SessionActionResult, SessionDetail, SessionQuestion, SessionState } from '../types'
import { Button, Dialog, toast } from '../ui'
import { errorMessage } from './shared'

const STATE_LABELS: Record<SessionState, string> = {
  question: 'Needs answer',
  blocked: 'Blocked',
  working: 'Working',
  idle: 'Idle',
  ended: 'Ended',
}

// Same colours as the seeded SES workflow states, so the panel matches the board column.
const STATE_COLORS: Record<SessionState, string> = {
  question: '#eb5757',
  blocked: '#fc7840',
  working: '#f2c94c',
  idle: '#e2e2e2',
  ended: '#95a2b3',
}

export function sessionKey(session: Pick<SessionDetail, 'machine' | 'session_id'>): string {
  return `${session.machine}:${session.session_id}`
}

function shortenPath(path: string | null): string {
  if (!path) return ''
  return path.replace(/^\/(?:Users|home)\/[^/]+(?=\/|$)/, '~')
}

export function SessionAgentPanel({
  detail,
  onDetail,
  compact = false,
}: {
  detail: IssueDetail
  onDetail: (detail: IssueDetail) => void
  compact?: boolean
}) {
  const t = useTracker()
  const session = detail.session_detail as SessionDetail
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState<string | null>(null)
  const [confirmInterrupt, setConfirmInterrupt] = useState(false)
  const label = STATE_LABELS[session.state] ?? session.state
  const ended = session.state === 'ended'

  const run = async (name: string, action: () => Promise<SessionActionResult>, success: string): Promise<boolean> => {
    setBusy(name)
    try {
      const result = await action()
      if (!result.ok) {
        toast.error(`${name} failed`)
        return false
      }
      toast.success(success)
      void t.refreshIssues()
      try {
        onDetail(await trackerApi.issue(detail.issue.id))
      } catch (err) {
        toast.error(`Could not reload ${detail.issue.identifier}: ${errorMessage(err)}`)
      }
      return true
    } catch (err) {
      toast.error(`${name} failed: ${errorMessage(err)}`)
      return false
    } finally {
      setBusy(null)
    }
  }

  const send = async () => {
    const text = message.trim()
    if (!text || busy) return
    if (await run('Send', () => trackerApi.sessions.send(session.machine, session.session_id, text), 'Message sent')) {
      setMessage('')
    }
  }

  const answer = (text: string, request: string) =>
    run('Answer', () => trackerApi.sessions.answer(session.machine, session.session_id, text, request), 'Answer sent')

  const interrupt = async () => {
    setConfirmInterrupt(false)
    await run('Interrupt', () => trackerApi.sessions.interrupt(session.machine, session.session_id), 'Session interrupted')
  }

  return (
    <section className="ld-agent" aria-label="omo session">
      <div className="ld-agent-head">
        <SquareTerminal size={14} color="var(--text-3)" />
        <span className="ld-agent-title">omo session</span>
        <span className="ld-lane-badge">
          <span className="ld-lane-dot" style={{ background: STATE_COLORS[session.state] ?? STATE_COLORS.idle }} />
          {label}
        </span>
        <span style={{ flex: 1 }} />
        {session.streaming ? (
          <Button
            size="sm"
            variant="danger"
            icon={<CircleStop size={13} />}
            disabled={busy !== null}
            onClick={() => setConfirmInterrupt(true)}
          >
            Interrupt
          </Button>
        ) : null}
      </div>
      <dl className="ld-agent-grid">
        <dt>Machine</dt>
        <dd>
          <span className="ld-session-chip">{session.machine}</span>
        </dd>
        {session.cwd ? (
          <>
            <dt>Directory</dt>
            <dd className="mono" title={session.cwd}>
              {shortenPath(session.cwd)}
            </dd>
          </>
        ) : null}
        {!compact ? (
          <>
            <dt>Session</dt>
            <dd className="mono">{session.session_id}</dd>
          </>
        ) : null}
        <dt>Messages</dt>
        <dd>
          {session.message_count}
          {session.pending_messages > 0 ? ` · ${session.pending_messages} queued` : ''}
        </dd>
        <dt>Updated</dt>
        <dd title={fullDate(session.updated_at)}>{relativeTime(session.updated_at) || '—'}</dd>
      </dl>

      {session.questions.map((q) => (
        <QuestionBlock key={q.request_id} question={q} disabled={busy !== null} onAnswer={answer} />
      ))}

      <details className="ld-agent-block" open={!compact}>
        <summary>
          <ChevronRight size={12} />
          Last user message
        </summary>
        <pre className="ld-agent-pre">{session.last_user || 'No messages yet'}</pre>
      </details>
      <details className="ld-agent-block" open={!compact}>
        <summary>
          <ChevronRight size={12} />
          Last assistant message
        </summary>
        <pre className="ld-agent-pre">{session.last_assistant || 'No reply yet'}</pre>
      </details>

      {!ended ? (
        <form
          className="ld-agent-form"
          onSubmit={(e) => {
            e.preventDefault()
            void send()
          }}
        >
          <label>
            Send message
            <textarea
              className="ld-field"
              rows={3}
              value={message}
              placeholder={session.streaming ? 'Steer the running turn…' : 'Message this session…'}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault()
                  void send()
                } else if (e.key === 'Escape') {
                  e.stopPropagation()
                  ;(e.target as HTMLTextAreaElement).blur()
                }
              }}
            />
          </label>
          <div className="ld-agent-form-row">
            <span className="ld-comment-time" style={{ flex: 1 }}>
              {modKey()} Enter to send
            </span>
            <Button size="sm" variant="primary" type="submit" icon={<Send size={13} />} disabled={!message.trim() || busy !== null}>
              {busy === 'Send' ? 'Sending…' : 'Send'}
            </Button>
          </div>
        </form>
      ) : null}

      <Dialog
        open={confirmInterrupt}
        onOpenChange={setConfirmInterrupt}
        title="Interrupt session?"
        width={400}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmInterrupt(false)}>
              Cancel
            </Button>
            <Button variant="danger" data-autofocus onClick={() => void interrupt()}>
              Interrupt
            </Button>
          </>
        }
      >
        <p className="t-2" style={{ fontSize: 13, margin: 0 }}>
          This stops the running turn in <b className="w-510">{detail.issue.title}</b> on {session.machine}.
        </p>
      </Dialog>
    </section>
  )
}

function QuestionBlock({
  question,
  disabled,
  onAnswer,
}: {
  question: SessionQuestion
  disabled: boolean
  onAnswer: (text: string, request: string) => Promise<boolean>
}) {
  const [text, setText] = useState('')
  const submit = async () => {
    const value = text.trim()
    if (!value || disabled) return
    if (await onAnswer(value, question.request_id)) setText('')
  }
  return (
    <div className="ld-agent-form ld-session-question">
      <span className="ld-session-question-title">
        <MessageSquareReply size={13} />
        Pending question
      </span>
      {question.questions.map((item) => (
        <div key={item.id} className="ld-session-question-item">
          {item.header ? <div className="ld-session-question-header">{item.header}</div> : null}
          {item.question ? <div className="ld-session-question-text">{item.question}</div> : null}
          {item.options.length > 0 ? (
            <div className="ld-session-options">
              {item.options.map((option) => (
                <Button key={option} size="sm" variant="secondary" disabled={disabled} onClick={() => void onAnswer(option, question.request_id)}>
                  {option}
                </Button>
              ))}
            </div>
          ) : null}
        </div>
      ))}
      <div className="ld-agent-form-row">
        <input
          className="ld-field"
          style={{ flex: 1 }}
          value={text}
          placeholder="Write an answer…"
          aria-label="Answer"
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing) {
              e.preventDefault()
              void submit()
            }
          }}
        />
        <Button size="sm" variant="primary" disabled={!text.trim() || disabled} onClick={() => void submit()}>
          Answer
        </Button>
      </div>
    </div>
  )
}
