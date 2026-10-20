import React, { useEffect, useMemo, useState } from 'react'
import { ArrowDown, ArrowUp, Monitor, Moon, Plus, Sun, Trash2 } from 'lucide-react'
import { trackerApi } from '../api'
import { hrefFor, navigate, type SettingsSection } from '../router'
import { useTracker } from '../store'
import type { ID, Label, StateType, Team, User, WorkflowState } from '../types'
import { STATE_TYPE_ORDER } from '../types'
import { relativeTime, stateProgress } from '../format'
import { PageHeader } from '../shell/Header'
import { TeamIcon } from '../shell/Sidebar'
import { Avatar, Button, LabelDot, StatusIcon, toast } from '../ui'

const PALETTE = ['#5e6ad2', '#26b5ce', '#4cb782', '#f2c94c', '#fc7840', '#eb5757', '#bb87fc', '#4ea7fc', '#95a2b3', '#e2e2e2']
const STATE_TYPE_LABELS: Record<StateType, string> = {
  triage: 'Triage',
  backlog: 'Backlog',
  unstarted: 'Unstarted',
  started: 'Started',
  completed: 'Completed',
  canceled: 'Canceled',
}

function useSave() {
  const { reportError } = useTracker()
  return async <T,>(fn: () => Promise<T>, message?: string): Promise<T | undefined> => {
    try {
      const result = await fn()
      if (message) toast(message)
      return result
    } catch (err) {
      reportError(err)
      return undefined
    }
  }
}

function Switch({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label: string }) {
  return <button type="button" role="switch" aria-checked={on} aria-label={label} className={`ls-switch ${on ? 'is-on' : ''}`} onClick={() => onChange(!on)} />
}

function Row({ title, desc, children }: { title: React.ReactNode; desc?: React.ReactNode; children?: React.ReactNode }) {
  return (
    <div className="ls-row">
      <div className="ls-row-main">
        <div className="ls-row-title">{title}</div>
        {desc ? <div className="ls-row-desc">{desc}</div> : null}
      </div>
      {children}
    </div>
  )
}

function InlineText({ value, onCommit, placeholder, width = 220, ariaLabel }: { value: string; onCommit: (v: string) => void; placeholder?: string; width?: number; ariaLabel: string }) {
  const [draft, setDraft] = useState(value)
  useEffect(() => setDraft(value), [value])
  const commit = () => {
    const next = draft.trim()
    if (next && next !== value) onCommit(next)
    else setDraft(value)
  }
  return (
    <input
      className="ls-input"
      style={{ width }}
      value={draft}
      placeholder={placeholder}
      aria-label={ariaLabel}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
        if (e.key === 'Escape') {
          setDraft(value)
          ;(e.target as HTMLInputElement).blur()
        }
      }}
    />
  )
}

function ColorInput({ value, onChange, label }: { value: string; onChange: (v: string) => void; label: string }) {
  return <input type="color" className="ls-color" aria-label={label} value={value.length === 7 ? value : '#5e6ad2'} onChange={(e) => onChange(e.target.value)} />
}

function AccountSection() {
  const t = useTracker()
  const save = useSave()
  const me = t.me
  if (!me) return null
  const update = (patch: Partial<User>) =>
    save(async () => {
      const user = await trackerApi.updateUser(me.id, patch)
      t.upsertUser(user)
      await t.refresh()
    }, 'Profile updated')
  return (
    <>
      <h1 className="ls-h1">Profile</h1>
      <p className="ls-sub">Manage how you appear to teammates and agents.</p>
      <div className="ls-card">
        <Row title="Avatar" desc="Shown next to issues you create or are assigned to.">
          <Avatar user={me} size={32} />
          <ColorInput label="Avatar color" value={me.avatar_color} onChange={(avatar_color) => void update({ avatar_color })} />
        </Row>
        <Row title="Full name">
          <InlineText ariaLabel="Full name" value={me.name} onCommit={(name) => void update({ name })} />
        </Row>
        <Row title="Username" desc="Used for mentions and git branch names.">
          <InlineText ariaLabel="Username" value={me.display_name} onCommit={(display_name) => void update({ display_name })} />
        </Row>
        <Row title="Email">
          <InlineText ariaLabel="Email" value={me.email ?? ''} placeholder="you@example.com" onCommit={(email) => void update({ email })} />
        </Row>
      </div>
    </>
  )
}

