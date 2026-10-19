import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from 'react'
import {
  TriangleAlert,
  ArrowLeft,
  ChevronDown,
  CircleStop,
  MessageSquare,
  MessageSquareReply,
  RefreshCw,
  Send,
  SquareTerminal,
  X,
} from 'lucide-react'
import { trackerApi } from '../api'
import { modKey, relativeTime } from '../format'
import { useHotkey } from '../hotkeys'
import { hrefFor, navigate } from '../router'
import { PageHeader, useIsMobile } from '../shell/Header'
import type { OmoSession, SessionCounts, SessionQuestion, SessionsOverview, SessionState } from '../types'
import { Button, Dialog, IconButton, Tooltip, toast } from '../ui'
import './sessions.css'

interface SessionsStore {
  data: SessionsOverview | null
  error: string | null
  loading: boolean
}

let store: SessionsStore = { data: null, error: null, loading: false }
let inflight: Promise<void> | null = null
const listeners = new Set<() => void>()

function setStore(patch: Partial<SessionsStore>) {
  store = { ...store, ...patch }
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

export function loadSessions(refresh = false): Promise<void> {
  if (inflight && !refresh) return inflight
  setStore({ loading: true })
  const request = trackerApi.sessions
    .list({ refresh })
    .then((data) => setStore({ data, error: null, loading: false }))
    .catch((err: unknown) => setStore({ error: errorMessage(err), loading: false }))
    .finally(() => {
      if (inflight === request) inflight = null
    })
  inflight = request
  return request
}

function useSessionsStore(): SessionsStore {
  return useSyncExternalStore(subscribe, () => store, () => store)
}

function usePolling(intervalMs: number) {
  useEffect(() => {
    const tick = () => {
      if (document.visibilityState === 'visible') void loadSessions()
    }
    tick()
    const timer = window.setInterval(tick, intervalMs)
    document.addEventListener('visibilitychange', tick)
    return () => {
      window.clearInterval(timer)
      document.removeEventListener('visibilitychange', tick)
    }
  }, [intervalMs])
}

export function useSessionsSummary(): SessionCounts | null {
  usePolling(30_000)
  return useSessionsStore().data?.counts ?? null
}

export function sessionKey(session: Pick<OmoSession, 'machine' | 'session_id'>): string {
  return `${session.machine}:${session.session_id}`
}

function shortenPath(path: string | null): string {
  if (!path) return ''
  return path.replace(/^\/Users\/indo(?=\/|$)/, '~')
}

function basename(path: string | null): string {
  if (!path) return ''
  const parts = path.split('/').filter(Boolean)
  return parts[parts.length - 1] ?? ''
}

function sessionTitle(session: OmoSession): string {
  const name = session.name?.trim()
  // omo falls back to the transcript path when a session has no title; that is not a name.
  const usable = name && !name.startsWith('/') ? name : ''
  return usable || basename(session.cwd) || session.session_id.slice(0, 8)
}

function firstLine(text: string): string {
  return text.split('\n').find((line) => line.trim() !== '')?.trim() ?? ''
}

// The probe keeps only the tail of each message, so the first line usually starts mid-word;
// the last complete line is the latest thing the session said.
function lastLine(text: string): string {
  const lines = text.split('\n').map((line) => line.trim()).filter((line) => line !== '')
  return lines[lines.length - 1] ?? ''
}

function byRecent(a: OmoSession, b: OmoSession): number {
  return (b.updated_at ?? '').localeCompare(a.updated_at ?? '')
}

const STATE_LABEL: Record<SessionState, string> = {
  working: 'Working',
  question: 'Needs answer',
  blocked: 'Blocked',
  idle: 'Idle',
}

function StateDot({ state }: { state: SessionState }) {
  return <span className={`ss-dot ss-dot--${state}`} role="img" aria-label={STATE_LABEL[state]} />
}

type GroupId = 'attention' | 'working' | 'idle'

interface Group {
  id: GroupId
  title: string
  sessions: OmoSession[]
}

const IDLE_COLLAPSE_THRESHOLD = 8

export function SessionsPage({ sessionKey: openKey }: { sessionKey: string | null }) {
  const { data, error, loading } = useSessionsStore()
  const mobile = useIsMobile()
  const [idleOpenChoice, setIdleOpenChoice] = useState<boolean | null>(null)
  const [cursor, setCursor] = useState<string | null>(openKey)
  const [, setTick] = useState(0)

  usePolling(15_000)

  useEffect(() => {
    const timer = window.setInterval(() => setTick((n) => n + 1), 30_000)
    return () => window.clearInterval(timer)
  }, [])

  useEffect(() => {
    if (openKey) setCursor(openKey)
  }, [openKey])

  const sessions = useMemo(() => data?.sessions ?? [], [data])
  const groups = useMemo<Group[]>(() => {
    const attention = sessions.filter((s) => s.state === 'question' || s.state === 'blocked').sort(byRecent)
    const working = sessions.filter((s) => s.state === 'working').sort(byRecent)
    const idle = sessions.filter((s) => s.state === 'idle').sort(byRecent)
    return [
      { id: 'attention' as const, title: 'Needs answer', sessions: attention },
      { id: 'working' as const, title: 'Working', sessions: working },
      { id: 'idle' as const, title: 'Idle', sessions: idle },
    ].filter((g) => g.sessions.length > 0)
  }, [sessions])

  const idleCount = groups.find((g) => g.id === 'idle')?.sessions.length ?? 0
  const idleOpen = idleOpenChoice ?? idleCount <= IDLE_COLLAPSE_THRESHOLD

  const visible = useMemo(
    () => groups.flatMap((g) => (g.id === 'idle' && !idleOpen ? [] : g.sessions)),
    [groups, idleOpen],
  )

  const openSession = openKey ? sessions.find((s) => sessionKey(s) === openKey) ?? null : null

  const open = useCallback(
    (key: string) => navigate({ name: 'sessions', sessionKey: key }, { replace: Boolean(openKey) }),
    [openKey],
  )
  const close = useCallback(() => navigate({ name: 'sessions', sessionKey: null }, { replace: true }), [])

  const step = (delta: number) => {
    if (visible.length === 0) return
    const current = visible.findIndex((s) => sessionKey(s) === (openKey ?? cursor))
    const index = current < 0 ? (delta > 0 ? 0 : visible.length - 1) : Math.min(visible.length - 1, Math.max(0, current + delta))
    const key = sessionKey(visible[index])
    setCursor(key)
    if (openKey) open(key)
    document.querySelector(`[data-session-key="${CSS.escape(key)}"]`)?.scrollIntoView({ block: 'nearest' })
  }

  useHotkey(['j', 'arrowdown'], () => step(1))
  useHotkey(['k', 'arrowup'], () => step(-1))
  useHotkey('enter', () => cursor && open(cursor), { enabled: Boolean(cursor) && cursor !== openKey })
  useHotkey('escape', close, { enabled: Boolean(openKey), preventDefault: false })

  const counts = data?.counts
  const failedMachines = data?.machines.filter((m) => !m.ok) ?? []
  const showList = !mobile || !openKey
  const showPanel = Boolean(openKey)

  const headerActions = (
    <>
      {counts ? (
        <span className="ss-head-meta">
          <span>
            Working <b>{counts.working}</b>
          </span>
          <span className="ss-head-sep">·</span>
          <span>
            Needs answer <b>{counts.question + counts.blocked}</b>
          </span>
          <span className="ss-head-sep">·</span>
          <span>
            Idle <b>{counts.idle}</b>
          </span>
          {data ? (
            <span className="ss-hide-mobile" title={data.collected_at}>
              Updated {relativeTime(data.collected_at)}
            </span>
          ) : null}
        </span>
      ) : null}
      <IconButton
        label="Refresh"
        icon={<RefreshCw size={14} className={loading ? 'ss-spin' : undefined} />}
        disabled={loading}
        onClick={() => void loadSessions(true)}
      />
    </>
  )

  return (
    <div className="ss-root">
      {showList ? (
        <div className="ss-list-col">
          <PageHeader title="Sessions" icon={<SquareTerminal size={14} />} actions={headerActions} />
          <div className="ss-scroll" role="listbox" aria-label="Sessions">
            {error ? (
              <div className="ss-banner" role="alert">
                <TriangleAlert size={14} />
                <span style={{ flex: 1 }}>Could not load sessions: {error}</span>
                <Button size="sm" variant="ghost" onClick={() => void loadSessions(true)}>
                  Retry
                </Button>
              </div>
            ) : null}
            {failedMachines.map((m) => (
              <div key={m.machine} className="ss-banner" role="status">
                <TriangleAlert size={14} />
                <span>
                  <b className="w-510">{m.machine}</b> is unreachable{m.error ? <> — <code>{m.error}</code></> : null}
                </span>
              </div>
            ))}
            {!data && !error ? (
              <div style={{ display: 'grid', gap: 10, padding: 16 }} role="status" aria-label="Loading">
                <div className="ls-skel" style={{ width: '60%' }} />
                <div className="ls-skel" />
                <div className="ls-skel" style={{ width: '40%' }} />
              </div>
            ) : null}
            {data && sessions.length === 0 ? (
              <div className="ss-empty">
                <SquareTerminal size={28} className="t-4" />
                <div className="ss-empty-title">No omo sessions are running</div>
                <div className="ss-machines">
                  {data.machines.map((m) => (
                    <div key={m.machine} className="ss-machine">
                      <span className={`ss-chip ${m.ok ? '' : 'is-muted'}`}>{m.machine}</span>
                      <span>{m.ok ? `${m.session_count} sessions` : 'unreachable'}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
            {groups.map((group) => {
              const collapsible = group.id === 'idle'
              const groupOpen = !collapsible || idleOpen
              return (
                <section key={group.id} aria-label={group.title}>
                  {collapsible ? (
                    <button
                      type="button"
                      className="ss-group-head"
                      aria-expanded={groupOpen}
                      onClick={() => setIdleOpenChoice(!groupOpen)}
                    >
                      <ChevronDown size={12} className={`ss-caret ${groupOpen ? '' : 'is-closed'}`} />
                      {group.title}
                      <span className="ss-group-count">{group.sessions.length}</span>
                    </button>
                  ) : (
                    <div className="ss-group-head">
                      {group.title}
                      <span className="ss-group-count">{group.sessions.length}</span>
                    </div>
                  )}
                  {groupOpen
                    ? group.sessions.map((session) => (
                        <SessionRow
                          key={sessionKey(session)}
                          session={session}
                          selected={sessionKey(session) === openKey}
                          cursor={sessionKey(session) === cursor}
                          onOpen={() => {
                            setCursor(sessionKey(session))
                            open(sessionKey(session))
                          }}
                        />
                      ))
                    : null}
                </section>
              )
            })}
          </div>
        </div>
      ) : null}
      {showPanel ? (
        openSession ? (
          <SessionPanel key={openKey ?? ''} session={openSession} mobile={mobile} onClose={close} />
        ) : (
          <aside className="ss-panel" aria-label="Session">
            <div className="ss-panel-head">
              <span className="ss-panel-title">Session not found</span>
              <IconButton label="Close" icon={<X size={14} />} kbd={['Esc']} onClick={close} />
            </div>
            <div className="ss-empty">{data ? 'This session is no longer running.' : 'Loading…'}</div>
          </aside>
        )
      ) : null}
    </div>
  )
}

function SessionRow({
  session,
  selected,
  cursor,
  onOpen,
}: {
  session: OmoSession
  selected: boolean
  cursor: boolean
  onOpen: () => void
}) {
  const key = sessionKey(session)
  const question = session.questions[0]?.questions[0]?.question
  const preview = (session.state === 'question' && question ? firstLine(question) : '') || lastLine(session.last_assistant) || lastLine(session.last_user)
  return (
    <a
      href={hrefFor({ name: 'sessions', sessionKey: key })}
      data-session-key={key}
      role="option"
      aria-selected={selected}
      className={`ss-row ${selected ? 'is-selected' : ''} ${cursor && !selected ? 'is-cursor' : ''}`}
      onClick={(event) => {
        event.preventDefault()
        onOpen()
      }}
    >
      <span className="ss-row-main">
        <StateDot state={session.state} />
        <span className="ss-row-title">{sessionTitle(session)}</span>
        <span className="ss-chip">{session.machine}</span>
        <span className="ss-row-cwd" title={session.cwd ?? undefined}>
          {shortenPath(session.cwd)}
        </span>
        <span className="ss-row-meta">
          {session.pending_messages > 0 ? (
            <span title="Queued messages">+{session.pending_messages} queued</span>
          ) : null}
          <span title="Messages">
            <MessageSquare size={12} />
            {session.message_count}
          </span>
          {session.updated_at ? <span title={session.updated_at}>{relativeTime(session.updated_at)}</span> : null}
        </span>
      </span>
      {preview ? <span className={`ss-row-sub ${session.state === 'question' ? 'is-question' : ''}`}>{preview}</span> : null}
    </a>
  )
}

function SessionPanel({ session, mobile, onClose }: { session: OmoSession; mobile: boolean; onClose: () => void }) {
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState<string | null>(null)
  const [confirmInterrupt, setConfirmInterrupt] = useState(false)

  const run = async (label: string, action: () => Promise<{ ok: boolean }>, success: string): Promise<boolean> => {
    setBusy(label)
    try {
      const result = await action()
      if (!result.ok) {
        toast.error(`${label} failed`)
        return false
      }
      toast.success(success)
      void loadSessions(true)
      return true
    } catch (err) {
      toast.error(`${label} failed: ${errorMessage(err)}`)
      return false
    } finally {
      setBusy(null)
    }
  }

  const send = async () => {
    const text = message.trim()
    if (!text || busy) return
    if (await run('Send', () => trackerApi.sessions.send(session.machine, session.session_id, text), 'Message sent')) setMessage('')
  }

  const answer = (text: string, request: string) =>
    run('Answer', () => trackerApi.sessions.answer(session.machine, session.session_id, text, request), 'Answer sent')

  const interrupt = async () => {
    setConfirmInterrupt(false)
    await run('Interrupt', () => trackerApi.sessions.interrupt(session.machine, session.session_id), 'Session interrupted')
  }

  return (
    <aside className="ss-panel" aria-label={`Session ${sessionTitle(session)}`}>
      <div className="ss-panel-head">
        {mobile ? <IconButton label="Back to sessions" icon={<ArrowLeft size={15} />} onClick={onClose} /> : null}
        <StateDot state={session.state} />
        <span className="ss-panel-title">{sessionTitle(session)}</span>
        {session.streaming ? (
          <Button size="sm" variant="danger" icon={<CircleStop size={14} />} disabled={busy !== null} onClick={() => setConfirmInterrupt(true)}>
            Interrupt
          </Button>
        ) : null}
        {!mobile ? <IconButton label="Close" icon={<X size={14} />} kbd={['Esc']} onClick={onClose} /> : null}
      </div>
      <div className="ss-panel-body">
        <dl className="ss-props">
          <dt>Status</dt>
          <dd>
            {STATE_LABEL[session.state]}
            {session.compacting ? ' · compacting' : ''}
            {session.pending_messages > 0 ? ` · ${session.pending_messages} queued` : ''}
          </dd>
          <dt>Machine</dt>
          <dd>
            <span className="ss-chip">{session.machine}</span>
          </dd>
          {session.cwd ? (
            <>
              <dt>Directory</dt>
              <dd className="ss-mono" title={session.cwd}>
                {shortenPath(session.cwd)}
              </dd>
            </>
          ) : null}
          <dt>Session</dt>
          <dd className="ss-mono">{session.session_id}</dd>
          <dt>Messages</dt>
          <dd>{session.message_count}</dd>
          {session.updated_at ? (
            <>
              <dt>Updated</dt>
              <dd title={session.updated_at}>{relativeTime(session.updated_at)}</dd>
            </>
          ) : null}
        </dl>

        {session.questions.map((q) => (
          <QuestionCard key={q.request_id} question={q} disabled={busy !== null} onAnswer={answer} />
        ))}

        <div>
          <h3 className="ss-section-title">Last user message</h3>
          <pre className={`ss-message ${session.last_user ? '' : 'is-empty'}`}>{session.last_user || 'No messages yet'}</pre>
        </div>
        <div>
          <h3 className="ss-section-title">Last assistant message</h3>
          <pre className={`ss-message ${session.last_assistant ? '' : 'is-empty'}`}>{session.last_assistant || 'No reply yet'}</pre>
        </div>

        <div>
          <h3 className="ss-section-title">Send message</h3>
          <textarea
            className="ss-textarea"
            value={message}
            placeholder={session.streaming ? 'Steer the running turn…' : 'Message this session…'}
            onChange={(event) => setMessage(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
                event.preventDefault()
                void send()
              }
            }}
            aria-label="Message"
          />
          <div className="ss-send-foot">
            <span>{modKey()} Enter to send</span>
            <Button
              size="sm"
              variant="primary"
              icon={<Send size={13} />}
              disabled={!message.trim() || busy !== null}
              onClick={() => void send()}
            >
              {busy === 'Send' ? 'Sending…' : 'Send'}
            </Button>
          </div>
        </div>
      </div>
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
          This stops the running turn in <b className="w-510">{sessionTitle(session)}</b> on {session.machine}.
        </p>
      </Dialog>
    </aside>
  )
}

function QuestionCard({
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
    <div className="ss-question">
      <h3 className="ss-section-title" style={{ margin: 0 }}>
        <MessageSquareReply size={13} />
        Pending question
      </h3>
      {question.questions.map((item) => (
        <div key={item.id} style={{ display: 'grid', gap: 6 }}>
          {item.header ? <div className="ss-question-header">{item.header}</div> : null}
          {item.question ? <div className="ss-question-text">{item.question}</div> : null}
          {item.options.length > 0 ? (
            <div className="ss-options">
              {item.options.map((option) => (
                <Button key={option} size="sm" disabled={disabled} onClick={() => void onAnswer(option, question.request_id)}>
                  {option}
                </Button>
              ))}
            </div>
          ) : null}
        </div>
      ))}
      <div className="ss-input-row">
        <input
          className="ss-input"
          value={text}
          placeholder="Write an answer…"
          aria-label="Answer"
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && !event.nativeEvent.isComposing) {
              event.preventDefault()
              void submit()
            }
          }}
        />
        <Tooltip content="Send answer">
          <Button size="sm" variant="primary" disabled={!text.trim() || disabled} onClick={() => void submit()}>
            Answer
          </Button>
        </Tooltip>
      </div>
    </div>
  )
}

export default SessionsPage
