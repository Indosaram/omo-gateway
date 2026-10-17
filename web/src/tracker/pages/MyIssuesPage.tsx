import { useMemo } from 'react'
import { IssueCollection } from '../issues/IssueCollection'
import type { MyIssuesTab } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import { Avatar } from '../ui'

const TABS: { id: MyIssuesTab; label: string }[] = [
  { id: 'assigned', label: 'Assigned' },
  { id: 'created', label: 'Created' },
  { id: 'subscribed', label: 'Subscribed' },
  { id: 'activity', label: 'Activity' },
]

export function MyIssuesPage({ tab }: { tab: MyIssuesTab }) {
  const t = useTracker()
  const meId = t.me?.id ?? null

  const issues = useMemo(() => {
    if (!meId) return []
    const live = t.issues.filter((issue) => !issue.archived_at)
    switch (tab) {
      case 'assigned':
        return live.filter((issue) => issue.assignee_id === meId)
      case 'created':
        return live.filter((issue) => issue.creator_id === meId)
      case 'subscribed':
        // TODO(FE-C): the issue list payload has no subscriber_ids; creators and assignees are auto-subscribed,
        // so this approximates subscriptions until the API exposes them on Issue.
        return live.filter((issue) => issue.creator_id === meId || issue.assignee_id === meId)
      case 'activity': {
        const since = Date.now() - 14 * 86_400_000
        return live.filter(
          (issue) =>
            (issue.creator_id === meId || issue.assignee_id === meId) && new Date(issue.updated_at).getTime() >= since,
        )
      }
    }
  }, [t.issues, meId, tab])

  return (
    <IssueCollection
      key={tab}
      issues={issues}
      viewKey={`my:${tab}`}
      showTeamTabs
      defaultDisplay={tab === 'activity' ? { ordering: 'updated', grouping: 'none' } : undefined}
      renderHeader={(toolbar) => (
        <PageHeader
          crumbs={[{ label: 'My issues', icon: <Avatar user={t.me} size={16} /> }]}
          tabs={TABS.map((x) => ({ id: x.id, label: x.label, href: `/my-issues/${x.id}` }))}
          activeTab={tab}
          tabsRight={toolbar}
        />
      )}
    />
  )
}

export default MyIssuesPage