function PreferencesSection() {
  const t = useTracker()
  return (
    <>
      <h1 className="ls-h1">Preferences</h1>
      <p className="ls-sub">Customize the look and behavior of the app.</p>
      <h2 className="ls-h2">Interface and theme</h2>
      <div className="ls-card">
        <Row title="Interface theme" desc="Select or customize your interface color scheme.">
          <div className="ls-seg" role="radiogroup" aria-label="Interface theme">
            <button type="button" role="radio" aria-checked={t.themePreference === 'system'} className={t.themePreference === 'system' ? 'is-on' : ''} onClick={() => t.setTheme('system')}>
              <Monitor size={13} /> System
            </button>
            <button type="button" role="radio" aria-checked={t.themePreference === 'light'} className={t.themePreference === 'light' ? 'is-on' : ''} onClick={() => t.setTheme('light')}>
              <Sun size={13} /> Light
            </button>
            <button type="button" role="radio" aria-checked={t.themePreference === 'dark'} className={t.themePreference === 'dark' ? 'is-on' : ''} onClick={() => t.setTheme('dark')}>
              <Moon size={13} /> Dark
            </button>
          </div>
        </Row>
        <Row title="Show sidebar" desc="Toggle with [ from anywhere.">
          <Switch label="Show sidebar" on={!t.sidebarCollapsed} onChange={(v) => t.setSidebarCollapsed(!v)} />
        </Row>
      </div>
      <h2 className="ls-h2">Keyboard</h2>
      <div className="ls-card">
        <Row title="Keyboard shortcuts" desc="See every shortcut available in the app.">
          <Button variant="secondary" size="sm" onClick={() => t.setShortcutsOpen(true)}>
            View shortcuts
          </Button>
        </Row>
      </div>
    </>
  )
}

function TeamsSection() {
  const t = useTracker()
  const save = useSave()
  const [key, setKey] = useState('')
  const [name, setName] = useState('')
  const valid = /^[A-Z][A-Z0-9]{0,4}$/.test(key) && name.trim().length > 0 && !t.teamsByKey.has(key)
  const create = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!valid) return
    const team = await save(() => trackerApi.createTeam({ key, name: name.trim(), color: PALETTE[t.teams.length % PALETTE.length] }), `Created team ${name.trim()}`)
    if (team) {
      setKey('')
      setName('')
      await t.refresh()
    }
  }
  return (
    <>
      <h1 className="ls-h1">Teams</h1>
      <p className="ls-sub">Teams group issues, workflows and cycles.</p>
      <div className="ls-card">
        {t.teams.map((team) => {
          const count = t.issues.filter((i) => i.team_id === team.id && !i.archived_at).length
          return (
            <Row key={team.id} title={<span style={{ display: 'inline-flex', gap: 8, alignItems: 'center' }}><TeamIcon team={team} />{team.name}<span className="t-3 mono" style={{ fontSize: 12 }}>{team.key}</span></span>} desc={`${count} issues${team.system ? ' · Managed by agent lanes' : ''}`}>
              <Button variant="ghost" size="sm" onClick={() => navigate(`/settings/team/${team.key}`)}>
                Settings
              </Button>
            </Row>
          )
        })}
      </div>
      <h2 className="ls-h2">Create team</h2>
      <form className="ls-card" onSubmit={create}>
        <Row title="Name and identifier" desc="Identifiers are 1-5 uppercase letters and prefix every issue ID.">
          <input className="ls-input" style={{ width: 160 }} placeholder="Team name" aria-label="Team name" value={name} onChange={(e) => setName(e.target.value)} />
          <input
            className="ls-input mono"
            style={{ width: 80 }}
            placeholder="KEY"
            aria-label="Team identifier"
            value={key}
            onChange={(e) => setKey(e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 5))}
          />
          <Button variant="primary" size="sm" type="submit" disabled={!valid}>
            Create
          </Button>
        </Row>
      </form>
    </>
  )
}

