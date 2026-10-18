import { useMemo, useRef, useState } from 'react'
import type { MouseEvent, MutableRefObject } from 'react'
import { ArrowUpRight, Link2, Plus, X } from 'lucide-react'
import { trackerApi } from '../api'
import { hrefFor } from '../router'
import { stateProgress } from '../format'
import { teamDefaultState, useTracker } from '../store'
import type { Issue, IssueDetail, Relation, RelationType } from '../types'
import { Avatar, Button, IconButton, MenuList, Popover, PriorityIcon, StatusIcon } from '../ui'
import type { MenuItem } from '../ui'
import { hostOf, menuId, notifyError } from './shared'

function IssueRow({ issue, onRemove, removeLabel }: { issue: Issue; onRemove?: () => void; removeLabel?: string }) {
  const t = useTracker()
  const state = t.statesById.get(issue.state_id)
  const assignee = issue.assignee_id ? t.usersById.get(issue.assignee_id) : undefined
  return (
    <a className="ld-row" href={hrefFor({ name: 'issue', identifier: issue.identifier })}>
      <PriorityIcon priority={issue.priority} size={14} />
      <span className="ld-row-id">{issue.identifier}</span>
      {state && (
        <StatusIcon
          type={state.type}
          color={state.color}
          progress={stateProgress(state, t.teamStates(issue.team_id))}
          size={14}
        />
      )}
      <span className="ld-row-title">{issue.title}</span>
      {onRemove && (
        <span className="ld-row-actions">
          <IconButton
            label={removeLabel ?? 'Remove'}
            icon={<X size={14} />}
            size="sm"
            onClick={(e: MouseEvent) => {
              e.preventDefault()
              e.stopPropagation()
              onRemove()
            }}
          />
        </span>
      )}
      <Avatar user={assignee ?? null} size={20} />
    </a>
  )
}

function ProgressRing({ done, total }: { done: number; total: number }) {
  const r = 6
  const c = 2 * Math.PI * r
  const ratio = total === 0 ? 0 : done / total
  return (
    <span className="ld-progress">
      <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden>
        <circle cx="8" cy="8" r={r} fill="none" stroke="var(--border-strong)" strokeWidth="2" />
        <circle
          cx="8"
          cy="8"
          r={r}
          fill="none"
          stroke="var(--accent)"
          strokeWidth="2"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - ratio)}
          transform="rotate(-90 8 8)"
          strokeLinecap="round"
        />
      </svg>
      {done}/{total}
    </span>
  )
}

export function SubIssues({ detail, onChanged }: { detail: IssueDetail; onChanged: () => void }) {
  const t = useTracker()
  const issue = detail.issue
  const [adding, setAdding] = useState(false)
  const [title, setTitle] = useState('')
  const children = detail.children
    .map((c) => t.issuesById.get(c.id) ?? c)
    .filter((c) => !c.archived_at && c.parent_id === issue.id)
  const done = children.filter((c) => t.statesById.get(c.state_id)?.type === 'completed').length

  const submit = async () => {
    const value = title.trim()
    if (!value) {
      setAdding(false)
      return
    }
    try {
      const state = teamDefaultState(issue.team_id, t.teamStates)
      await t.createIssue({
        team_id: issue.team_id,
        title: value,
        parent_id: issue.id,
        state_id: state?.id,
        project_id: issue.project_id,
        cycle_id: issue.cycle_id,
      })
      setTitle('')
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  return (
    <section className="ld-section" aria-label="Sub-issues">
      <div className="ld-section-head">
        <span>Sub-issues</span>
        {children.length > 0 && <ProgressRing done={done} total={children.length} />}
        <span className="ld-spacer" />
        <IconButton label="Add sub-issue" icon={<Plus size={14} />} size="sm" onClick={() => setAdding(true)} />
      </div>
      {(children.length > 0 || adding) && (
        <div className="ld-rows">
          {children.map((c) => (
            <IssueRow
              key={c.id}
              issue={c}
              removeLabel="Remove from parent"
              onRemove={() => t.updateIssues([c.id], { parent_id: null }).then(onChanged).catch(notifyError)}
            />
          ))}
          {adding && (
            <div className="ld-row">
              <Plus size={14} color="var(--text-4)" />
              <input
                autoFocus
                className="ld-inline-input"
                placeholder="Sub-issue title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={() => {
                  if (!title.trim()) setAdding(false)
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    void submit()
                  } else if (e.key === 'Escape') {
                    e.preventDefault()
                    e.stopPropagation()
                    setTitle('')
                    setAdding(false)
                  }
                }}
              />
            </div>
          )}
        </div>
      )}
    </section>
  )
}

const RELATION_GROUPS: { direction: Relation['direction']; label: string }[] = [
  { direction: 'blocked_by', label: 'Blocked by' },
  { direction: 'blocks', label: 'Blocks' },
  { direction: 'related', label: 'Related' },
  { direction: 'duplicate', label: 'Duplicate of' },
  { direction: 'duplicated_by', label: 'Duplicated by' },
]

type RelationChoice = 'blocks' | 'blocked_by' | 'related' | 'duplicate'

const RELATION_CHOICES: { id: RelationChoice; label: string }[] = [
  { id: 'blocked_by', label: 'Blocked by…' },
  { id: 'blocks', label: 'Blocking…' },
  { id: 'related', label: 'Related to…' },
  { id: 'duplicate', label: 'Duplicate of…' },
]

