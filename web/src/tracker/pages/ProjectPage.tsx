import { Calendar, Star } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { trackerApi } from '../api'
import { shortDate } from '../format'
import { IssueCollection } from '../issues'
import { HealthLabel, PROJECT_STATUSES, ProgressBar, ProjectStatusIcon, percent, projectIssueStats } from '../projects/shared'
import { MilestoneList } from '../projects/Milestones'
import { UpdateCard, UpdateComposer } from '../projects/Updates'
import { hrefFor, navigate, type ProjectTab } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import type { ID, Milestone, Priority, Project, ProjectDetail, ProjectUpdate } from '../types'
import { PRIORITY_LABELS, PROJECT_STATUS_LABELS } from '../types'
import type { MenuItem } from '../ui'
import { Avatar, Button, IconButton, MenuList, Popover, PriorityIcon, ProjectIcon } from '../ui'

const TABS: { id: ProjectTab; label: string }[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'issues', label: 'Issues' },
  { id: 'updates', label: 'Updates' },
]

export function ProjectPage({ projectId, tab }: { projectId: string; tab: ProjectTab }) {
  const t = useTracker()
  const project = t.projectsById.get(projectId)
  const [detail, setDetail] = useState<ProjectDetail | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const next = await trackerApi.project(projectId)
      setDetail(next)
      setLoadError(null)
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : String(err))
    }
  }, [projectId])

  useEffect(() => {
    setDetail(null)
    void load()
  }, [load])

  const current = project ?? detail?.project
  const issues = useMemo(() => t.issues.filter((i) => i.project_id === projectId && !i.archived_at), [t.issues, projectId])

  const save = async (patch: Partial<Project>) => {
    if (!current) return
    t.upsertProject({ ...current, ...patch })
    try {
      t.upsertProject(await trackerApi.updateProject(current.id, patch))
    } catch (err) {
      t.upsertProject(current)
      t.reportError(err)
    }
  }

  if (!current) {
    return (
      <div className="flex h-full flex-col">
        <PageHeader title="Project" />
        <div className="flex flex-1 items-center justify-center text-[13px] t-3">
          {loadError ? `Project not found: ${loadError}` : t.ready ? 'Loading project...' : 'Loading...'}
        </div>
      </div>
    )
  }

  const favorited = t.isFavorite('project', current.id)

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title={
          <span className="flex min-w-0 items-center gap-1.5">
            <a href={hrefFor('/projects')} className="t-3 hover:text-[var(--text-1)]">
              Projects
            </a>
            <span className="t-4">›</span>
            <ProjectIcon icon={current.icon} color={current.color} />
            <span className="truncate t-1">{current.name}</span>
          </span>
        }
        actions={
          <IconButton
            label={favorited ? 'Remove from favorites' : 'Add to favorites'}
            icon={<Star size={14} fill={favorited ? 'var(--warn)' : 'none'} color={favorited ? 'var(--warn)' : undefined} />}
            active={favorited}
            onClick={() => void t.toggleFavorite('project', current.id).catch(t.reportError)}
          />
        }
      />
      <div className="flex h-10 shrink-0 items-center gap-1 border-b px-4">
        {TABS.map((x) => (
          <a
            key={x.id}
            href={hrefFor(`/project/${current.id}/${x.id}`)}
            className="inline-flex h-[26px] items-center rounded-[6px] border px-2.5 text-[12px] w-510"
            style={{
              background: tab === x.id ? 'var(--bg-active)' : 'transparent',
              borderColor: tab === x.id ? 'var(--border-strong)' : 'var(--border)',
              color: tab === x.id ? 'var(--text-1)' : 'var(--text-3)',
            }}
          >
            {x.label}
            {x.id === 'issues' && <span className="ml-1.5 t-4 tabular">{issues.length}</span>}
          </a>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {tab === 'overview' && (
          <Overview
            project={current}
            detail={detail}
            onSave={save}
            onMilestones={(milestones) => setDetail((d) => (d ? { ...d, milestones } : d))}
          />
        )}
        {tab === 'issues' && (
          <IssueCollection
            issues={issues}
            viewKey={`project:${current.id}`}
            defaultDisplay={{ grouping: 'status' }}
            emptyState={
              <div className="flex flex-col items-center gap-3 p-10 text-center">
                <p className="text-[13px] t-3">No issues in this project yet.</p>
                <Button variant="primary" size="sm" onClick={() => t.openCreate({ project_id: current.id, team_id: current.team_ids[0] })}>
                  Create issue
                </Button>
              </div>
            }
          />
        )}
        {tab === 'updates' && (
          <UpdatesFeed
            project={current}
            updates={detail?.updates ?? []}
            onChange={(updates) => {
              setDetail((d) => (d ? { ...d, updates } : d))
              const latest = updates.slice().sort((a, b) => b.created_at.localeCompare(a.created_at))[0]
              t.upsertProject({ ...current, latest_health: latest?.health ?? null })
            }}
          />
        )}
      </div>
    </div>
  )
}

function Overview({
  project,
  detail,
  onSave,
  onMilestones,
}: {
  project: Project
  detail: ProjectDetail | null
  onSave: (patch: Partial<Project>) => Promise<void>
  onMilestones: (milestones: Milestone[]) => void
}) {
  const t = useTracker()
  const stats = projectIssueStats(project.id, t.issues, t.statesById)
  const latest = (detail?.updates ?? []).slice().sort((a, b) => b.created_at.localeCompare(a.created_at))[0]
  const issues = t.issues.filter((i) => i.project_id === project.id && !i.archived_at)

  return (
    <div className="mx-auto flex max-w-[860px] flex-col gap-6 px-6 py-8 max-md:px-4">
      <div className="flex flex-col gap-3">
        <span className="flex h-10 w-10 items-center justify-center rounded-[8px]" style={{ background: 'var(--bg-active)' }}>
          <ProjectIcon icon={project.icon} color={project.color} />
        </span>
        <EditableText
          value={project.name}
          placeholder="Project name"
          className="text-[26px] w-590 t-1"
          onCommit={(name) => name.trim() && void onSave({ name: name.trim() })}
        />
        <EditableText
          value={project.summary}
          placeholder="Add a short summary..."
          className="text-[16px] t-2"
          onCommit={(summary) => void onSave({ summary })}
        />
      </div>

      <Properties project={project} onSave={onSave} />

      <div className="flex flex-col gap-2 rounded-[8px] border p-4">
        <div className="flex items-center justify-between text-[13px]">
          <span className="w-510 t-2">Progress</span>
          <span className="t-3 tabular">
            {stats.completed} of {stats.total} issues · {percent(stats.ratio)}
          </span>
        </div>
        <ProgressBar ratio={stats.ratio} />
        <div className="flex gap-4 text-[12px] t-3">
          <span>
            <span className="mr-1 inline-block h-2 w-2 rounded-[2px]" style={{ background: 'var(--text-4)' }} />
            Scope {stats.total}
          </span>
          <span>
            <span className="mr-1 inline-block h-2 w-2 rounded-[2px]" style={{ background: 'var(--warn)' }} />
            Started {stats.started}
          </span>
          <span>
            <span className="mr-1 inline-block h-2 w-2 rounded-[2px]" style={{ background: 'var(--accent)' }} />
            Completed {stats.completed}
          </span>
        </div>
      </div>

      <section className="flex flex-col gap-2">
        <div className="flex items-center gap-2 text-[13px]">
          <span className="w-510 t-2">Latest update</span>
          <a href={hrefFor(`/project/${project.id}/updates`)} className="t-3 hover:text-[var(--text-1)]">
            {latest ? 'View all' : 'Write update'}
          </a>
        </div>
        {latest ? <UpdateCard update={latest} compact /> : <p className="text-[13px] t-4">No updates yet.</p>}
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-[13px] w-510 t-2">Description</h3>
        <EditableText
          multiline
          value={project.description}
          placeholder="Write a description, a project brief, or collect ideas..."
          className="text-[15px] t-2"
          onCommit={(description) => void onSave({ description })}
        />
      </section>

      <MilestoneList projectId={project.id} milestones={detail?.milestones ?? []} issues={issues} onChange={onMilestones} />
    </div>
  )
}

function EditableText({
  value,
  placeholder,
  className,
  multiline,
  onCommit,
}: {
  value: string
  placeholder: string
  className: string
  multiline?: boolean
  onCommit: (value: string) => void
}) {
  const [draft, setDraft] = useState(value)
  const focused = useRef(false)
  useEffect(() => {
    if (!focused.current) setDraft(value)
  }, [value])
  const commit = () => {
    focused.current = false
    if (draft !== value) onCommit(draft)
  }
  const common = {
    value: draft,
    placeholder,
    onFocus: () => {
      focused.current = true
    },
    onBlur: commit,
    className: `w-full resize-none bg-transparent placeholder:text-[var(--text-4)] ${className}`,
  }
  if (multiline) {
    return (
      <textarea
        {...common}
        rows={Math.max(3, draft.split('\n').length)}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') (e.target as HTMLTextAreaElement).blur()
        }}
      />
    )
  }
  return (
    <input
      {...common}
      onChange={(e) => setDraft(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === 'Escape') (e.target as HTMLInputElement).blur()
      }}
    />
  )
}

