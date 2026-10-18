import { Layers, Plus, Star } from 'lucide-react'
import { useMemo, useState } from 'react'
import { relativeTime } from '../format'
import { SaveViewDialog } from '../issues/SaveViewDialog'
import { hrefFor } from '../router'
import { PageHeader } from '../shell/Header'
import { filterCount, useTracker } from '../store'
import { DEFAULT_DISPLAY } from '../types'
import { Avatar, Button, iconComponent } from '../ui'
import { viewIssueCount } from './ViewPage'
import '../issues/issues.css'

export function ViewsPage() {
  const t = useTracker()
  const [creating, setCreating] = useState(false)
  const views = useMemo(() => t.views.slice().sort((a, b) => a.name.localeCompare(b.name)), [t.views])

  return (
    <>
      <PageHeader
        crumbs={[{ label: 'Views', icon: <Layers size={14} /> }]}
        actions={
          <Button size="sm" variant="secondary" icon={<Plus size={14} />} onClick={() => setCreating(true)}>
            New view
          </Button>
        }
      />
      <div className="lic-scroll">
        {views.length === 0 ? (
          <div className="lic-empty">
            <div className="lic-empty-inner">
              <Layers size={28} className="t-4" />
              <div className="t-1 w-510">Views</div>
              <div className="t-3" style={{ maxWidth: 360, textAlign: 'center' }}>
                Save a set of filters and display options as a view. Apply filters on any issue list and choose Save view.
              </div>
              <Button size="sm" variant="primary" onClick={() => setCreating(true)}>
                Create view
              </Button>
            </div>
          </div>
        ) : (
          <div className="lic-views" role="list">
            <div className="lic-views-head t-3">
              <span>Name</span>
              <span>Issues</span>
              <span>Filters</span>
              <span>Owner</span>
              <span>Updated</span>
            </div>
            {views.map((view) => {
              const Icon = iconComponent(view.icon)
              const owner = t.usersById.get(view.owner_id) ?? null
              const team = view.team_id ? t.teamsById.get(view.team_id) : undefined
              const favorite = t.isFavorite('view', view.id)
              return (
                <a key={view.id} className="lic-views-row" role="listitem" href={hrefFor({ name: 'view', viewId: view.id })} data-view-row={view.id}>
                  <span className="lic-views-name">
                    <Icon size={14} color={view.color ?? 'var(--text-3)'} />
                    <span className="w-510 t-1">{view.name}</span>
                    {favorite && <Star size={12} className="t-3" fill="currentColor" />}
                    {team && <span className="t-4">{team.key}</span>}
                    {view.description && <span className="t-3 lic-views-desc">{view.description}</span>}
                  </span>
                  <span className="t-2 tabular">{viewIssueCount(view, t)}</span>
                  <span className="t-3 tabular">{filterCount(view.filters ?? {})}</span>
                  <span>
                    <Avatar user={owner} size={18} />
                  </span>
                  <span className="t-3">{relativeTime(view.updated_at)}</span>
                </a>
              )
            })}
          </div>
        )}
      </div>
      <SaveViewDialog open={creating} onOpenChange={setCreating} filters={{}} display={DEFAULT_DISPLAY} teamId={null} t={t} />
    </>
  )
}

export default ViewsPage
