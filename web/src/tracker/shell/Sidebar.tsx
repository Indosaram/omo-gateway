import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Box,
  ChevronDown,
  CircleDot,
  Copy,
  Inbox,
  Layers,
  RefreshCw,
  Search,
  SquarePen,
  Target,
  CircleUser,
} from 'lucide-react'
import { hrefFor, navigate, type Route } from '../router'
import { useTracker } from '../store'
import type { Favorite, Team } from '../types'
import { cycleLabel, initials } from '../format'
import { AgentGlyph, Avatar, Kbd, LabelDot, MenuList, Popover, ProjectIcon, Tooltip } from '../ui'

const WIDTH_KEY = 'tracker.sidebarWidth'
const OPEN_KEY = 'tracker.sidebarSections'
export const SIDEBAR_MIN = 220
export const SIDEBAR_MAX = 330
export const SIDEBAR_DEFAULT = 244

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

function writeJson(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    return
  }
}

export function useSidebarWidth(): [number, (w: number) => void] {
  const [width, setWidth] = useState<number>(() => {
    const stored = readJson<number>(WIDTH_KEY, SIDEBAR_DEFAULT)
    return Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, stored))
  })
  const update = useCallback((w: number) => {
    const clamped = Math.round(Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, w)))
    setWidth(clamped)
    writeJson(WIDTH_KEY, clamped)
  }, [])
  return [width, update]
}

export function TeamIcon({ team, size = 18 }: { team: Team; size?: number }) {
  if (team.system) {
    return (
      <span className="ls-team-icon" style={{ width: size, height: size, background: 'var(--bg-active)', color: 'var(--text-2)' }}>
        <AgentGlyph size={Math.round(size * 0.7)} />
      </span>
    )
  }
  return (
    <span className="ls-team-icon" style={{ width: size, height: size, background: team.color || 'var(--accent)' }} aria-hidden>
      {team.key.slice(0, 1)}
    </span>
  )
}

function isRoute(route: Route, path: string): boolean {
  switch (route.name) {
    case 'inbox':
      return path === '/inbox'
    case 'my-issues':
      return path === '/my-issues'
    case 'projects':
    case 'project':
      return path === '/projects'
    case 'views':
      return path === '/views'
    case 'view':
      return path === `/view/${route.viewId}`
    case 'team-issues':
      return path === `/team/${route.teamKey}/issues`
    case 'team-cycles':
      return path === `/team/${route.teamKey}/cycles`
    default:
      return false
  }
}

interface ItemProps {
  href: string
  icon?: React.ReactNode
  label: React.ReactNode
  active?: boolean
  sub?: boolean
  trailing?: React.ReactNode
  onNavigate?: () => void
  title?: string
}

function NavItem({ href, icon, label, active, sub, trailing, onNavigate, title }: ItemProps) {
  return (
    <a
      className={`ls-item ${active ? 'is-active' : ''} ${sub ? 'is-sub' : ''}`}
      href={hrefFor(href)}
      aria-current={active ? 'page' : undefined}
      onClick={onNavigate}
      title={title}
    >
      {icon ? <span className="ls-item-icon">{icon}</span> : null}
      <span className="ls-item-label">{label}</span>
      {trailing}
    </a>
  )
}

function Section({
  id,
  title,
  open,
  onToggle,
  children,
  action,
}: {
  id: string
  title: React.ReactNode
  open: boolean
  onToggle: (id: string) => void
  children: React.ReactNode
  action?: React.ReactNode
}) {
  return (
    <div className="ls-section">
      <div style={{ display: 'flex', alignItems: 'center' }}>
        <button type="button" className="ls-section-head" aria-expanded={open} onClick={() => onToggle(id)}>
          <span>{title}</span>
          <ChevronDown size={12} className={`ls-caret ${open ? '' : 'is-closed'}`} />
        </button>
        {action}
      </div>
      {open ? <div>{children}</div> : null}
    </div>
  )
}