type PropKind = 'status' | 'priority' | 'lead' | 'teams'

function Properties({ project, onSave }: { project: Project; onSave: (patch: Partial<Project>) => Promise<void> }) {
  const t = useTracker()
  const [open, setOpen] = useState<{ kind: PropKind; anchor: HTMLElement } | null>(null)
  const lead = project.lead_id ? t.usersById.get(project.lead_id) ?? null : null

  const items = (kind: PropKind): MenuItem[] => {
    switch (kind) {
      case 'status':
        return PROJECT_STATUSES.map((s) => ({ id: s, label: PROJECT_STATUS_LABELS[s], icon: <ProjectStatusIcon status={s} /> }))
      case 'priority':
        return ([0, 1, 2, 3, 4] as Priority[]).map((p) => ({ id: String(p), label: PRIORITY_LABELS[p], icon: <PriorityIcon priority={p} /> }))
      case 'lead':
        return [
          { id: 'none', label: 'No lead' },
          ...t.users.map((u) => ({ id: u.id, label: u.display_name || u.name, icon: <Avatar user={u} size={16} /> })),
        ]
      case 'teams':
        return t.teams.map((team) => ({
          id: team.id,
          label: team.name,
          icon: <span className="block h-2.5 w-2.5 rounded-[3px]" style={{ background: team.color }} />,
        }))
    }
  }

  const select = (kind: PropKind, item: MenuItem) => {
    if (kind === 'status') void onSave({ status: item.id as Project['status'] })
    if (kind === 'priority') void onSave({ priority: Number(item.id) as Priority })
    if (kind === 'lead') void onSave({ lead_id: item.id === 'none' ? null : (item.id as ID) })
    if (kind === 'teams') {
      const next = project.team_ids.includes(item.id) ? project.team_ids.filter((id) => id !== item.id) : [...project.team_ids, item.id]
      void onSave({ team_ids: next })
      return
    }
    setOpen(null)
  }

  const chip = (kind: PropKind, content: ReactNode) => (
    <button
      type="button"
      onClick={(e) => setOpen({ kind, anchor: e.currentTarget })}
      className="lin-hover inline-flex h-7 items-center gap-1.5 rounded-[6px] px-2 text-[13px] t-2"
    >
      {content}
    </button>
  )

  const teamNames = project.team_ids.map((id) => t.teamsById.get(id)?.name).filter(Boolean)

  return (
    <div className="flex flex-wrap items-center gap-x-1 gap-y-1">
      <span className="mr-2 w-[88px] text-[13px] t-3 max-sm:w-full">Properties</span>
      {chip(
        'status',
        <>
          <ProjectStatusIcon status={project.status} />
          {PROJECT_STATUS_LABELS[project.status]}
        </>,
      )}
      {chip(
        'priority',
        <>
          <PriorityIcon priority={project.priority} />
          {PRIORITY_LABELS[project.priority]}
        </>,
      )}
      {chip(
        'lead',
        <>
          <Avatar user={lead} size={16} />
          {lead ? lead.display_name || lead.name : 'Lead'}
        </>,
      )}
      <span className="inline-flex h-7 items-center gap-1.5 px-1 text-[13px] t-2">
        <Calendar size={14} className="t-3" />
        <DateInput value={project.start_date} label="Start" onChange={(v) => void onSave({ start_date: v })} />
        <span className="t-4">→</span>
        <DateInput value={project.target_date} label="Target" onChange={(v) => void onSave({ target_date: v })} />
      </span>
      {chip('teams', <>{teamNames.length === 0 ? 'Teams' : teamNames.length > 2 ? `${teamNames.length} teams` : teamNames.join(', ')}</>)}
      <span className="ml-1 text-[13px]">
        <HealthLabel health={project.latest_health} />
      </span>
      <Popover open={open !== null} onOpenChange={(v) => !v && setOpen(null)} anchor={open?.anchor ?? null} width={240}>
        {open && (
          <MenuList
            items={items(open.kind)}
            searchable={open.kind === 'lead' || open.kind === 'teams'}
            multi={open.kind === 'teams'}
            selectedIds={
              open.kind === 'teams'
                ? project.team_ids
                : open.kind === 'status'
                  ? [project.status]
                  : open.kind === 'priority'
                    ? [String(project.priority)]
                    : [project.lead_id ?? 'none']
            }
            onSelect={(item: MenuItem) => select(open.kind, item)}
          />
        )}
      </Popover>
    </div>
  )
}

