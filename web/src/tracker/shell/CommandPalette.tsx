import React, { useEffect, useMemo, useRef, useState } from 'react'
import {
  ArrowRight,
  Box,
  CircleDot,
  CircleUser,
  Copy,
  Inbox,
  Keyboard,
  Layers,
  Moon,
  PanelLeft,
  RefreshCw,
  Search,
  Settings,
  SquarePen,
  Sun,
  Tag,
  Trash2,
  User as UserIcon,
  Archive,
} from 'lucide-react'
import { navigate, useRoute, type Route } from '../router'
import { TrackerApiError } from '../api'
import { useTracker, type TrackerState } from '../store'
import type { ID, Issue, Priority } from '../types'
import { PRIORITY_LABELS } from '../types'
import { branchName, stateProgress } from '../format'
import { Avatar, Kbd, LabelDot, PriorityIcon, ProjectIcon, StatusIcon, toast } from '../ui'

export interface PaletteCommand {
  id: string
  label: string
  group: string
  icon?: React.ReactNode
  keywords?: string
  kbd?: string[]
  run?: () => void | Promise<void>
  page?: string
  checked?: boolean
}

export function fuzzyScore(query: string, text: string): number {
  if (!query) return 1
  const q = query.toLowerCase()
  const s = text.toLowerCase()
  const direct = s.indexOf(q)
  if (direct >= 0) return 1000 - direct * 2 - (s.length - q.length) * 0.1 + (direct === 0 || s[direct - 1] === ' ' ? 200 : 0)
  let score = 0
  let qi = 0
  let last = -2
  for (let i = 0; i < s.length && qi < q.length; i += 1) {
    if (s[i] === q[qi]) {
      score += i === last + 1 ? 6 : 1
      if (i === 0 || s[i - 1] === ' ') score += 8
      last = i
      qi += 1
    }
  }
  return qi === q.length ? score : -1
}

export function targetIssueIds(t: TrackerState, route: Route): ID[] {
  if (t.selection.size > 0) return [...t.selection]
  if (t.peekIssueId) return [t.peekIssueId]
  if (route.name === 'issue') {
    const issue = t.issuesByIdentifier.get(route.identifier) ?? t.issuesById.get(route.identifier)
    if (issue) return [issue.id]
  }
  if (t.focusedIssueId) return [t.focusedIssueId]
  return []
}

function currentTeamId(t: TrackerState, route: Route): ID | undefined {
  if (route.name === 'team-issues' || route.name === 'team-cycles') return t.teamsByKey.get(route.teamKey)?.id
  return undefined
}

async function copyText(text: string, message: string) {
  try {
    await navigator.clipboard.writeText(text)
    toast(message)
  } catch {
    toast('Could not copy to clipboard')
  }
}

