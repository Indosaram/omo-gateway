import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useHotkey } from '../hotkeys'
import { CreateProjectDialog } from '../projects/CreateProjectDialog'
import { HealthLabel, ProgressRing, ProjectStatusLabel, percent, projectProgress } from '../projects/shared'
import { hrefFor, navigate } from '../router'
import { PageHeader } from '../shell/Header'
import { shortDate } from '../format'
import { useTracker } from '../store'
import type { Project, ProjectStatus } from '../types'
import { PRIORITY_LABELS } from '../types'
import { Avatar, Button, ContextMenu, PriorityIcon, ProjectIcon } from '../ui'

type Tab = 'all' | 'active' | 'planned' | 'completed'

const TABS: { id: Tab; label: string; statuses: ProjectStatus[] | null }[] = [
  { id: 'all', label: 'All projects', statuses: null },
  { id: 'active', label: 'Active', statuses: ['started', 'paused'] },
  { id: 'planned', label: 'Planned', statuses: ['backlog', 'planned'] },
  { id: 'completed', label: 'Completed', statuses: ['completed', 'canceled'] },
]

const GRID = 'minmax(220px,1fr) 120px 100px 64px 96px 120px 72px'

export function ProjectsPage() {
  const t = useTracker()
  const [tab, setTab] = useState<Tab>('all')
  const [creating, setCreating] = useState(false)

  useHotkey('n', () => setCreating(true), { enabled: !creating })

  const rows = useMemo(() => {
    const statuses = TABS.find((x) => x.id === tab)?.statuses
    return t.projects
      .filter((p) => !p.archived_at && (!statuses || statuses.includes(p.status)))
      .slice()
      .sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name))
  }, [t.projects, tab])

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title="Projects"
        actions={
          <Button variant="secondary" size="sm" icon={<Plus size={14} />} onClick={() => setCreating(true)}>
            New project
          </Button>
        }
      />
      <div className="flex h-10 shrink-0 items-center gap-1 border-b px-4">
        {TABS.map((x) => (
          <button
            key={x.id}
            type="button"
            onClick={() => setTab(x.id)}
            className="h-[26px] rounded-[6px] border px-2.5 text-[12px] w-510"
            style={{
              background: tab === x.id ? 'var(--bg-active)' : 'transparent',
              borderColor: tab === x.id ? 'var(--border-strong)' : 'var(--border)',
              color: tab === x.id ? 'var(--text-1)' : 'var(--text-3)',
            }}
          >
            {x.label}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {rows.length === 0 ? (
          <EmptyProjects onCreate={() => setCreating(true)} />
        ) : (
          <div className="min-w-[760px]">
            <div
              className="sticky top-0 z-[1] grid h-9 items-center border-b px-4 text-[12px] t-3"
              style={{ gridTemplateColumns: GRID, background: 'var(--panel)' }}
            >
              <span>Name</span>
              <span>Health</span>
              <span>Priority</span>
              <span>Lead</span>
              <span>Target date</span>
              <span>Status</span>
              <span className="text-right">Progress</span>
            </div>
            {rows.map((p) => (
              <ProjectRow key={p.id} project={p} />
            ))}
          </div>
        )}
      </div>
      <CreateProjectDialog open={creating} onOpenChange={setCreating} />
    </div>
  )
}

function ProjectRow({ project }: { project: Project }) {
  const t = useTracker()
  const lead = project.lead_id ? t.usersById.get(project.lead_id) ?? null : null
  const ratio = projectProgress(project, t.issues, t.statesById)
  const favorited = t.isFavorite('project', project.id)
  return (
    <ContextMenu
      items={[
        { id: 'open', label: 'Open project' },
        { id: 'favorite', label: favorited ? 'Remove from favorites' : 'Add to favorites' },
        { id: 'copy', label: 'Copy link' },
      ]}
      onSelect={(item) => {
        if (item.id === 'open') navigate(`/project/${project.id}/overview`)
        if (item.id === 'favorite') void t.toggleFavorite('project', project.id).catch(t.reportError)
        if (item.id === 'copy') void navigator.clipboard?.writeText(`${window.location.origin}${window.location.pathname}${hrefFor(`/project/${project.id}/overview`)}`)
      }}
    >
      <a
        href={hrefFor(`/project/${project.id}/overview`)}
        className="lin-hover grid h-11 items-center border-b px-4 text-[13px]"
        style={{ gridTemplateColumns: GRID }}
      >
        <span className="flex min-w-0 items-center gap-2.5">
          <ProjectIcon icon={project.icon} color={project.color} />
          <span className="truncate w-510 t-1">{project.name}</span>
        </span>
        <span className="text-[12px]">
          <HealthLabel health={project.latest_health} />
        </span>
        <span className="flex items-center gap-1.5 text-[12px] t-3" title={PRIORITY_LABELS[project.priority]}>
          <PriorityIcon priority={project.priority} />
          <span className="max-md:hidden">{project.priority === 0 ? '' : PRIORITY_LABELS[project.priority]}</span>
        </span>
        <span>{lead ? <Avatar user={lead} size={20} /> : <span className="t-4">-</span>}</span>
        <span className="text-[12px] t-3 tabular">{project.target_date ? shortDate(project.target_date) : ''}</span>
        <span className="text-[12px]">
          <ProjectStatusLabel status={project.status} />
        </span>
        <span className="flex items-center justify-end gap-1.5 text-[12px] t-3 tabular">
          <ProgressRing ratio={ratio} />
          {percent(ratio)}
        </span>
      </a>
    </ContextMenu>
  )
}

function EmptyProjects({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
      <div className="text-[15px] w-510 t-1">Projects</div>
      <p className="max-w-[360px] text-[13px] t-3">
        Projects are larger units of work with a clear outcome, such as a feature launch. They can be shared across multiple teams.
      </p>
      <Button variant="primary" size="sm" onClick={onCreate}>
        Create new project
      </Button>
    </div>
  )
}

export default ProjectsPage
