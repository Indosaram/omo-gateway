import { useMemo } from 'react'
import { IssueCollection } from '../issues/IssueCollection'
import type { IssueTab } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import type { StateType } from '../types'
import { TeamIcon } from '../ui'

const TAB_TYPES: Record<IssueTab, StateType[] | undefined> = {
  all: undefined,
  active: ['unstarted', 'started'],
  backlog: ['backlog'],
  triage: ['triage'],
}

const TAB_LABELS: Record<IssueTab, string> = {
  all: 'All issues',
  active: 'Active',
  backlog: 'Backlog',
  triage: 'Triage',
}

export function TeamIssuesPage({ teamKey, tab }: { teamKey: string; tab: IssueTab }) {
  const t = useTracker()
  const team = t.teamsByKey.get(teamKey)
  const types = TAB_TYPES[tab]

  const issues = useMemo(() => {
    if (!team) return []
    return t.issues.filter((issue) => {
      if (issue.team_id !== team.id || issue.archived_at) return false
      if (!types) return true
      const state = t.statesById.get(issue.state_id)
      return state ? types.includes(state.type) : false
    })
  }, [t.issues, t.statesById, team, types])

  if (!team) return null

  const tabs: IssueTab[] = team.triage_enabled ? ['triage', 'all', 'active', 'backlog'] : ['all', 'active', 'backlog']

  return (
    <IssueCollection
      key={`${teamKey}:${tab}`}
      issues={issues}
      viewKey={`team:${teamKey}:${tab}`}
      teamId={team.id}
      stateTypes={types}
      defaultDisplay={tab === 'backlog' ? { show_empty_groups: false } : undefined}
      renderHeader={(toolbar) => (
        <PageHeader
          crumbs={[{ label: team.name, icon: <TeamIcon color={team.color} name={team.name} size={16} />, href: `/team/${team.key}/all` }]}
          title={TAB_LABELS[tab]}
          tabs={tabs.map((id) => ({ id, label: TAB_LABELS[id], href: `/team/${team.key}/${id}` }))}
          activeTab={tab}
          tabsRight={toolbar}
        />
      )}
    />
  )
}

export default TeamIssuesPage