function buildCommands(t: TrackerState, route: Route, page: string | null): PaletteCommand[] {
  const ids = targetIssueIds(t, route)
  const issues = ids.map((id) => t.issuesById.get(id)).filter((i): i is Issue => Boolean(i))
  const one = issues.length === 1 ? issues[0] : null
  const apply = (patch: Parameters<TrackerState['updateIssues']>[1], message: string) => async () => {
    try {
      await t.updateIssues(ids, patch)
      toast(message)
    } catch (err) {
      // updateIssues already rolled back and reported the error to the store, except for
      // lane_not_verified, which needs the "Force close" action offered here.
      if (err instanceof TrackerApiError && err.code === 'lane_not_verified' && patch.state_id) {
        toast(err.message, {
          kind: 'error',
          action: {
            label: 'Force close',
            onClick: () => {
              void t.updateIssues(ids, { ...patch, force_note: 'closed from tracker' }).catch(t.reportError)
            },
          },
        })
      }
    }
  }

  if (page === 'status') {
    const teamIds = Array.from(new Set(issues.map((i) => i.team_id)))
    const teamId = teamIds.length === 1 ? teamIds[0] : undefined
    const states = teamId ? t.teamStates(teamId) : []
    return states.map((state) => ({
      id: `status:${state.id}`,
      group: 'Change status',
      label: state.name,
      icon: <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, states)} size={14} />,
      checked: issues.every((i) => i.state_id === state.id),
      run: apply({ state_id: state.id }, `Status changed to ${state.name}`),
    }))
  }
  if (page === 'priority') {
    return ([0, 1, 2, 3, 4] as Priority[]).map((p, index) => ({
      id: `priority:${p}`,
      group: 'Change priority',
      label: PRIORITY_LABELS[p],
      icon: <PriorityIcon priority={p} size={14} />,
      kbd: [String(index)],
      checked: issues.every((i) => i.priority === p),
      run: apply({ priority: p }, `Priority set to ${PRIORITY_LABELS[p]}`),
    }))
  }
  if (page === 'assignee') {
    return [
      {
        id: 'assignee:none',
        group: 'Assign to',
        label: 'No assignee',
        icon: <Avatar user={null} size={16} />,
        checked: issues.every((i) => i.assignee_id === null),
        run: apply({ assignee_id: null }, 'Unassigned'),
      },
      ...t.users.map((user) => ({
        id: `assignee:${user.id}`,
        group: 'Assign to',
        label: user.is_me ? `${user.name} (you)` : user.name,
        keywords: `${user.display_name} ${user.email ?? ''} ${user.model ?? ''}`,
        icon: <Avatar user={user} size={16} />,
        checked: issues.every((i) => i.assignee_id === user.id),
        run: apply({ assignee_id: user.id }, `Assigned to ${user.name}`),
      })),
    ]
  }
  if (page === 'label') {
    return t.labels.map((label) => {
      const all = issues.every((i) => i.label_ids.includes(label.id))
      return {
        id: `label:${label.id}`,
        group: 'Labels',
        label: label.name,
        icon: <LabelDot color={label.color} />,
        checked: all,
        run: apply(all ? { remove_label_ids: [label.id] } : { add_label_ids: [label.id] }, all ? `Removed ${label.name}` : `Added ${label.name}`),
      }
    })
  }
  if (page === 'project') {
    return [
      { id: 'project:none', group: 'Move to project', label: 'No project', icon: <Box size={14} />, run: apply({ project_id: null }, 'Removed from project') },
      ...t.projects
        .filter((p) => !p.archived_at)
        .map((project) => ({
          id: `project:${project.id}`,
          group: 'Move to project',
          label: project.name,
          icon: <ProjectIcon icon={project.icon} color={project.color} />,
          checked: issues.every((i) => i.project_id === project.id),
          run: apply({ project_id: project.id }, `Moved to ${project.name}`),
        })),
    ]
  }

  const list: PaletteCommand[] = []
  const teamId = currentTeamId(t, route) ?? t.teams.find((team) => !team.system)?.id

  if (issues.length > 0) {
    const scope = one ? one.identifier : `${issues.length} issues`
    list.push(
      { id: 'issue:status', group: scope, label: 'Change status…', icon: <CircleDot size={14} />, kbd: ['S'], page: 'status' },
      { id: 'issue:priority', group: scope, label: 'Change priority…', icon: <PriorityIcon priority={2} size={14} />, kbd: ['P'], page: 'priority' },
      { id: 'issue:assignee', group: scope, label: 'Assign to…', icon: <UserIcon size={14} />, kbd: ['A'], page: 'assignee' },
      {
        id: 'issue:assign-me',
        group: scope,
        label: 'Assign to me',
        icon: <CircleUser size={14} />,
        kbd: ['I'],
        run: t.me ? apply({ assignee_id: t.me.id }, 'Assigned to you') : undefined,
      },
      { id: 'issue:labels', group: scope, label: 'Change labels…', icon: <Tag size={14} />, kbd: ['L'], page: 'label' },
      { id: 'issue:project', group: scope, label: 'Move to project…', icon: <Box size={14} />, kbd: ['⇧', 'P'], page: 'project' },
    )
    if (one) {
      list.push(
        { id: 'issue:open', group: scope, label: 'Open issue', icon: <ArrowRight size={14} />, kbd: ['O'], run: () => navigate(`/issue/${one.identifier}`) },
        { id: 'issue:copy-id', group: scope, label: 'Copy issue ID', icon: <Copy size={14} />, kbd: ['⌘', '.'], run: () => copyText(one.identifier, `Copied ${one.identifier}`) },
        {
          id: 'issue:copy-url',
          group: scope,
          label: 'Copy issue URL',
          icon: <Copy size={14} />,
          run: () => copyText(`${window.location.origin}${window.location.pathname}#/tracker/issue/${one.identifier}`, 'Copied issue URL'),
        },
        {
          id: 'issue:copy-branch',
          group: scope,
          label: 'Copy git branch name',
          icon: <Copy size={14} />,
          kbd: ['⌘', '⇧', ','],
          run: () => copyText(branchName(one, t.me), 'Copied branch name'),
        },
        {
          id: 'issue:sub',
          group: scope,
          label: 'Create sub-issue',
          icon: <SquarePen size={14} />,
          run: () => t.openCreate({ team_id: one.team_id, parent_id: one.id }),
        },
        {
          id: 'issue:favorite',
          group: scope,
          label: t.isFavorite('issue', one.id) ? 'Remove from favorites' : 'Add to favorites',
          icon: <Layers size={14} />,
          run: () => t.toggleFavorite('issue', one.id).catch(t.reportError),
        },
      )
    }
    list.push({
      id: 'issue:archive',
      group: scope,
      label: issues.every((i) => i.archived_at) ? 'Unarchive' : 'Archive',
      icon: <Archive size={14} />,
      kbd: ['⌘', '⇧', 'A'],
      run: apply({ archived: !issues.every((i) => i.archived_at) }, issues.every((i) => i.archived_at) ? 'Unarchived' : 'Archived'),
    })
    if (one && !one.lane_key) {
      list.push({
        id: 'issue:delete',
        group: scope,
        label: 'Delete issue',
        icon: <Trash2 size={14} />,
        kbd: ['⌘', '⌫'],
        run: async () => {
          if (!window.confirm(`Delete ${one.identifier}? This cannot be undone.`)) return
          try {
            await t.deleteIssue(one.id)
            if (t.peekIssueId === one.id) t.setPeekIssueId(null)
            toast(`Deleted ${one.identifier}`)
          } catch (err) {
            t.reportError(err)
          }
        },
      })
    }
  }

  list.push({
    id: 'create:issue',
    group: 'Issues',
    label: 'Create new issue',
    icon: <SquarePen size={14} />,
    kbd: ['C'],
    run: () => t.openCreate(teamId ? { team_id: teamId } : undefined),
  })

  const go = (id: string, label: string, path: string, icon: React.ReactNode, kbd?: string[], keywords?: string): PaletteCommand => ({
    id: `go:${id}`,
    group: 'Navigation',
    label,
    icon,
    kbd,
    keywords,
    run: () => navigate(path),
  })
  const firstTeam = t.teams.find((team) => !team.system) ?? t.teams[0]
  const navTeam = (route.name === 'team-issues' || route.name === 'team-cycles') ? t.teamsByKey.get(route.teamKey) ?? firstTeam : firstTeam
  list.push(
    go('inbox', 'Go to Inbox', '/inbox', <Inbox size={14} />, ['G', 'I']),
    go('my-issues', 'Go to My issues', '/my-issues/assigned', <CircleUser size={14} />, ['G', 'M']),
    go('projects', 'Go to Projects', '/projects', <Box size={14} />, ['G', 'P']),
    go('views', 'Go to Views', '/views', <Layers size={14} />),
    go('search', 'Go to Search', '/search', <Search size={14} />, ['/']),
    go('settings', 'Go to Settings', '/settings/preferences', <Settings size={14} />, ['G', 'S']),
  )
  if (navTeam) {
    list.push(
      go('active', `Go to ${navTeam.name} active issues`, `/team/${navTeam.key}/active`, <CircleDot size={14} />, ['G', 'A']),
      go('backlog', `Go to ${navTeam.name} backlog`, `/team/${navTeam.key}/backlog`, <CircleDot size={14} />, ['G', 'B']),
      go('cycles', `Go to ${navTeam.name} cycles`, `/team/${navTeam.key}/cycles`, <RefreshCw size={14} />, ['G', 'V']),
    )
  }
  for (const team of t.teams) {
    list.push(go(`team:${team.key}`, `Go to team ${team.name}`, `/team/${team.key}/all`, <CircleDot size={14} />, undefined, team.key))
  }
  for (const view of t.views) list.push(go(`view:${view.id}`, `Open view: ${view.name}`, `/view/${view.id}`, <Layers size={14} />))
  for (const project of t.projects.filter((p) => !p.archived_at)) {
    list.push(go(`project:${project.id}`, `Open project: ${project.name}`, `/project/${project.id}/overview`, <ProjectIcon icon={project.icon} color={project.color} />))
  }

  list.push(
    {
      id: 'pref:theme',
      group: 'Preferences',
      label: t.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme',
      keywords: 'theme appearance dark light mode',
      icon: t.theme === 'dark' ? <Sun size={14} /> : <Moon size={14} />,
      kbd: ['⌘', '⇧', 'L'],
      run: () => t.setTheme(t.theme === 'dark' ? 'light' : 'dark'),
    },
    {
      id: 'pref:sidebar',
      group: 'Preferences',
      label: t.sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar',
      icon: <PanelLeft size={14} />,
      kbd: ['['],
      run: () => t.setSidebarCollapsed(!t.sidebarCollapsed),
    },
    {
      id: 'help:shortcuts',
      group: 'Help',
      label: 'Keyboard shortcuts',
      icon: <Keyboard size={14} />,
      kbd: ['?'],
      run: () => t.setShortcutsOpen(true),
    },
    {
      id: 'app:refresh',
      group: 'Help',
      label: 'Reload data',
      icon: <RefreshCw size={14} />,
      run: () => t.refresh(),
    },
  )

  return list
}

