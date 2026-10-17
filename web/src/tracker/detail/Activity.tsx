import { forwardRef, useImperativeHandle, useMemo, useRef, useState } from 'react'
import type { MouseEvent, ReactElement } from 'react'
import {
  Archive,
  ArrowLeftRight,
  Bot,
  Calendar,
  CircleCheck,
  CircleDot,
  Copy,
  CornerDownRight,
  Gauge,
  GitBranch,
  Hexagon,
  Pencil,
  Plus,
  RotateCcw,
  Smile,
  Tag,
  Trash2,
  User as UserIcon,
} from 'lucide-react'
import { trackerApi } from '../api'
import { fullDate, relativeTime } from '../format'
import { useTracker } from '../store'
import type { TrackerState } from '../store'
import type { Activity, Comment, IssueDetail } from '../types'
import { LANE_STATUS_LABELS, PRIORITY_LABELS } from '../types'
import type { LaneStatus, Priority } from '../types'
import { Avatar, Button, IconButton, Kbd, Popover, Tooltip } from '../ui'
import { copyText, Markdown, notifyError, REACTION_EMOJIS, resolveValue, useAutosize } from './shared'

function actorName(t: TrackerState, id: string | null) {
  if (!id) return 'System'
  return t.usersById.get(id)?.name ?? 'Someone'
}

function describe(a: Activity, t: TrackerState): { icon: ReactElement; text: ReactElement } {
  const actor = <b>{actorName(t, a.actor_id)}</b>
  const to = <b>{resolveValue(a.to_value, t)}</b>
  const from = <b>{resolveValue(a.from_value, t)}</b>
  switch (a.kind) {
    case 'created':
      return { icon: <Plus size={14} />, text: <>{actor} created the issue</> }
    case 'title':
      return { icon: <Pencil size={14} />, text: <>{actor} changed the title to {<b>{a.to_value}</b>}</> }
    case 'description':
      return { icon: <Pencil size={14} />, text: <>{actor} updated the description</> }
    case 'state':
      return { icon: <CircleDot size={14} />, text: <>{actor} changed status from {from} to {to}</> }
    case 'priority': {
      const p = (v: string | null) => <b>{PRIORITY_LABELS[(Number(v ?? 0) as Priority) ?? 0] ?? v}</b>
      return { icon: <Gauge size={14} />, text: <>{actor} set priority to {p(a.to_value)}</> }
    }
    case 'assignee':
      return {
        icon: <UserIcon size={14} />,
        text: a.to_value ? <>{actor} assigned the issue to {to}</> : <>{actor} removed assignee {from}</>,
      }
    case 'estimate':
      return {
        icon: <Gauge size={14} />,
        text: a.to_value ? <>{actor} set estimate to {<b>{a.to_value} points</b>}</> : <>{actor} removed the estimate</>,
      }
    case 'label_added':
      return { icon: <Tag size={14} />, text: <>{actor} added label {to}</> }
    case 'label_removed':
      return { icon: <Tag size={14} />, text: <>{actor} removed label {from}</> }
    case 'project':
      return {
        icon: <Hexagon size={14} />,
        text: a.to_value ? <>{actor} added the issue to project {to}</> : <>{actor} removed the issue from {from}</>,
      }
    case 'cycle':
      return {
        icon: <RotateCcw size={14} />,
        text: a.to_value ? <>{actor} moved the issue to {to}</> : <>{actor} removed the issue from {from}</>,
      }
    case 'parent':
      return {
        icon: <CornerDownRight size={14} />,
        text: a.to_value ? <>{actor} set parent to {to}</> : <>{actor} removed parent {from}</>,
      }
    case 'due_date':
      return {
        icon: <Calendar size={14} />,
        text: a.to_value ? <>{actor} set due date to {to}</> : <>{actor} removed the due date</>,
      }
    case 'archived':
      return { icon: <Archive size={14} />, text: <>{actor} archived the issue</> }
    case 'unarchived':
      return { icon: <Archive size={14} />, text: <>{actor} restored the issue</> }
    case 'relation_added':
      return { icon: <ArrowLeftRight size={14} />, text: <>{actor} added relation {to}</> }
    case 'relation_removed':
      return { icon: <ArrowLeftRight size={14} />, text: <>{actor} removed relation {from}</> }
    case 'delegated':
      return { icon: <Bot size={14} />, text: <>{actor} delegated the issue to an agent{a.to_value ? <> ({<b>{a.to_value}</b>})</> : null}</> }
    case 'lane_status': {
      const lane = (v: string | null) => <b>{v ? LANE_STATUS_LABELS[v as LaneStatus] ?? v : 'none'}</b>
      return { icon: <GitBranch size={14} />, text: <>Agent lane moved from {lane(a.from_value)} to {lane(a.to_value)}</> }
    }
    default:
      return { icon: <CircleDot size={14} />, text: <>{actor} updated {a.kind.replace(/_/g, ' ')}</> }
  }
}

