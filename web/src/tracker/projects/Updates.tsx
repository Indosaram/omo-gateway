import { useState } from 'react'
import { trackerApi } from '../api'
import { relativeTime } from '../format'
import { useTracker } from '../store'
import type { ProjectHealth, ProjectUpdate } from '../types'
import { HEALTH_LABELS } from '../types'
import { Avatar, Button } from '../ui'
import { HEALTHS, HEALTH_COLORS, HealthIcon } from './shared'
import Markdown from 'react-markdown'

export function UpdateComposer({ projectId, onPosted }: { projectId: string; onPosted: (update: ProjectUpdate) => void }) {
  const t = useTracker()
  const [health, setHealth] = useState<ProjectHealth>('on_track')
  const [body, setBody] = useState('')
  const [saving, setSaving] = useState(false)

  const post = async () => {
    if (!body.trim() || saving) return
    setSaving(true)
    try {
      const update = await trackerApi.addProjectUpdate(projectId, { health, body: body.trim() })
      onPosted(update)
      setBody('')
    } catch (err) {
      t.reportError(err)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="rounded-[8px] border p-3" style={{ background: 'var(--card)' }}>
      <div className="mb-2 flex flex-wrap items-center gap-1">
        {HEALTHS.map((h) => (
          <button
            key={h}
            type="button"
            onClick={() => setHealth(h)}
            className="inline-flex h-7 items-center gap-1.5 rounded-[6px] px-2 text-[12px] w-510"
            style={{
              background: health === h ? 'var(--bg-active)' : 'transparent',
              color: health === h ? HEALTH_COLORS[h] : 'var(--text-3)',
            }}
          >
            <HealthIcon health={h} />
            {HEALTH_LABELS[h]}
          </button>
        ))}
      </div>
      <textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault()
            void post()
          }
        }}
        rows={4}
        placeholder="Write a project update..."
        className="w-full resize-y bg-transparent text-[14px] t-2 placeholder:text-[var(--text-4)]"
      />
      <div className="flex justify-end">
        <Button variant="primary" size="sm" disabled={!body.trim() || saving} onClick={() => void post()}>
          Post update
        </Button>
      </div>
    </div>
  )
}

export function UpdateCard({
  update,
  onDelete,
  compact,
}: {
  update: ProjectUpdate
  onDelete?: () => void
  compact?: boolean
}) {
  const t = useTracker()
  const user = t.usersById.get(update.user_id) ?? null
  return (
    <article className="rounded-[8px] border p-4" style={{ background: 'var(--card)' }}>
      <header className="mb-2 flex items-center gap-2 text-[13px]">
        <span className="inline-flex items-center gap-1.5 w-510" style={{ color: HEALTH_COLORS[update.health] }}>
          <HealthIcon health={update.health} />
          {HEALTH_LABELS[update.health]}
        </span>
        <span className="t-4">·</span>
        <Avatar user={user} size={16} />
        <span className="t-2">{user?.display_name || user?.name || 'Unknown'}</span>
        <span className="t-4">·</span>
        <span className="t-3" title={update.created_at}>
          {relativeTime(update.created_at)}
        </span>
        {onDelete && (
          <button type="button" onClick={onDelete} className="lin-hover ml-auto rounded-[6px] px-2 py-0.5 text-[12px] t-3">
            Delete
          </button>
        )}
      </header>
      <div className="lin-markdown" style={compact ? { maxHeight: 160, overflow: 'hidden' } : undefined}>
        <Markdown>{update.body}</Markdown>
      </div>
    </article>
  )
}