function TeamSection({ team }: { team: Team }) {
  const t = useTracker()
  const save = useSave()
  const update = (patch: Partial<Team>) =>
    save(async () => {
      const next = await trackerApi.updateTeam(team.id, patch)
      t.upsertTeam(next)
    }, 'Team updated')
  return (
    <>
      <h1 className="ls-h1" style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
        <TeamIcon team={team} size={24} />
        {team.name}
      </h1>
      <p className="ls-sub">
        Team settings · <a className="t-2" href={hrefFor(`/settings/workflow/${team.key}`)}>Workflow</a>
      </p>
      <div className="ls-card">
        <Row title="Name">
          <InlineText ariaLabel="Team name" value={team.name} onCommit={(name) => void update({ name })} />
        </Row>
        <Row title="Identifier" desc="Changing the identifier is not supported once issues exist.">
          <span className="mono t-2">{team.key}</span>
        </Row>
        <Row title="Color">
          <ColorInput label="Team color" value={team.color} onChange={(color) => void update({ color })} />
        </Row>
      </div>
      <h2 className="ls-h2">Features</h2>
      <div className="ls-card">
        <Row title="Cycles" desc="Time-boxed iterations for planning work.">
          <Switch label="Enable cycles" on={team.cycles_enabled} onChange={(cycles_enabled) => void update({ cycles_enabled })} />
        </Row>
        {team.cycles_enabled ? (
          <Row title="Cycle duration">
            <select className="ls-select" aria-label="Cycle duration" value={team.cycle_weeks} onChange={(e) => void update({ cycle_weeks: Number(e.target.value) })}>
              {[1, 2, 3, 4, 6, 8].map((w) => (
                <option key={w} value={w}>
                  {w} {w === 1 ? 'week' : 'weeks'}
                </option>
              ))}
            </select>
          </Row>
        ) : null}
        <Row title="Triage" desc="Route new issues from integrations and other teams into a triage inbox.">
          <Switch label="Enable triage" on={team.triage_enabled} onChange={(triage_enabled) => void update({ triage_enabled })} />
        </Row>
      </div>
    </>
  )
}