function issueSearchCommands(t: TrackerState, query: string): PaletteCommand[] {
  if (query.trim().length < 2) return []
  return t.issues
    .filter((i) => !i.archived_at)
    .map((issue) => ({ issue, score: fuzzyScore(query, `${issue.identifier} ${issue.title}`) }))
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 8)
    .map(({ issue }) => {
      const state = t.statesById.get(issue.state_id)
      return {
        id: `open:${issue.id}`,
        group: 'Issues',
        label: `${issue.identifier} ${issue.title}`,
        icon: state ? <StatusIcon type={state.type} color={state.color} size={14} /> : <CircleDot size={14} />,
        run: () => navigate(`/issue/${issue.identifier}`),
      }
    })
}

const PAGE_PLACEHOLDER: Record<string, string> = {
  status: 'Change status to…',
  priority: 'Set priority to…',
  assignee: 'Assign to…',
  label: 'Add or remove labels…',
  project: 'Move to project…',
}

export function CommandPalette() {
  const t = useTracker()
  const route = useRoute()
  const { paletteOpen, setPaletteOpen } = t
  const [query, setQuery] = useState('')
  const [page, setPage] = useState<string | null>(null)
  const [active, setActive] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const listRef = useRef<HTMLDivElement>(null)
  const restoreFocus = useRef<HTMLElement | null>(null)

  useEffect(() => {
    if (paletteOpen) {
      restoreFocus.current = document.activeElement as HTMLElement | null
      setQuery('')
      setActive(0)
      const opener = (window as unknown as { __trackerPalettePage?: string | null }).__trackerPalettePage
      setPage(opener ?? null)
      ;(window as unknown as { __trackerPalettePage?: string | null }).__trackerPalettePage = null
      requestAnimationFrame(() => inputRef.current?.focus())
    } else {
      restoreFocus.current?.focus?.()
    }
  }, [paletteOpen])

  const close = () => setPaletteOpen(false)

  const items = useMemo(() => {
    if (!paletteOpen) return []
    const base = buildCommands(t, route, page)
    const extra = page ? [] : issueSearchCommands(t, query)
    if (!query.trim()) return base
    return [...base, ...extra]
      .map((cmd) => ({ cmd, score: fuzzyScore(query.trim(), `${cmd.label} ${cmd.keywords ?? ''}`) }))
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.cmd)
  }, [paletteOpen, t, route, page, query])

  useEffect(() => {
    setActive((current) => Math.min(current, Math.max(0, items.length - 1)))
  }, [items.length])

  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' })
  }, [active])

  if (!paletteOpen) return null

  const runItem = (cmd: PaletteCommand | undefined) => {
    if (!cmd) return
    if (cmd.page) {
      setPage(cmd.page)
      setQuery('')
      setActive(0)
      return
    }
    close()
    void cmd.run?.()
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        setActive((i) => (items.length ? (i + 1) % items.length : 0))
        break
      case 'ArrowUp':
        event.preventDefault()
        setActive((i) => (items.length ? (i - 1 + items.length) % items.length : 0))
        break
      case 'Enter':
        event.preventDefault()
        runItem(items[active])
        break
      case 'Escape':
        event.preventDefault()
        event.stopPropagation()
        if (page) {
          setPage(null)
          setQuery('')
        } else close()
        break
      case 'Backspace':
        if (!query && page) {
          event.preventDefault()
          setPage(null)
        }
        break
      default:
        if (page && !query && /^[0-9]$/.test(event.key)) {
          const index = Number(event.key)
          if (items[index]) {
            event.preventDefault()
            runItem(items[index])
          }
        }
    }
  }

  const ids = targetIssueIds(t, route)
  const contextIssues = ids.map((id) => t.issuesById.get(id)).filter((i): i is Issue => Boolean(i))

  let lastGroup = ''
  return (
    <div className="ls-palette-overlay" onMouseDown={(e) => e.target === e.currentTarget && close()}>
      <div className="ls-palette" role="dialog" aria-modal="true" aria-label="Command menu" onKeyDown={onKeyDown}>
        {contextIssues.length > 0 ? (
          <div className="ls-palette-ctx">
            {contextIssues.slice(0, 3).map((issue) => (
              <span key={issue.id} className="ls-palette-chip">
                <span className="mono">{issue.identifier}</span>
                <span style={{ maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{issue.title}</span>
              </span>
            ))}
            {contextIssues.length > 3 ? <span className="ls-palette-chip">+{contextIssues.length - 3} more</span> : null}
          </div>
        ) : null}
        <input
          ref={inputRef}
          className="ls-palette-input"
          placeholder={page ? PAGE_PLACEHOLDER[page] : 'Type a command or search…'}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setActive(0)
          }}
          role="combobox"
          aria-expanded="true"
          aria-controls="ls-palette-list"
          aria-activedescendant={items[active] ? `ls-cmd-${active}` : undefined}
        />
        <div className="ls-palette-list" id="ls-palette-list" role="listbox" ref={listRef}>
          {items.length === 0 ? <div className="ls-palette-empty">No results</div> : null}
          {items.map((cmd, index) => {
            const header = cmd.group !== lastGroup ? cmd.group : null
            lastGroup = cmd.group
            return (
              <React.Fragment key={cmd.id}>
                {header ? <div className="ls-palette-group">{header}</div> : null}
                <button
                  type="button"
                  id={`ls-cmd-${index}`}
                  data-index={index}
                  role="option"
                  aria-selected={index === active}
                  className={`ls-palette-item ${index === active ? 'is-active' : ''}`}
                  onMouseMove={() => index !== active && setActive(index)}
                  onClick={() => runItem(cmd)}
                >
                  <span className="ls-palette-item-icon">{cmd.icon}</span>
                  <span className="ls-palette-item-label">{cmd.label}</span>
                  {cmd.checked ? <span className="t-3" style={{ fontSize: 12 }}>Current</span> : null}
                  {cmd.kbd ? <Kbd keys={cmd.kbd} /> : null}
                </button>
              </React.Fragment>
            )
          })}
        </div>
      </div>
    </div>
  )
}

export function openPaletteAt(t: Pick<TrackerState, 'setPaletteOpen'>, page: string | null) {
  ;(window as unknown as { __trackerPalettePage?: string | null }).__trackerPalettePage = page
  t.setPaletteOpen(true)
}

export default CommandPalette
