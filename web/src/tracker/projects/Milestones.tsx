import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import { trackerApi } from '../api'
import { shortDate } from '../format'
import { useTracker } from '../store'
import type { Issue, Milestone } from '../types'
import { Button, IconButton } from '../ui'
import { ProgressBar, percent } from './shared'

function MilestoneGlyph({ ratio }: { ratio: number }) {
  const done = ratio >= 1
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" style={{ flexShrink: 0 }}>
      <path
        d="M7 1.5l5.5 5.5L7 12.5 1.5 7z"
        fill={done ? 'var(--accent)' : 'none'}
        stroke={done ? 'var(--accent)' : 'var(--text-3)'}
        strokeWidth="1.3"
        strokeLinejoin="round"
      />
    </svg>
  )
}

export function MilestoneList({
  projectId,
  milestones,
  issues,
  onChange,
}: {
  projectId: string
  milestones: Milestone[]
  issues: Issue[]
  onChange: (next: Milestone[]) => void
}) {
  const t = useTracker()
  const [adding, setAdding] = useState(false)
  const [name, setName] = useState('')
  const [date, setDate] = useState('')
  const [saving, setSaving] = useState(false)

  const stats = (m: Milestone) => {
    const own = issues.filter((i) => i.milestone_id === m.id && t.statesById.get(i.state_id)?.type !== 'canceled')
    const done = own.filter((i) => t.statesById.get(i.state_id)?.type === 'completed').length
    return { total: own.length, done, ratio: own.length ? done / own.length : 0 }
  }

  const add = async () => {
    if (!name.trim() || saving) return
    setSaving(true)
    try {
      const created = await trackerApi.addMilestone(projectId, { name: name.trim(), target_date: date || null })
      onChange([...milestones, created])
      setName('')
      setDate('')
      setAdding(false)
    } catch (err) {
      t.reportError(err)
    } finally {
      setSaving(false)
    }
  }

  const remove = async (id: string) => {
    try {
      await trackerApi.deleteMilestone(id)
      onChange(milestones.filter((m) => m.id !== id))
    } catch (err) {
      t.reportError(err)
    }
  }

  const sorted = milestones.slice().sort((a, b) => a.sort_order - b.sort_order)

  return (
    <section>
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-[13px] w-510 t-2">Milestones</h3>
        {!adding && (
          <Button variant="ghost" size="sm" onClick={() => setAdding(true)}>
            Add milestone
          </Button>
        )}
      </div>
      {sorted.length === 0 && !adding && <p className="text-[13px] t-4">Break the project into stages by adding milestones.</p>}
      <ul className="flex flex-col">
        {sorted.map((m) => {
          const s = stats(m)
          return (
            <li key={m.id} className="group flex items-center gap-3 border-b py-2 text-[13px]">
              <MilestoneGlyph ratio={s.ratio} />
              <span className="min-w-0 flex-1 truncate t-1">{m.name}</span>
              <span className="w-[120px] max-sm:hidden">
                <ProgressBar ratio={s.ratio} />
              </span>
              <span className="w-10 text-right text-[12px] t-3 tabular">{percent(s.ratio)}</span>
              <span className="w-[70px] text-right text-[12px] t-3 tabular">
                {s.done}/{s.total}
              </span>
              <span className="w-[64px] text-right text-[12px] t-3">{m.target_date ? shortDate(m.target_date) : ''}</span>
              <span className="opacity-0 group-hover:opacity-100" style={{ transition: 'opacity 120ms var(--ease)' }}>
                <IconButton label="Delete milestone" icon={<Trash2 size={13} />} size="sm" onClick={() => void remove(m.id)} />
              </span>
            </li>
          )
        })}
      </ul>
      {adding && (
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void add()
              if (e.key === 'Escape') setAdding(false)
            }}
            placeholder="Milestone name"
            className="h-8 min-w-[200px] flex-1 rounded-[6px] border px-2 text-[13px]"
            style={{ background: 'var(--input)' }}
          />
          <input
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            className="h-8 rounded-[6px] border px-2 text-[13px] t-2"
            style={{ background: 'var(--input)' }}
          />
          <Button variant="ghost" size="sm" onClick={() => setAdding(false)}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" disabled={!name.trim() || saving} onClick={() => void add()}>
            Add
          </Button>
        </div>
      )}
    </section>
  )
}
