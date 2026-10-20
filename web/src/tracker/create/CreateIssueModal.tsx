import { useEffect, useMemo, useRef, useState } from 'react'
import type { MouseEvent, ReactNode } from 'react'
import { Calendar, ChevronRight, Maximize2, Minimize2, X } from 'lucide-react'
import { cycleLabel, shortDate, toDateOnly } from '../format'
import { useHotkey, useHotkeyScope } from '../hotkeys'
import { navigate } from '../router'
import { teamDefaultState, useTracker } from '../store'
import type { CreateDraft } from '../store'
import type { ID, IssueCreate, Priority, Team } from '../types'
import { PRIORITY_LABELS } from '../types'
import { Avatar, Button, IconButton, LabelDot, MenuList, Popover, toast } from '../ui'
import type { MenuItem } from '../ui'
import { menuId, notifyError, useAutosize, useIsMobile } from '../detail/shared'
import {
  assigneeItems,
  cycleItems,
  dueItems,
  estimateItems,
  iconFor,
  labelItems,
  NONE,
  parseNullable,
  priorityItems,
  projectItems,
  statusItems,
} from './options'
import './create.css'

type ChipKind = 'team' | 'status' | 'priority' | 'assignee' | 'labels' | 'project' | 'cycle' | 'estimate' | 'due'

interface Form {
  team_id: ID
  title: string
  description: string
  state_id: ID | null
  priority: Priority
  assignee_id: ID | null
  label_ids: ID[]
  project_id: ID | null
  milestone_id: ID | null
  cycle_id: ID | null
  estimate: number | null
  due_date: string | null
  parent_id: ID | null
}

const CREATE_MORE_KEY = 'tracker.createMore'

function readCreateMore(): boolean {
  try {
    return localStorage.getItem(CREATE_MORE_KEY) === '1'
  } catch {
    return false
  }
}

export function CreateIssueModal() {
  const t = useTracker()
  const draft = t.createDraft
  if (!draft || !t.ready) return null
  return <CreateIssueDialog draft={draft} />
}

function initialForm(
  draft: CreateDraft,
  t: ReturnType<typeof useTracker>,
  fallbackTeam: ID,
  selectableTeams: Team[],
): Form {
  // System teams (the Sessions mirror) reject creation server-side, so never preselect one.
  const team_id =
    draft.team_id && selectableTeams.some((candidate) => candidate.id === draft.team_id) ? draft.team_id : fallbackTeam
  const state = draft.state_id && t.statesById.get(draft.state_id)?.team_id === team_id ? draft.state_id : null
  return {
    team_id,
    title: draft.title ?? '',
    description: draft.description ?? '',
    state_id: state ?? teamDefaultState(team_id, t.teamStates)?.id ?? null,
    priority: draft.priority ?? 0,
    assignee_id: draft.assignee_id ?? null,
    label_ids: draft.label_ids ?? [],
    project_id: draft.project_id ?? null,
    milestone_id: draft.milestone_id ?? null,
    cycle_id: draft.cycle_id ?? null,
    estimate: draft.estimate ?? null,
    due_date: draft.due_date ?? null,
    parent_id: draft.parent_id ?? null,
  }
}