function DateInput({ value, label, onChange }: { value: string | null; label: string; onChange: (value: string | null) => void }) {
  const ref = useRef<HTMLInputElement>(null)
  return (
    <span className="relative inline-flex items-center">
      <button
        type="button"
        className="lin-hover rounded-[6px] px-1 py-0.5"
        onClick={() => {
          const input = ref.current
          if (!input) return
          if (typeof input.showPicker === 'function') input.showPicker()
          else input.focus()
        }}
      >
        {value ? shortDate(value) : <span className="t-4">{label}</span>}
      </button>
      <input
        ref={ref}
        type="date"
        aria-label={`${label} date`}
        value={value ?? ''}
        onChange={(e) => onChange(e.target.value || null)}
        className="pointer-events-none absolute inset-0 opacity-0"
        tabIndex={-1}
      />
    </span>
  )
}

function UpdatesFeed({
  project,
  updates,
  onChange,
}: {
  project: Project
  updates: ProjectUpdate[]
  onChange: (updates: ProjectUpdate[]) => void
}) {
  const t = useTracker()
  const sorted = updates.slice().sort((a, b) => b.created_at.localeCompare(a.created_at))
  return (
    <div className="mx-auto flex max-w-[760px] flex-col gap-4 px-6 py-6 max-md:px-4">
      <UpdateComposer projectId={project.id} onPosted={(u) => onChange([u, ...updates])} />
      {sorted.length === 0 && <p className="py-6 text-center text-[13px] t-4">No project updates yet. Share progress with your team.</p>}
      {sorted.map((u) => (
        <UpdateCard
          key={u.id}
          update={u}
          onDelete={
            u.user_id === t.me?.id
              ? () =>
                  void trackerApi
                    .deleteProjectUpdate(u.id)
                    .then(() => onChange(updates.filter((x) => x.id !== u.id)))
                    .catch(t.reportError)
              : undefined
          }
        />
      ))}
      {sorted.length > 0 && (
        <button type="button" className="self-center text-[12px] t-3" onClick={() => navigate(`/project/${project.id}/overview`)}>
          Back to overview
        </button>
      )}
    </div>
  )
}

export default ProjectPage
