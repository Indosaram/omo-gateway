import { useEffect, useState } from 'react'
import { trackerApi } from '../api'
import { navigate } from '../router'
import { useTracker } from '../store'
import type { ID, Priority, ProjectStatus } from '../types'
import { PRIORITY_LABELS, PROJECT_STATUS_LABELS } from '../types'
import { Avatar, Button, Dialog, PriorityIcon, ProjectIcon, toast } from '../ui'
import { ChoiceChip, PROJECT_COLORS, PROJECT_STATUSES, ProjectStatusIcon } from './shared'

export function CreateProjectDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const t = useTracker()
  const [name, setName] = useState('')
  const [summary, setSummary] = useState('')
  const [description, setDescription] = useState('')
  const [status, setStatus] = useState<ProjectStatus>('planned')
  const [priority, setPriority] = useState<Priority>(0)
  const [color, setColor] = useState(PROJECT_COLORS[0])
  const [leadId, setLeadId] = useState<ID | null>(null)
  const [teamIds, setTeamIds] = useState<ID[]>([])
  const [startDate, setStartDate] = useState('')
  const [targetDate, setTargetDate] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (!open) return
    setName('')
    setSummary('')
    setDescription('')
    setStatus('planned')
    setPriority(0)
    setColor(PROJECT_COLORS[0])
    setLeadId(t.me?.id ?? null)
    setTeamIds(t.teams.filter((team) => !team.system).slice(0, 1).map((team) => team.id))
    setStartDate('')
    setTargetDate('')
  }, [open])

  const submit = async () => {
    if (!name.trim() || saving) return
    setSaving(true)
    try {
      const project = await trackerApi.createProject({
        name: name.trim(),
        summary,
        description,
        status,
        priority,
        color,
        icon: 'box',
        lead_id: leadId,
        team_ids: teamIds,
        start_date: startDate || null,
        target_date: targetDate || null,
      })
      t.upsertProject(project)
      onOpenChange(false)
      toast(`Project "${project.name}" created`, {
        action: { label: 'View', onClick: () => navigate(`/project/${project.id}/overview`) },
      })
    } catch (err) {
      t.reportError(err)
    } finally {
      setSaving(false)
    }
  }

  const humans = t.users.filter((u) => u.kind === 'human')

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="New project"
      width={640}
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" disabled={!name.trim() || saving} onClick={() => void submit()}>
            Create project
          </Button>
        </div>
      }
    >
      <div
        className="flex flex-col gap-3"
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault()
            void submit()
          }
        }}
      >
        <div className="flex items-center gap-2">
          <ProjectIcon icon="box" color={color} />
          <div className="flex gap-1">
            {PROJECT_COLORS.map((c) => (
              <button
                key={c}
                type="button"
                aria-label={`Color ${c}`}
                onClick={() => setColor(c)}
                className="h-4 w-4 rounded-full"
                style={{ background: c, boxShadow: c === color ? '0 0 0 2px var(--panel), 0 0 0 3px var(--text-3)' : undefined }}
              />
            ))}
          </div>
        </div>
        <input
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Project name"
          className="w-full bg-transparent text-[22px] w-590 t-1 placeholder:text-[var(--text-4)]"
        />
        <input
          value={summary}
          onChange={(e) => setSummary(e.target.value)}
          placeholder="Add a short summary..."
          className="w-full bg-transparent text-[15px] t-2 placeholder:text-[var(--text-4)]"
        />
        <div className="flex flex-wrap items-center gap-1.5">
          {PROJECT_STATUSES.map((s) => (
            <ChoiceChip key={s} active={status === s} onClick={() => setStatus(s)}>
              <ProjectStatusIcon status={s} />
              {PROJECT_STATUS_LABELS[s]}
            </ChoiceChip>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {([0, 1, 2, 3, 4] as Priority[]).map((p) => (
            <ChoiceChip key={p} active={priority === p} onClick={() => setPriority(p)}>
              <PriorityIcon priority={p} />
              {PRIORITY_LABELS[p]}
            </ChoiceChip>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="t-3 text-[12px] w-16">Lead</span>
          <ChoiceChip active={leadId === null} onClick={() => setLeadId(null)}>
            No lead
          </ChoiceChip>
          {humans.map((u) => (
            <ChoiceChip key={u.id} active={leadId === u.id} onClick={() => setLeadId(u.id)}>
              <Avatar user={u} size={16} />
              {u.display_name || u.name}
            </ChoiceChip>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="t-3 text-[12px] w-16">Teams</span>
          {t.teams.map((team) => (
            <ChoiceChip
              key={team.id}
              active={teamIds.includes(team.id)}
              onClick={() => setTeamIds((cur) => (cur.includes(team.id) ? cur.filter((x) => x !== team.id) : [...cur, team.id]))}
            >
              <span className="h-2 w-2 rounded-[2px]" style={{ background: team.color }} />
              {team.name}
            </ChoiceChip>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-3 text-[12px]">
          <label className="flex items-center gap-2 t-3">
            Start date
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="h-7 rounded-[6px] border px-2 t-2"
              style={{ background: 'var(--input)' }}
            />
          </label>
          <label className="flex items-center gap-2 t-3">
            Target date
            <input
              type="date"
              value={targetDate}
              onChange={(e) => setTargetDate(e.target.value)}
              className="h-7 rounded-[6px] border px-2 t-2"
              style={{ background: 'var(--input)' }}
            />
          </label>
        </div>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Write a description, a project brief, or collect ideas..."
          rows={5}
          className="w-full resize-none rounded-[6px] border bg-transparent p-2 text-[14px] t-2 placeholder:text-[var(--text-4)]"
        />
      </div>
    </Dialog>
  )
}