function Reactions({ comment, onChanged }: { comment: Comment; onChanged: () => void }) {
  const t = useTracker()
  const meId = t.me?.id
  if (comment.reactions.length === 0) return null
  return (
    <div className="ld-reactions">
      {comment.reactions
        .filter((r) => r.user_ids.length > 0)
        .map((r) => {
          const mine = meId ? r.user_ids.includes(meId) : false
          const names = r.user_ids.map((id) => t.usersById.get(id)?.name ?? 'Someone').join(', ')
          return (
            <Tooltip key={r.emoji} content={names}>
              <button
                type="button"
                className="ld-reaction"
                aria-pressed={mine}
                onClick={() => trackerApi.toggleReaction(comment.id, r.emoji).then(onChanged).catch(notifyError)}
              >
                <span>{r.emoji}</span>
                <span>{r.user_ids.length}</span>
              </button>
            </Tooltip>
          )
        })}
    </div>
  )
}

function EmojiButton({ comment, onChanged }: { comment: Comment; onChanged: () => void }) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)
  return (
    <>
      <IconButton
        label="Add reaction"
        icon={<Smile size={14} />}
        size="sm"
        onClick={(e: MouseEvent<HTMLElement>) => setAnchor(e.currentTarget)}
      />
      <Popover open={anchor !== null} onOpenChange={(o) => !o && setAnchor(null)} anchor={anchor} align="end">
        <div className="ld-emoji-grid">
          {REACTION_EMOJIS.map((emoji) => (
            <button
              key={emoji}
              type="button"
              aria-label={`React with ${emoji}`}
              onClick={() => {
                setAnchor(null)
                trackerApi.toggleReaction(comment.id, emoji).then(onChanged).catch(notifyError)
              }}
            >
              {emoji}
            </button>
          ))}
        </div>
      </Popover>
    </>
  )
}