function CreateIssueDialog({ draft }: { draft: CreateDraft }) {
  const t = useTracker()
  const mobile = useIsMobile()
  const selectableTeams = useMemo(() => t.teams.filter((team) => !team.system), [t.teams])
  const fallbackTeam = selectableTeams[0]?.id ?? ''
  const noTeam = selectableTeams.length === 0
  const [form, setForm] = useState<Form>(() => initialForm(draft, t, fallbackTeam, selectableTeams))
  const [createMore, setCreateMore] = useState(readCreateMore)
  const [expanded, setExpanded] = useState(false)
  const [busy, setBusy] = useState(false)
  const [menu, setMenu] = useState<{ kind: ChipKind; anchor: HTMLElement } | null>(null)
  const titleRef = useRef<HTMLTextAreaElement>(null)
  const descRef = useRef<HTMLTextAreaElement>(null)
  useAutosize(titleRef, form.title)
  useAutosize(descRef, form.description, expanded ? 240 : 72)
  useHotkeyScope('create-issue', true)

  useEffect(() => {
    titleRef.current?.focus()
  }, [])

  const team = t.teamsById.get(form.team_id)
  const set = (patch: Partial<Form>) => setForm((f) => ({ ...f, ...patch }))
  const close = () => t.closeCreate()

  const changeTeam = (teamId: ID) => {
    if (teamId === form.team_id) return
    const teamLabelIds = new Set(t.labels.filter((l) => l.team_id === null || l.team_id === teamId).map((l) => l.id))
    set({
      team_id: teamId,
      state_id: teamDefaultState(teamId, t.teamStates)?.id ?? null,
      cycle_id: null,
      label_ids: form.label_ids.filter((id) => teamLabelIds.has(id)),
    })
  }

  const submit = async () => {
    const title = form.title.trim()
    if (!form.team_id) {
      toast.error('Issues need a regular team: system teams (like Sessions) cannot hold them')
      return
    }
    if (!title || busy) {
      titleRef.current?.focus()
      return
    }
    setBusy(true)
    const input: IssueCreate = {
      team_id: form.team_id,
      title,
      description: form.description.trim() || undefined,
      state_id: form.state_id ?? undefined,
      priority: form.priority,
      assignee_id: form.assignee_id,
      label_ids: form.label_ids,
      project_id: form.project_id,
      milestone_id: form.milestone_id,
      cycle_id: form.cycle_id,
      estimate: form.estimate,
      due_date: form.due_date,
      parent_id: form.parent_id,
    }
    try {
      const issue = await t.createIssue(input)
      toast(`${issue.identifier} created`, {
        action: {
          label: 'View issue',
          onClick: () => navigate({ name: 'issue', identifier: issue.identifier }),
        },
      })
      if (createMore) {
        setForm((f) => ({ ...f, title: '', description: '' }))
        requestAnimationFrame(() => titleRef.current?.focus())
      } else {
        close()
      }
    } catch (err) {
      notifyError(err)
    } finally {
      setBusy(false)
    }
  }

  useHotkey('mod+enter', () => void submit(), { allowInInputs: true, scope: 'create-issue' })
  useHotkey(
    'escape',
    () => {
      if (menu) setMenu(null)
      else close()
    },
    { allowInInputs: true, scope: 'create-issue' },
  )

  const toggleCreateMore = () => {
    const next = !createMore
    setCreateMore(next)
    try {
      localStorage.setItem(CREATE_MORE_KEY, next ? '1' : '0')
    } catch (storageErr) {
      console.warn('tracker: could not persist create-more preference', storageErr)
    }
  }

  const open = (kind: ChipKind) => (e: MouseEvent<HTMLElement>) => setMenu({ kind, anchor: e.currentTarget })

  const state = form.state_id ? t.statesById.get(form.state_id) : undefined
  const assignee = form.assignee_id ? t.usersById.get(form.assignee_id) : undefined
  const project = form.project_id ? t.projectsById.get(form.project_id) : undefined
  const cycle = form.cycle_id ? t.cyclesById.get(form.cycle_id) : undefined
  const labels = form.label_ids.map((id) => t.labelsById.get(id)).filter((l) => l !== undefined)
  const parent = form.parent_id ? t.issuesById.get(form.parent_id) : undefined
  const statuses = statusItems(t, form.team_id)
  const priorities = priorityItems()
  const projects = projectItems(t, form.team_id)
  const cycles = cycleItems(t, form.team_id)
  const estimates = estimateItems()

  const menuContent = (): ReactNode => {
    if (!menu) return null
    const pick = (fn: (id: string) => void) => (item: MenuItem | string) => {
      fn(menuId(item))
      setMenu(null)
    }
    switch (menu.kind) {
      case 'team':
        return (
          <MenuList
            searchable
            placeholder="Switch team…"
            selectedIds={[form.team_id]}
            emptyText="No regular team"
            items={selectableTeams.map((tm) => ({
              id: tm.id,
              label: tm.name,
              hint: tm.key,
              icon: (
                <span className="ci-team-badge" style={{ background: tm.color }}>
                  {tm.key.slice(0, 1)}
                </span>
              ),
            }))}
            onSelect={pick(changeTeam)}
          />
        )
      case 'status':
        return (
          <MenuList
            searchable
            placeholder="Change status…"
            items={statuses}
            selectedIds={form.state_id ? [form.state_id] : []}
            onSelect={pick((id) => set({ state_id: id }))}
          />
        )
      case 'priority':
        return (
          <MenuList
            searchable
            placeholder="Set priority…"
            items={priorities}
            selectedIds={[String(form.priority)]}
            onSelect={pick((id) => set({ priority: Number(id) as Priority }))}
          />
        )
      case 'assignee':
        return (
          <MenuList
            searchable
            placeholder="Assign to…"
            items={assigneeItems(t)}
            selectedIds={[form.assignee_id ?? NONE]}
            onSelect={pick((id) => set({ assignee_id: parseNullable(id) }))}
          />
        )
      case 'labels':
        return (
          <MenuList
            searchable
            multi
            placeholder="Add labels…"
            items={labelItems(t, form.team_id)}
            selectedIds={form.label_ids}
            emptyText="No labels"
            onSelect={(item: MenuItem | string) => {
              const id = menuId(item)
              setForm((f) => ({
                ...f,
                label_ids: f.label_ids.includes(id) ? f.label_ids.filter((l) => l !== id) : [...f.label_ids, id],
              }))
            }}
          />
        )
      case 'project':
        return (
          <MenuList
            searchable
            placeholder="Add to project…"
            items={projects}
            selectedIds={[form.project_id ?? NONE]}
            onSelect={pick((id) => set({ project_id: parseNullable(id), milestone_id: null }))}
          />
        )
      case 'cycle':
        return (
          <MenuList
            searchable
            placeholder="Add to cycle…"
            items={cycles}
            selectedIds={[form.cycle_id ?? NONE]}
            emptyText="Cycles are not enabled for this team"
            onSelect={pick((id) => set({ cycle_id: parseNullable(id) }))}
          />
        )
      case 'estimate':
        return (
          <MenuList
            items={estimates}
            placeholder="Set estimate…"
            selectedIds={[form.estimate === null ? NONE : String(form.estimate)]}
            onSelect={pick((id) => set({ estimate: id === NONE ? null : Number(id) }))}
          />
        )
      case 'due':
        return (
          <div>
            <MenuList
              items={dueItems(form.due_date)}
              selectedIds={form.due_date ? [form.due_date] : []}
              onSelect={pick((id) => set({ due_date: parseNullable(id) }))}
            />
            <div className="ci-date-row">
              <input
                type="date"
                className="ci-date"
                aria-label="Custom due date"
                value={form.due_date ?? ''}
                min={toDateOnly(new Date(Date.now() - 365 * 86_400_000))}
                onChange={(e) => {
                  set({ due_date: e.target.value || null })
                  if (e.target.value) setMenu(null)
                }}
              />
            </div>
          </div>
        )
    }
  }

  const chip = (kind: ChipKind, icon: ReactNode, label: string, muted: boolean, title: string) => (
    <button
      type="button"
      className={`ci-chip ${muted ? 'ci-chip-muted' : ''}`}
      aria-label={title}
      title={title}
      onClick={open(kind)}
    >
      {icon}
      {label && <span className="ci-chip-text">{label}</span>}
    </button>
  )

  const labelIcon =
    labels.length === 0 ? (
      <LabelDot color="var(--text-4)" />
    ) : (
      <span className="ci-dots">
        {labels.slice(0, 3).map((l) => (
          <LabelDot key={l.id} color={l.color} />
        ))}
      </span>
    )

  return (
    <div className="ci-overlay anim-fade" onMouseDown={(e) => e.target === e.currentTarget && close()}>
      <div
        className={`ci-dialog anim-pop ${expanded ? 'ci-expanded' : ''} ${mobile ? 'ci-mobile' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-label="New issue"
      >
        <div className="ci-head">
          <button type="button" className="ci-team" onClick={open('team')} aria-label="Team">
            {team && (
              <span className="ci-team-badge" style={{ background: team.color }}>
                {team.key.slice(0, 1)}
              </span>
            )}
            <span>{team?.key ?? 'Team'}</span>
          </button>
          <ChevronRight size={14} className="ci-sep" />
          <span className="ci-crumb">{parent ? `Sub-issue of ${parent.identifier}` : 'New issue'}</span>
          <span style={{ flex: 1 }} />
          {!mobile && (
            <IconButton
              label={expanded ? 'Collapse' : 'Expand'}
              icon={expanded ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
              size="sm"
              onClick={() => setExpanded((v) => !v)}
            />
          )}
          <IconButton label="Close" icon={<X size={16} />} size="sm" onClick={close} />
        </div>

        <div className="ci-body">
          <textarea
            ref={titleRef}
            className="ci-title"
            rows={1}
            placeholder="Issue title"
            aria-label="Issue title"
            value={form.title}
            onChange={(e) => set({ title: e.target.value.replace(/\n/g, ' ') })}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
                e.preventDefault()
                descRef.current?.focus()
              }
            }}
          />
          <textarea
            ref={descRef}
            className="ci-desc"
            placeholder="Add description…"
            aria-label="Description"
            value={form.description}
            onChange={(e) => set({ description: e.target.value })}
          />
          {noTeam && (
            <p className="ci-notice" role="status">
              Every team here is a system team (the Sessions mirror), so there is nowhere to file an issue yet. Create a
              regular team first, then press <b>c</b> again.
            </p>
          )}
        </div>

        <div className="ci-chips">
          {chip(
            'status',
            iconFor(statuses, form.state_id),
            state?.name ?? 'Status',
            !state,
            'Change status',
          )}
          {chip(
            'priority',
            iconFor(priorities, String(form.priority)),
            form.priority === 0 ? 'Priority' : PRIORITY_LABELS[form.priority],
            form.priority === 0,
            'Set priority',
          )}
          {chip('assignee', <Avatar user={assignee ?? null} size={16} />, assignee?.name ?? 'Assignee', !assignee, 'Assign')}
          {chip(
            'labels',
            labelIcon,
            labels.length === 0 ? 'Labels' : labels.length === 1 ? labels[0].name : `${labels.length} labels`,
            labels.length === 0,
            'Add labels',
          )}
          {chip('project', iconFor(projects, form.project_id), project?.name ?? 'Project', !project, 'Add to project')}
          {(team?.cycles_enabled || cycle) &&
            chip('cycle', iconFor(cycles, form.cycle_id), cycle ? cycleLabel(cycle) : 'Cycle', !cycle, 'Add to cycle')}
          {chip(
            'estimate',
            iconFor(estimates, form.estimate === null ? NONE : String(form.estimate)),
            form.estimate === null ? '' : `${form.estimate}`,
            form.estimate === null,
            'Set estimate',
          )}
          {chip(
            'due',
            <Calendar size={14} />,
            form.due_date ? shortDate(form.due_date) : '',
            !form.due_date,
            'Set due date',
          )}
        </div>

        <div className="ci-foot">
          <span style={{ flex: 1 }} />
          <button type="button" className="ci-toggle" role="switch" aria-checked={createMore} onClick={toggleCreateMore}>
            <span className="ci-switch" data-on={createMore} />
            Create more
          </button>
          <Button variant="primary" size="md" disabled={!form.title.trim() || busy || noTeam} onClick={() => void submit()}>
            {busy ? 'Creating…' : 'Create issue'}
          </Button>
        </div>

        <Popover
          open={menu !== null}
          onOpenChange={(o) => !o && setMenu(null)}
          anchor={menu?.anchor ?? null}
          align="start"
          width={menu?.kind === 'team' ? 220 : 240}
        >
          {menuContent()}
        </Popover>
      </div>
    </div>
  )
}