function WorkflowSection({ team }: { team: Team }) {
  const t = useTracker()
  const save = useSave()
  const states = t.teamStates(team.id)
  const [adding, setAdding] = useState<StateType | null>(null)
  const [newName, setNewName] = useState('')
  const locked = team.system

  const updateState = (state: WorkflowState, patch: Partial<WorkflowState>) =>
    save(async () => {
      const next = await trackerApi.updateState(state.id, patch)
      t.upsertState(next)
    })

  const move = (state: WorkflowState, dir: -1 | 1) => {
    const siblings = states.filter((s) => s.type === state.type)
    const index = siblings.findIndex((s) => s.id === state.id)
    const other = siblings[index + dir]
    if (!other) return
    void updateState(state, { position: other.position })
    void updateState(other, { position: state.position })
  }

  const remove = async (state: WorkflowState) => {
    const used = t.issues.some((i) => i.state_id === state.id)
    if (used) {
      toast(`Move issues out of ${state.name} before deleting it`)
      return
    }
    await save(async () => {
      await trackerApi.deleteState(state.id)
      t.removeState(state.id)
    }, `Deleted ${state.name}`)
  }

  const add = async (type: StateType) => {
    const name = newName.trim()
    if (!name) return
    const last = states.filter((s) => s.type === type).at(-1)
    const created = await save(
      () => trackerApi.createState({ team_id: team.id, name, type, color: last?.color ?? '#95a2b3', position: (last?.position ?? 0) + 1 }),
      `Added ${name}`,
    )
    if (created) {
      t.upsertState(created)
      setNewName('')
      setAdding(null)
    }
  }

  const types = STATE_TYPE_ORDER.filter((type) => type !== 'triage' || team.triage_enabled || states.some((s) => s.type === 'triage'))
  return (
    <>
      <h1 className="ls-h1">Workflow</h1>
      <p className="ls-sub">
        {team.name} issue statuses.{' '}
        {locked ? 'This team mirrors agent lanes, so its workflow is managed by the gateway.' : 'Statuses are grouped by category.'}
      </p>
      {types.map((type) => {
        const group = states.filter((s) => s.type === type)
        return (
          <section key={type}>
            <h2 className="ls-h2" style={{ display: 'flex', alignItems: 'center' }}>
              <span style={{ flex: 1 }}>{STATE_TYPE_LABELS[type]}</span>
              {!locked ? (
                <button type="button" className="ls-icon-btn" aria-label={`Add ${STATE_TYPE_LABELS[type]} status`} onClick={() => setAdding(type)}>
                  <Plus size={14} />
                </button>
              ) : null}
            </h2>
            <div className="ls-card">
              {group.map((state, index) => (
                <div className="ls-row" key={state.id}>
                  <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, states)} size={14} />
                  {locked ? (
                    <span className="ls-row-main ls-row-title">{state.name}</span>
                  ) : (
                    <div className="ls-row-main">
                      <InlineText ariaLabel={`${state.name} name`} value={state.name} width={200} onCommit={(name) => void updateState(state, { name })} />
                    </div>
                  )}
                  {state.lane_status ? <span className="t-3 mono" style={{ fontSize: 11 }}>{state.lane_status}</span> : null}
                  <span className="t-3" style={{ fontSize: 12 }}>
                    {t.issues.filter((i) => i.state_id === state.id && !i.archived_at).length} issues
                  </span>
                  {!locked ? (
                    <>
                      <ColorInput label={`${state.name} color`} value={state.color} onChange={(color) => void updateState(state, { color })} />
                      <button type="button" className="ls-icon-btn" aria-label="Move up" disabled={index === 0} onClick={() => move(state, -1)}>
                        <ArrowUp size={13} />
                      </button>
                      <button type="button" className="ls-icon-btn" aria-label="Move down" disabled={index === group.length - 1} onClick={() => move(state, 1)}>
                        <ArrowDown size={13} />
                      </button>
                      <button type="button" className="ls-icon-btn" aria-label={`Delete ${state.name}`} disabled={group.length <= 1 && type !== 'triage'} onClick={() => void remove(state)}>
                        <Trash2 size={13} />
                      </button>
                    </>
                  ) : null}
                </div>
              ))}
              {adding === type ? (
                <form
                  className="ls-row"
                  onSubmit={(e) => {
                    e.preventDefault()
                    void add(type)
                  }}
                >
                  <input autoFocus className="ls-input" style={{ flex: 1 }} placeholder="Status name" aria-label="New status name" value={newName} onChange={(e) => setNewName(e.target.value)} onKeyDown={(e) => e.key === 'Escape' && setAdding(null)} />
                  <Button variant="ghost" size="sm" type="button" onClick={() => setAdding(null)}>
                    Cancel
                  </Button>
                  <Button variant="primary" size="sm" type="submit" disabled={!newName.trim()}>
                    Add
                  </Button>
                </form>
              ) : null}
              {group.length === 0 && adding !== type ? <Row title={<span className="t-3">No statuses</span>} /> : null}
            </div>
          </section>
        )
      })}
    </>
  )
}