function CommentItem({
  comment,
  isReply,
  onReply,
  onChanged,
}: {
  comment: Comment
  isReply: boolean
  onReply?: () => void
  onChanged: () => void
}) {
  const t = useTracker()
  const author = t.usersById.get(comment.user_id)
  const mine = comment.user_id === t.me?.id
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(comment.body)
  const el = useRef<HTMLTextAreaElement>(null)
  useAutosize(el, draft, 40)

  const saveEdit = async () => {
    const body = draft.trim()
    if (!body || body === comment.body) {
      setEditing(false)
      return
    }
    try {
      await trackerApi.updateComment(comment.id, { body })
      setEditing(false)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  const remove = async () => {
    if (!window.confirm('Delete this comment?')) return
    try {
      await trackerApi.deleteComment(comment.id)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  return (
    <div className={`ld-comment-item ${isReply ? 'ld-comment-reply' : ''}`} id={`comment-${comment.id}`}>
      <div className="ld-comment-head">
        <Avatar user={author ?? null} size={20} />
        <span className="ld-comment-author">{author?.name ?? 'Unknown'}</span>
        <Tooltip content={fullDate(comment.created_at)}>
          <span className="ld-comment-time">
            {relativeTime(comment.created_at)}
            {comment.edited ? ' (edited)' : ''}
          </span>
        </Tooltip>
        <span className="ld-comment-tools">
          <EmojiButton comment={comment} onChanged={onChanged} />
          {onReply && (
            <IconButton label="Reply" icon={<CornerDownRight size={14} />} size="sm" onClick={onReply} />
          )}
          {!isReply && (
            <IconButton
              label={comment.resolved_at ? 'Unresolve thread' : 'Resolve thread'}
              icon={<CircleCheck size={14} />}
              size="sm"
              active={comment.resolved_at !== null}
              onClick={() =>
                trackerApi
                  .updateComment(comment.id, { resolved: comment.resolved_at === null })
                  .then(onChanged)
                  .catch(notifyError)
              }
            />
          )}
          <IconButton
            label="Copy link to comment"
            icon={<Copy size={14} />}
            size="sm"
            onClick={() => void copyText(`${window.location.href.split('#comment-')[0]}#comment-${comment.id}`, 'Comment link copied')}
          />
          {mine && (
            <>
              <IconButton
                label="Edit"
                icon={<Pencil size={14} />}
                size="sm"
                onClick={() => {
                  setDraft(comment.body)
                  setEditing(true)
                  requestAnimationFrame(() => el.current?.focus())
                }}
              />
              <IconButton label="Delete" icon={<Trash2 size={14} />} size="sm" onClick={() => void remove()} />
            </>
          )}
        </span>
      </div>
      {editing ? (
        <div className="ld-comment-body">
          <div className="ld-composer" style={{ marginTop: 6 }}>
            <textarea
              ref={el}
              value={draft}
              aria-label="Edit comment"
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault()
                  void saveEdit()
                } else if (e.key === 'Escape') {
                  e.preventDefault()
                  e.stopPropagation()
                  setEditing(false)
                }
              }}
            />
            <div className="ld-composer-foot">
              <Button variant="ghost" size="sm" onClick={() => setEditing(false)}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" onClick={() => void saveEdit()} disabled={!draft.trim()}>
                Save
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="ld-comment-body">
          <Markdown source={comment.body} />
        </div>
      )}
      <Reactions comment={comment} onChanged={onChanged} />
    </div>
  )
}

export interface ComposerHandle {
  focus: () => void
}

export const Composer = forwardRef<
  ComposerHandle,
  { issueId: string; parentId?: string | null; inline?: boolean; onPosted: () => void; onCancel?: () => void; autoFocus?: boolean }
>(function Composer({ issueId, parentId = null, inline = false, onPosted, onCancel, autoFocus }, ref) {
  const [body, setBody] = useState('')
  const [busy, setBusy] = useState(false)
  const el = useRef<HTMLTextAreaElement>(null)
  useAutosize(el, body, inline ? 36 : 52)
  useImperativeHandle(ref, () => ({
    focus: () => {
      el.current?.focus()
      el.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    },
  }))

  const submit = async () => {
    const value = body.trim()
    if (!value || busy) return
    setBusy(true)
    try {
      await trackerApi.addComment(issueId, value, parentId)
      setBody('')
      onPosted()
    } catch (err) {
      notifyError(err)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={`ld-composer ${inline ? 'ld-composer-inline' : ''}`}>
      <textarea
        ref={el}
        value={body}
        autoFocus={autoFocus}
        placeholder={parentId ? 'Leave a reply…' : 'Leave a comment…'}
        aria-label={parentId ? 'Reply' : 'Comment'}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault()
            void submit()
          } else if (e.key === 'Escape') {
            e.preventDefault()
            e.stopPropagation()
            if (onCancel) onCancel()
            else el.current?.blur()
          }
        }}
      />
      {(body.trim() || !inline) && (
        <div className="ld-composer-foot">
          <span className="ld-composer-hint">Markdown supported</span>
          {onCancel && (
            <Button variant="ghost" size="sm" onClick={onCancel}>
              Cancel
            </Button>
          )}
          <Button variant="primary" size="sm" disabled={!body.trim() || busy} onClick={() => void submit()}>
            {parentId ? 'Reply' : 'Comment'}
          </Button>
        </div>
      )}
    </div>
  )
})