export function Sidebar({
  route,
  width,
  onResize,
  onNavigate,
}: {
  route: Route
  width: number
  onResize?: (w: number) => void
  onNavigate?: () => void
}) {
  const t = useTracker()
  const [open, setOpen] = useState<Record<string, boolean>>(() => readJson(OPEN_KEY, {}))
  const [wsMenuAnchor, setWsMenuAnchor] = useState<HTMLElement | null>(null)
  const [dragging, setDragging] = useState(false)
  const rootRef = useRef<HTMLElement>(null)

  const isOpen = (id: string, fallback = true) => open[id] ?? fallback
  const toggle = useCallback((id: string) => {
    setOpen((current) => {
      const next = { ...current, [id]: !(current[id] ?? true) }
      writeJson(OPEN_KEY, next)
      return next
    })
  }, [])

  const currentTeamKey =
    route.name === 'team-issues' || route.name === 'team-cycles'
      ? route.teamKey
      : route.name === 'settings'
        ? route.teamKey
        : null

  useEffect(() => {
    if (!currentTeamKey) return
    setOpen((current) => {
      if (current[`team:${currentTeamKey}`] !== false) return current
      const next = { ...current, [`team:${currentTeamKey}`]: true }
      writeJson(OPEN_KEY, next)
      return next
    })
  }, [currentTeamKey])

  useEffect(() => {
    if (!dragging || !onResize) return
    const onMove = (event: PointerEvent) => {
      const left = rootRef.current?.getBoundingClientRect().left ?? 0
      onResize(event.clientX - left)
    }
    const onUp = () => setDragging(false)
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    document.body.style.cursor = 'col-resize'
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
      document.body.style.cursor = ''
    }
  }, [dragging, onResize])

  const myIssueCount = useMemo(() => {
    if (!t.me) return 0
    return t.issues.filter((issue) => {
      if (issue.assignee_id !== t.me!.id || issue.archived_at) return false
      const state = t.statesById.get(issue.state_id)
      return state ? state.type !== 'completed' && state.type !== 'canceled' : true
    }).length
  }, [t.issues, t.me, t.statesById])

  const triageCounts = useMemo(() => {
    const counts = new Map<string, number>()
    for (const issue of t.issues) {
      if (issue.archived_at) continue
      if (t.statesById.get(issue.state_id)?.type === 'triage') counts.set(issue.team_id, (counts.get(issue.team_id) ?? 0) + 1)
    }
    return counts
  }, [t.issues, t.statesById])

  const favoriteEntry = (fav: Favorite): ItemProps | null => {
    switch (fav.kind) {
      case 'issue': {
        const issue = t.issuesById.get(fav.target_id)
        return issue
          ? { href: `/issue/${issue.identifier}`, icon: <CircleDot size={14} />, label: `${issue.identifier} ${issue.title}`, title: issue.title }
          : null
      }
      case 'project': {
        const project = t.projectsById.get(fav.target_id)
        return project
          ? { href: `/project/${project.id}/overview`, icon: <ProjectIcon icon={project.icon} color={project.color} />, label: project.name }
          : null
      }
      case 'view': {
        const view = t.views.find((v) => v.id === fav.target_id)
        return view ? { href: `/view/${view.id}`, icon: <Layers size={14} />, label: view.name } : null
      }
      case 'cycle': {
        const cycle = t.cyclesById.get(fav.target_id)
        return cycle ? { href: `/cycle/${cycle.id}`, icon: <RefreshCw size={14} />, label: cycleLabel(cycle) } : null
      }
      case 'team': {
        const team = t.teamsById.get(fav.target_id)
        return team ? { href: `/team/${team.key}/all`, icon: <TeamIcon team={team} size={16} />, label: team.name } : null
      }
      case 'label': {
        const label = t.labelsById.get(fav.target_id)
        return label ? { href: `/search?q=${encodeURIComponent(label.name)}`, icon: <LabelDot color={label.color} />, label: label.name } : null
      }
    }
  }

  const favorites = t.favorites
    .slice()
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((fav) => ({ fav, entry: favoriteEntry(fav) }))
    .filter((x): x is { fav: Favorite; entry: ItemProps } => x.entry !== null)

  const workspaceName = 'Omon'
  const wsMenuItems = [
    { id: 'settings', label: 'Settings', hint: 'G then S' },
    { id: 'members', label: 'Members' },
    { id: 'theme', label: t.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme' },
    { id: 'shortcuts', label: 'Keyboard shortcuts', hint: '?' },
    { id: 'dashboard', label: 'Back to dashboard' },
  ]
  const onWsSelect = (item: { id: string }) => {
    setWsMenuAnchor(null)
    switch (item.id) {
      case 'settings':
        navigate('/settings/preferences')
        break
      case 'members':
        navigate('/settings/members')
        break
      case 'theme':
        t.setTheme(t.theme === 'dark' ? 'light' : 'dark')
        break
      case 'shortcuts':
        t.setShortcutsOpen(true)
        break
      case 'dashboard':
        window.location.hash = '#/overview'
        break
    }
    onNavigate?.()
  }

  const currentTeamId = currentTeamKey ? t.teamsByKey.get(currentTeamKey)?.id : undefined

  return (
    <aside ref={rootRef} className="ls-sidebar" style={{ width }} aria-label="Sidebar">
      <div className="ls-sidebar-top">
        <button
          type="button"
          className="ls-ws"
          aria-haspopup="menu"
          aria-expanded={Boolean(wsMenuAnchor)}
          onClick={(event) => setWsMenuAnchor(wsMenuAnchor ? null : event.currentTarget)}
        >
          <span className="ls-ws-logo">{initials(workspaceName).slice(0, 1)}</span>
          <span className="ls-ws-name">{workspaceName}</span>
          <ChevronDown size={12} className="t-3" />
        </button>
        <Tooltip content="Search" kbd={['/']}>
          <button type="button" className="ls-icon-btn" aria-label="Search" onClick={() => { navigate('/search'); onNavigate?.() }}>
            <Search size={15} />
          </button>
        </Tooltip>
        <Tooltip content="Create new issue" kbd={['C']}>
          <button
            type="button"
            className="ls-icon-btn is-new"
            aria-label="Create new issue"
            onClick={() => {
              t.openCreate(currentTeamId ? { team_id: currentTeamId } : undefined)
              onNavigate?.()
            }}
          >
            <SquarePen size={15} />
          </button>
        </Tooltip>
      </div>
      <Popover open={Boolean(wsMenuAnchor)} onOpenChange={(o: boolean) => !o && setWsMenuAnchor(null)} anchor={wsMenuAnchor} align="start" width={240}>
        <MenuList items={wsMenuItems} onSelect={onWsSelect} />
      </Popover>

      <div className="ls-sidebar-scroll">
        <NavItem
          href="/inbox"
          icon={<Inbox size={15} />}
          label="Inbox"
          active={isRoute(route, '/inbox')}
          onNavigate={onNavigate}
          trailing={t.unreadCount > 0 ? <span className="ls-badge">{t.unreadCount > 99 ? '99+' : t.unreadCount}</span> : null}
        />
        <NavItem
          href="/my-issues/assigned"
          icon={<CircleUser size={15} />}
          label="My issues"
          active={isRoute(route, '/my-issues')}
          onNavigate={onNavigate}
          trailing={myIssueCount > 0 ? <span className="ls-count">{myIssueCount}</span> : null}
        />

        <Section id="workspace" title="Workspace" open={isOpen('workspace')} onToggle={toggle}>
          <NavItem href="/projects" icon={<Box size={15} />} label="Projects" active={isRoute(route, '/projects')} onNavigate={onNavigate} />
          <NavItem href="/views" icon={<Layers size={15} />} label="Views" active={isRoute(route, '/views')} onNavigate={onNavigate} />
          {t.views.slice(0, 12).map((view) => (
            <NavItem
              key={view.id}
              sub
              href={`/view/${view.id}`}
              label={view.name}
              active={isRoute(route, `/view/${view.id}`)}
              onNavigate={onNavigate}
            />
          ))}
        </Section>

        {favorites.length > 0 ? (
          <Section id="favorites" title="Favorites" open={isOpen('favorites')} onToggle={toggle}>
            {favorites.map(({ fav, entry }) => (
              <NavItem key={fav.id} {...entry} onNavigate={onNavigate} active={false} />
            ))}
          </Section>
        ) : null}

        <Section id="teams" title="Your teams" open={isOpen('teams')} onToggle={toggle}>
          {t.teams.map((team) => {
            const teamOpen = isOpen(`team:${team.key}`, true)
            const triage = triageCounts.get(team.id) ?? 0
            return (
              <div key={team.id}>
                <button
                  type="button"
                  className="ls-item"
                  aria-expanded={teamOpen}
                  onClick={() => toggle(`team:${team.key}`)}
                >
                  <span className="ls-item-icon">
                    <TeamIcon team={team} />
                  </span>
                  <span className="ls-item-label">{team.name}</span>
                  <ChevronDown size={12} className={`ls-caret t-3 ${teamOpen ? '' : 'is-closed'}`} />
                </button>
                {teamOpen ? (
                  <>
                    {team.triage_enabled ? (
                      <NavItem
                        sub
                        href={`/team/${team.key}/triage`}
                        icon={<Copy size={14} />}
                        label="Triage"
                        active={route.name === 'team-issues' && route.teamKey === team.key && route.tab === 'triage'}
                        onNavigate={onNavigate}
                        trailing={triage > 0 ? <span className="ls-count">{triage}</span> : null}
                      />
                    ) : null}
                    <NavItem
                      sub
                      href={`/team/${team.key}/all`}
                      icon={<CircleDot size={14} />}
                      label="Issues"
                      active={route.name === 'team-issues' && route.teamKey === team.key && route.tab !== 'triage'}
                      onNavigate={onNavigate}
                    />
                    {team.cycles_enabled || !team.system ? (
                      <NavItem
                        sub
                        href={`/team/${team.key}/cycles`}
                        icon={<RefreshCw size={14} />}
                        label="Cycles"
                        active={isRoute(route, `/team/${team.key}/cycles`) || (route.name === 'cycle' && t.cyclesById.get(route.cycleId)?.team_id === team.id)}
                        onNavigate={onNavigate}
                      />
                    ) : null}
                    <NavItem sub href="/projects" icon={<Target size={14} />} label="Projects" onNavigate={onNavigate} />
                  </>
                ) : null}
              </div>
            )
          })}
        </Section>
      </div>

      <div className="ls-sidebar-foot">
        <a className="ls-item" href={hrefFor('/settings/account')} onClick={onNavigate} style={{ flex: 1 }}>
          <span className="ls-item-icon">
            <Avatar user={t.me} size={16} />
          </span>
          <span className="ls-item-label">{t.me?.display_name || t.me?.name || 'Me'}</span>
        </a>
        <Tooltip content="Keyboard shortcuts" kbd={['?']}>
          <button type="button" className="ls-icon-btn ls-hide-mobile" aria-label="Keyboard shortcuts" onClick={() => t.setShortcutsOpen(true)}>
            <Kbd keys={['?']} />
          </button>
        </Tooltip>
      </div>

      {onResize ? (
        <div
          className={`ls-resizer ${dragging ? 'is-dragging' : ''}`}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize sidebar"
          aria-valuemin={SIDEBAR_MIN}
          aria-valuemax={SIDEBAR_MAX}
          aria-valuenow={width}
          onPointerDown={(event) => {
            event.preventDefault()
            setDragging(true)
          }}
          onDoubleClick={() => onResize(SIDEBAR_DEFAULT)}
        />
      ) : null}
    </aside>
  )
}

export default Sidebar