function LabelsSection() {
  const t = useTracker()
  const save = useSave()
  const [query, setQuery] = useState('')
  const [name, setName] = useState('')
  const [color, setColor] = useState(PALETTE[0])
  const [teamId, setTeamId] = useState<ID | ''>('')
  const filtered = useMemo(
    () => t.labels.filter((l) => l.name.toLowerCase().includes(query.trim().toLowerCase())).sort((a, b) => a.name.localeCompare(b.name)),
    [t.labels, query],
  )
  const usage = (label: Label) => t.issues.filter((i) => i.label_ids.includes(label.id)).length
  const create = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!name.trim()) return
    const label = await save(() => trackerApi.createLabel({ name: name.trim(), color, team_id: teamId || null }), `Created label ${name.trim()}`)
    if (label) {
      t.upsertLabel(label)
      setName('')
      setColor(PALETTE[(PALETTE.indexOf(color) + 1) % PALETTE.length])
    }
  }
  const update = (label: Label, patch: Partial<Label>) =>
    save(async () => {
      const next = await trackerApi.updateLabel(label.id, patch)
      t.upsertLabel(next)
    })
  const remove = async (label: Label) => {
    if (!window.confirm(`Delete label "${label.name}"? It will be removed from ${usage(label)} issues.`)) return
    await save(async () => {
      await trackerApi.deleteLabel(label.id)
      t.removeLabel(label.id)
      await t.refreshIssues()
    }, `Deleted ${label.name}`)
  }
  return (
    <>
      <h1 className="ls-h1">Labels</h1>
      <p className="ls-sub">Workspace labels are available to every team; team labels only within their team.</p>
      <form className="ls-card" onSubmit={create} style={{ marginBottom: 16 }}>
        <div className="ls-row">
          <ColorInput label="New label color" value={color} onChange={setColor} />
          <input className="ls-input" style={{ flex: 1 }} placeholder="Label name" aria-label="New label name" value={name} onChange={(e) => setName(e.target.value)} />
          <select className="ls-select" aria-label="Label scope" value={teamId} onChange={(e) => setTeamId(e.target.value)}>
            <option value="">Workspace</option>
            {t.teams.map((team) => (
              <option key={team.id} value={team.id}>
                {team.name}
              </option>
            ))}
          </select>
          <Button variant="primary" size="sm" type="submit" disabled={!name.trim()}>
            Create label
          </Button>
        </div>
      </form>
      <input className="ls-input" style={{ width: '100%', marginBottom: 10 }} placeholder="Filter labels…" aria-label="Filter labels" value={query} onChange={(e) => setQuery(e.target.value)} />
      <div className="ls-card">
        {filtered.length === 0 ? <Row title={<span className="t-3">No labels</span>} /> : null}
        {filtered.map((label) => (
          <div className="ls-row" key={label.id}>
            <LabelDot color={label.color} />
            <div className="ls-row-main">
              <InlineText ariaLabel={`${label.name} name`} value={label.name} width={220} onCommit={(n) => void update(label, { name: n })} />
            </div>
            <span className="t-3" style={{ fontSize: 12 }}>{label.team_id ? t.teamsById.get(label.team_id)?.name ?? 'Team' : 'Workspace'}</span>
            <span className="t-3" style={{ fontSize: 12, width: 72, textAlign: 'right' }}>{usage(label)} issues</span>
            <ColorInput label={`${label.name} color`} value={label.color} onChange={(c) => void update(label, { color: c })} />
            <button type="button" className="ls-icon-btn" aria-label={`Delete ${label.name}`} onClick={() => void remove(label)}>
              <Trash2 size={13} />
            </button>
          </div>
        ))}
      </div>
    </>
  )
}

function MembersSection() {
  const t = useTracker()
  const save = useSave()
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const humans = t.users.filter((u) => u.kind === 'human')
  const agents = t.users.filter((u) => u.kind === 'agent')
  const invite = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!name.trim()) return
    const user = await save(
      () => trackerApi.createUser({ name: name.trim(), email: email.trim() || undefined, avatar_color: PALETTE[t.users.length % PALETTE.length] }),
      `Added ${name.trim()}`,
    )
    if (user) {
      t.upsertUser(user)
      setName('')
      setEmail('')
    }
  }
  const assigned = (u: User) => t.issues.filter((i) => i.assignee_id === u.id && !i.archived_at).length
  const renderUser = (u: User) => (
    <div className="ls-row" key={u.id}>
      <Avatar user={u} size={24} />
      <div className="ls-row-main">
        <div className="ls-row-title">
          {u.name}
          {u.is_me ? <span className="t-3"> (you)</span> : null}
        </div>
        <div className="ls-row-desc">{u.kind === 'agent' ? u.model ?? 'Agent' : u.email ?? u.display_name}</div>
      </div>
      <span className="t-3" style={{ fontSize: 12 }}>{assigned(u)} assigned</span>
      <span className="t-4" style={{ fontSize: 12, width: 110, textAlign: 'right' }}>Joined {relativeTime(u.created_at)}</span>
    </div>
  )
  return (
    <>
      <h1 className="ls-h1">Members</h1>
      <p className="ls-sub">People and agents in this workspace.</p>
      <form className="ls-card" onSubmit={invite} style={{ marginBottom: 8 }}>
        <div className="ls-row">
          <input className="ls-input" style={{ flex: 1 }} placeholder="Name" aria-label="Member name" value={name} onChange={(e) => setName(e.target.value)} />
          <input className="ls-input" style={{ flex: 1 }} placeholder="Email (optional)" aria-label="Member email" value={email} onChange={(e) => setEmail(e.target.value)} />
          <Button variant="primary" size="sm" type="submit" disabled={!name.trim()}>
            Add member
          </Button>
        </div>
      </form>
      <h2 className="ls-h2">People · {humans.length}</h2>
      <div className="ls-card">{humans.map(renderUser)}</div>
      <h2 className="ls-h2">Agents · {agents.length}</h2>
      <div className="ls-card">
        {agents.length === 0 ? <Row title={<span className="t-3">No agents yet</span>} desc="Agents appear when a lane starts work." /> : agents.map(renderUser)}
      </div>
    </>
  )
}