function Thread({ root, replies, onChanged }: { root: Comment; replies: Comment[]; onChanged: () => void }) {
  const t = useTracker()
  const [replying, setReplying] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const resolved = root.resolved_at !== null
  if (resolved && !expanded) {
    const author = t.usersById.get(root.user_id)
    return (
      <div className="ld-comment">
        <button type="button" className="ld-resolved" style={{ width: '100%' }} onClick={() => setExpanded(true)}>
          <CircleCheck size={14} color="var(--ok)" />
          <span>
            Resolved thread by {author?.name ?? 'Unknown'} · {replies.length + 1}{' '}
            {replies.length === 0 ? 'comment' : 'comments'}
          </span>
          <span className="ld-spacer" style={{ flex: 1 }} />
          <span>Show</span>
        </button>
      </div>
    )
  }
  return (
    <div className="ld-comment">
      <CommentItem comment={root} isReply={false} onReply={() => setReplying(true)} onChanged={onChanged} />
      {replies.map((r) => (
        <CommentItem key={r.id} comment={r} isReply onChanged={onChanged} />
      ))}
      {replying ? (
        <Composer
          issueId={root.issue_id}
          parentId={root.id}
          inline
          autoFocus
          onPosted={() => {
            setReplying(false)
            onChanged()
          }}
          onCancel={() => setReplying(false)}
        />
      ) : (
        replies.length > 0 && (
          <button type="button" className="ld-resolved" onClick={() => setReplying(true)}>
            <CornerDownRight size={14} />
            Reply…
          </button>
        )
      )}
      {resolved && (
        <button type="button" className="ld-resolved" onClick={() => setExpanded(false)}>
          Collapse resolved thread
        </button>
      )}
    </div>
  )
}

type Entry = { at: string; kind: 'activity'; activity: Activity } | { at: string; kind: 'thread'; root: Comment; replies: Comment[] }

export const ActivityFeed = forwardRef<
  ComposerHandle,
  {
    detail: IssueDetail
    subscribed: boolean
    onToggleSubscribe: () => void
    onChanged: () => void
    compact?: boolean
  }
>(function ActivityFeed({ detail, subscribed, onToggleSubscribe, onChanged, compact = false }, ref) {
  const t = useTracker()
  const composer = useRef<ComposerHandle>(null)
  const [showAll, setShowAll] = useState(false)
  useImperativeHandle(ref, () => ({ focus: () => composer.current?.focus() }))

  const entries = useMemo<Entry[]>(() => {
    const byParent = new Map<string, Comment[]>()
    for (const c of detail.comments) {
      if (c.parent_id) {
        const list = byParent.get(c.parent_id) ?? []
        list.push(c)
        byParent.set(c.parent_id, list)
      }
    }
    const threads: Entry[] = detail.comments
      .filter((c) => !c.parent_id)
      .map((root) => ({
        at: root.created_at,
        kind: 'thread' as const,
        root,
        replies: (byParent.get(root.id) ?? []).sort((a, b) => a.created_at.localeCompare(b.created_at)),
      }))
    const events: Entry[] = detail.activity
      .filter((a) => a.kind !== 'description')
      .map((a) => ({ at: a.created_at, kind: 'activity' as const, activity: a }))
    return [...threads, ...events].sort((a, b) => a.at.localeCompare(b.at))
  }, [detail.comments, detail.activity])

  const LIMIT = compact ? 6 : 30
  const hidden = showAll ? 0 : Math.max(0, entries.length - LIMIT)
  const visible = hidden > 0 ? entries.slice(hidden) : entries

  return (
    <section className="ld-activity" aria-label="Activity">
      <div className="ld-activity-head">
        <h3>Activity</h3>
        <span style={{ flex: 1 }} />
        <Button variant="ghost" size="sm" onClick={onToggleSubscribe}>
          {subscribed ? 'Unsubscribe' : 'Subscribe'}
        </Button>
      </div>
      {hidden > 0 && (
        <button type="button" className="ld-event" onClick={() => setShowAll(true)}>
          <span className="ld-event-icon">
            <Plus size={14} />
          </span>
          Show {hidden} earlier {hidden === 1 ? 'item' : 'items'}
        </button>
      )}
      {visible.map((entry) => {
        if (entry.kind === 'thread') {
          return <Thread key={entry.root.id} root={entry.root} replies={entry.replies} onChanged={onChanged} />
        }
        const { icon, text } = describe(entry.activity, t)
        return (
          <div key={`a${entry.activity.id}`} className="ld-event">
            <span className="ld-event-icon">{icon}</span>
            <span>
              {text}
              <Tooltip content={fullDate(entry.activity.created_at)}>
                <span className="ld-event-time"> · {relativeTime(entry.activity.created_at)}</span>
              </Tooltip>
            </span>
          </div>
        )
      })}
      <Composer ref={composer} issueId={detail.issue.id} onPosted={onChanged} />
      {!compact && (
        <div className="ld-composer-hint" style={{ marginTop: 6, display: 'flex', alignItems: 'center', gap: 6 }}>
          Press <Kbd keys={['M']} /> to comment, <Kbd keys={['⌘', 'Enter']} /> to submit
        </div>
      )}
    </section>
  )
})
