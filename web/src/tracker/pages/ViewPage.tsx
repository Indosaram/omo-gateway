import { Layers, Star, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { trackerApi } from '../api'
import { IssueCollection } from '../issues/IssueCollection'
import { errorMessage } from '../issues/model'
import { navigate } from '../router'
import { PageHeader } from '../shell/Header'
import { applyFilters, useTracker } from '../store'
import type { View, ViewDisplay, ViewFilters } from '../types'
import { DEFAULT_DISPLAY } from '../types'
import { Button, Dialog, IconButton, iconComponent, toast } from '../ui'

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

function ViewBody({ view }: { view: View }) {
  const t = useTracker()
  const viewKey = `view:${view.id}`
  const seeded = useRef<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (seeded.current === view.id) return
    seeded.current = view.id
    t.setFiltersFor(viewKey, view.filters ?? {})
    t.setDisplayFor(viewKey, { ...DEFAULT_DISPLAY, ...view.display })
  }, [view.id, view.filters, view.display, viewKey, t])

  const issues = useMemo(
    () => t.issues.filter((issue) => !issue.archived_at && (!view.team_id || issue.team_id === view.team_id)),
    [t.issues, view.team_id],
  )

  const currentFilters = t.filtersFor(viewKey)
  const currentDisplay = t.displayFor(viewKey, view.display)
  const dirty = !sameJson(currentFilters, view.filters ?? {}) || !sameJson(currentDisplay, { ...DEFAULT_DISPLAY, ...view.display })
  const Icon = iconComponent(view.icon)
  const favorite = t.isFavorite('view', view.id)

  const saveChanges = async (filters: ViewFilters, display: ViewDisplay) => {
    setSaving(true)
    try {
      const updated = await trackerApi.updateView(view.id, { filters, display })
      t.upsertView(updated)
      // The server returns filters with sorted keys; re-seed so the saved view is not dirty.
      t.setFiltersFor(viewKey, updated.filters ?? {})
      t.setDisplayFor(viewKey, { ...DEFAULT_DISPLAY, ...updated.display })
      toast.success('View updated')
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  const remove = async () => {
    try {
      await trackerApi.deleteView(view.id)
      t.removeView(view.id)
      setConfirmDelete(false)
      navigate({ name: 'views' })
    } catch (err) {
      toast.error(errorMessage(err))
    }
  }

  const team = view.team_id ? t.teamsById.get(view.team_id) : undefined

  return (
    <>
      <IssueCollection
        issues={issues}
        viewKey={viewKey}
        teamId={view.team_id}
        showTeamTabs={!view.team_id}
        defaultDisplay={view.display}
        toolbarExtra={
          dirty ? (
            <>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  t.setFiltersFor(viewKey, view.filters ?? {})
                  t.setDisplayFor(viewKey, { ...DEFAULT_DISPLAY, ...view.display })
                }}
              >
                Reset
              </Button>
              <Button size="sm" variant="primary" disabled={saving} onClick={() => void saveChanges(currentFilters, currentDisplay)}>
                Save changes
              </Button>
            </>
          ) : null
        }
        renderHeader={(toolbar) => (
          <PageHeader
            crumbs={[
              { label: 'Views', href: '/views', icon: <Layers size={14} /> },
              ...(team ? [{ label: team.name, muted: true }] : []),
            ]}
            title={view.name}
            icon={<Icon size={14} color={view.color ?? 'var(--text-3)'} />}
            actions={
              <>
                <IconButton
                  label={favorite ? 'Remove from favorites' : 'Add to favorites'}
                  size="sm"
                  active={favorite}
                  icon={<Star size={14} fill={favorite ? 'currentColor' : 'none'} />}
                  onClick={() => t.toggleFavorite('view', view.id).catch((err: unknown) => toast.error(errorMessage(err)))}
                />
                <IconButton label="Delete view" size="sm" icon={<Trash2 size={14} />} onClick={() => setConfirmDelete(true)} />
              </>
            }
            tabs={[{ id: 'issues', label: 'Issues', active: true }]}
            tabsRight={toolbar}
          />
        )}
      />
      <Dialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Delete view "${view.name}"?`}
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={() => void remove()}>
              Delete view
            </Button>
          </>
        }
      >
        <p className="t-2" style={{ margin: 0 }}>
          Issues are not affected. Anyone who favorited this view loses it.
        </p>
      </Dialog>
    </>
  )
}

export function ViewPage({ viewId }: { viewId: string }) {
  const t = useTracker()
  const view = t.views.find((v) => v.id === viewId)
  if (!view) {
    return (
      <>
        <PageHeader crumbs={[{ label: 'Views', href: '/views' }]} title="Not found" />
        <div className="lic-empty">
          <div className="lic-empty-inner">
            <div className="t-1 w-510">This view does not exist</div>
            <a className="lic-link" href="#/tracker/views">
              Back to views
            </a>
          </div>
        </div>
      </>
    )
  }
  return <ViewBody key={view.id} view={view} />
}

export function viewIssueCount(view: View, t: ReturnType<typeof useTracker>): number {
  const base = t.issues.filter((issue) => !issue.archived_at && (!view.team_id || issue.team_id === view.team_id))
  return applyFilters(base, view.filters ?? {}, t).length
}

export default ViewPage