export function SettingsPage({ section, teamKey }: { section: SettingsSection; teamKey?: string | null }) {
  const t = useTracker()
  const team = teamKey ? t.teamsByKey.get(teamKey) : undefined
  const nav: { id: string; label: string; href: string; active: boolean }[] = [
    { id: 'account', label: 'Profile', href: '/settings/account', active: section === 'account' },
    { id: 'preferences', label: 'Preferences', href: '/settings/preferences', active: section === 'preferences' },
    { id: 'members', label: 'Members', href: '/settings/members', active: section === 'members' },
    { id: 'labels', label: 'Labels', href: '/settings/labels', active: section === 'labels' },
    { id: 'teams', label: 'Teams', href: '/settings/teams', active: section === 'teams' },
  ]

  let body: React.ReactNode
  switch (section) {
    case 'account':
      body = <AccountSection />
      break
    case 'preferences':
      body = <PreferencesSection />
      break
    case 'teams':
      body = <TeamsSection />
      break
    case 'labels':
      body = <LabelsSection />
      break
    case 'members':
      body = <MembersSection />
      break
    case 'team':
      body = team ? <TeamSection team={team} /> : <p className="t-3">Team not found.</p>
      break
    case 'workflow':
      body = team ? <WorkflowSection team={team} /> : <p className="t-3">Team not found.</p>
      break
  }

  return (
    <>
      <PageHeader crumbs={[{ label: 'Settings', href: '/settings/preferences' }]} title={team && (section === 'team' || section === 'workflow') ? `${team.name} · ${section === 'team' ? 'General' : 'Workflow'}` : undefined} />
      <div className="ls-settings">
        <nav className="ls-settings-nav ls-hide-mobile" aria-label="Settings">
          <div className="ls-section-head" style={{ cursor: 'default' }}>Account</div>
          {nav.slice(0, 2).map((item) => (
            <a key={item.id} className={`ls-item ${item.active ? 'is-active' : ''}`} href={hrefFor(item.href)}>
              <span className="ls-item-label">{item.label}</span>
            </a>
          ))}
          <div className="ls-section-head" style={{ cursor: 'default', marginTop: 12 }}>Workspace</div>
          {nav.slice(2).map((item) => (
            <a key={item.id} className={`ls-item ${item.active ? 'is-active' : ''}`} href={hrefFor(item.href)}>
              <span className="ls-item-label">{item.label}</span>
            </a>
          ))}
          <div className="ls-section-head" style={{ cursor: 'default', marginTop: 12 }}>Your teams</div>
          {t.teams.map((tm) => (
            <React.Fragment key={tm.id}>
              <a className={`ls-item ${section === 'team' && teamKey === tm.key ? 'is-active' : ''}`} href={hrefFor(`/settings/team/${tm.key}`)}>
                <span className="ls-item-icon">
                  <TeamIcon team={tm} size={16} />
                </span>
                <span className="ls-item-label">{tm.name}</span>
              </a>
              {teamKey === tm.key ? (
                <a className={`ls-item is-sub ${section === 'workflow' ? 'is-active' : ''}`} href={hrefFor(`/settings/workflow/${tm.key}`)}>
                  <span className="ls-item-label">Workflow</span>
                </a>
              ) : null}
            </React.Fragment>
          ))}
        </nav>
        <div className="ls-settings-body">
          <div className="ls-settings-inner">{body}</div>
        </div>
      </div>
    </>
  )
}

export default SettingsPage