export function RelationsAndLinks({
  detail,
  onChanged,
  relationAnchorRef,
}: {
  detail: IssueDetail
  onChanged: () => void
  relationAnchorRef?: MutableRefObject<(() => void) | null>
}) {
  const t = useTracker()
  const issue = detail.issue
  const addButton = useRef<HTMLButtonElement | null>(null)
  const [menuOpen, setMenuOpen] = useState(false)
  const [choice, setChoice] = useState<RelationChoice | null>(null)
  const [linkForm, setLinkForm] = useState(false)
  const [url, setUrl] = useState('')
  const [linkTitle, setLinkTitle] = useState('')

  if (relationAnchorRef) relationAnchorRef.current = () => setMenuOpen(true)

  const candidates: MenuItem[] = useMemo(() => {
    const linked = new Set(detail.relations.map((r) => r.issue.id))
    return t.issues
      .filter((i) => i.id !== issue.id && !i.archived_at && !linked.has(i.id))
      .slice(0, 500)
      .map((i) => {
        const s = t.statesById.get(i.state_id)
        return {
          id: i.id,
          label: `${i.identifier} ${i.title}`,
          icon: s ? <StatusIcon type={s.type} color={s.color} size={14} /> : undefined,
        }
      })
  }, [t.issues, t.statesById, detail.relations, issue.id])

  const addRelation = async (relatedId: string, kind: RelationChoice) => {
    try {
      if (kind === 'blocked_by') await trackerApi.addRelation(relatedId, 'blocks', issue.id)
      else await trackerApi.addRelation(issue.id, kind as RelationType, relatedId)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  const removeRelation = async (id: string) => {
    try {
      await trackerApi.deleteRelation(id)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  const submitLink = async () => {
    const value = url.trim()
    if (!value) return
    const normalized = /^[a-z]+:\/\//i.test(value) ? value : `https://${value}`
    try {
      await trackerApi.addLink(issue.id, normalized, linkTitle.trim() || undefined)
      setUrl('')
      setLinkTitle('')
      setLinkForm(false)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  const removeLink = async (id: string) => {
    try {
      await trackerApi.deleteLink(id)
      onChanged()
    } catch (err) {
      notifyError(err)
    }
  }

  const hasContent = detail.relations.length > 0 || detail.links.length > 0

  return (
    <section className="ld-section" aria-label="Relations and links">
      <div className="ld-section-head">
        <span>Relations</span>
        <span className="ld-spacer" />
        <button
          type="button"
          ref={addButton}
          className="ld-tab"
          onClick={() => {
            setChoice(null)
            setMenuOpen(true)
          }}
        >
          <Plus size={12} style={{ verticalAlign: '-2px', marginRight: 4 }} />
          Add relation
        </button>
        <button type="button" className="ld-tab" onClick={() => setLinkForm(true)}>
          <Link2 size={12} style={{ verticalAlign: '-2px', marginRight: 4 }} />
          Add link
        </button>
      </div>

      <Popover
        open={menuOpen}
        onOpenChange={(open) => {
          setMenuOpen(open)
          if (!open) setChoice(null)
        }}
        anchor={addButton.current}
        align="end"
        width={choice ? 360 : 220}
      >
        {choice === null ? (
          <MenuList
            items={RELATION_CHOICES.map((c) => ({ id: c.id, label: c.label }))}
            onSelect={(item: MenuItem | string) => setChoice(menuId(item) as RelationChoice)}
          />
        ) : (
          <MenuList
            items={candidates}
            searchable
            placeholder="Search issues…"
            emptyText="No issues found"
            onSelect={(item: MenuItem | string) => {
              const kind = choice
              setMenuOpen(false)
              setChoice(null)
              void addRelation(menuId(item), kind)
            }}
          />
        )}
      </Popover>

      {hasContent && (
        <div>
          {RELATION_GROUPS.map((group) => {
            const items = detail.relations.filter((r) => r.direction === group.direction)
            if (items.length === 0) return null
            return (
              <div key={group.direction} className="ld-rel-group">
                <div className="ld-rel-label">{group.label}</div>
                <div className="ld-rows">
                  {items.map((r) => (
                    <IssueRow
                      key={r.id}
                      issue={t.issuesById.get(r.issue.id) ?? r.issue}
                      removeLabel="Remove relation"
                      onRemove={() => void removeRelation(r.id)}
                    />
                  ))}
                </div>
              </div>
            )
          })}
          {detail.links.length > 0 && (
            <div className="ld-rel-group">
              <div className="ld-rel-label">Links</div>
              <div className="ld-rows">
                {detail.links.map((l) => (
                  <a key={l.id} className="ld-row" href={l.url} target="_blank" rel="noreferrer">
                    <Link2 size={14} color="var(--text-3)" />
                    <span className="ld-row-title">{l.title || l.url}</span>
                    <span className="ld-row-meta">{hostOf(l.url)}</span>
                    <span className="ld-row-actions">
                      <IconButton
                        label="Remove link"
                        icon={<X size={14} />}
                        size="sm"
                        onClick={(e: MouseEvent) => {
                          e.preventDefault()
                          e.stopPropagation()
                          void removeLink(l.id)
                        }}
                      />
                    </span>
                    <ArrowUpRight size={14} color="var(--text-4)" />
                  </a>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {linkForm && (
        <form
          className="ld-link-form"
          onSubmit={(e) => {
            e.preventDefault()
            void submitLink()
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.stopPropagation()
              setLinkForm(false)
            }
          }}
        >
          <input
            autoFocus
            className="ld-field"
            style={{ flex: '2 1 220px' }}
            placeholder="https://"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <input
            className="ld-field"
            style={{ flex: '1 1 140px' }}
            placeholder="Title (optional)"
            value={linkTitle}
            onChange={(e) => setLinkTitle(e.target.value)}
          />
          <Button variant="ghost" size="sm" type="button" onClick={() => setLinkForm(false)}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" type="submit" disabled={!url.trim()}>
            Add link
          </Button>
        </form>
      )}
    </section>
  )
}
