import { useMemo, useRef, useState } from 'react'
import type { MouseEvent, ReactNode } from 'react'
import {
  Archive,
  Bell,
  BellOff,
  ChevronDown,
  ChevronUp,
  Copy,
  GitBranch,
  Link2,
  Maximize2,
  MoreHorizontal,
  Star,
  Trash2,
  X,
} from 'lucide-react'
import { trackerApi } from '../api'
import { branchName } from '../format'
import { useHotkey } from '../hotkeys'
import { hrefFor, navigate } from '../router'
import { sortIssues, useTracker } from '../store'
import type { Issue, IssueDetail } from '../types'
import { Button, Dialog, IconButton, MenuList, Popover, PropertyChip, toast } from '../ui'
import type { MenuItem } from '../ui'
import { ActivityFeed } from './Activity'
import type { ComposerHandle } from './Activity'
import { AgentPanel } from './AgentPanel'
import { DescriptionEditor, TitleEditor } from './Editors'
import type { TitleHandle } from './Editors'
import { PropertiesSidebar, PropertyPickers, usePickerState, usePropertyAnchors } from './Properties'
import type { PropertyKind } from './Properties'
import { RelationsAndLinks, SubIssues } from './Relations'
import { copyText, menuId, notifyError } from './shared'
import { useIssueDetail } from './useIssueDetail'
import './detail.css'

function issueUrl(issue: Issue) {
  return `${window.location.origin}${window.location.pathname}${hrefFor({ name: 'issue', identifier: issue.identifier })}`
}

function useIssueActions(issue: Issue | null, onDeleted: () => void) {
  const t = useTracker()
  const [confirmDelete, setConfirmDelete] = useState(false)

  const copyId = () => issue && void copyText(issue.identifier, `Copied ${issue.identifier} to clipboard`)
  const copyBranch = () => issue && void copyText(branchName(issue, t.me), 'Copied git branch name to clipboard')
  const copyLink = () => issue && void copyText(issueUrl(issue), 'Copied issue URL to clipboard')
  const toggleArchive = () => {
    if (!issue) return
    const archived = !issue.archived_at
    t.updateIssues([issue.id], { archived })
      .then(() =>
        toast(archived ? `${issue.identifier} archived` : `${issue.identifier} restored`, {
          action: { label: 'Undo', onClick: () => void t.updateIssues([issue.id], { archived: !archived }).catch(notifyError) },
        }),
      )
      .catch(notifyError)
  }
  const doDelete = async () => {
    if (!issue) return
    try {
      await t.deleteIssue(issue.id)
      setConfirmDelete(false)
      toast(`${issue.identifier} deleted`)
      onDeleted()
    } catch (err) {
      notifyError(err)
    }
  }

  const deleteDialog = issue ? (
    <Dialog
      open={confirmDelete}
      onOpenChange={setConfirmDelete}
      title={`Delete ${issue.identifier}?`}
      width={420}
      footer={
        <>
          <Button variant="ghost" size="md" onClick={() => setConfirmDelete(false)}>
            Cancel
          </Button>
          <Button variant="danger" size="md" autoFocus onClick={() => void doDelete()}>
            Delete issue
          </Button>
        </>
      }
    >
      <div className="ld-dialog-body">
        {issue.lane_key
          ? 'This issue is linked to an agent lane and cannot be deleted. Close the lane or archive the issue instead.'
          : `"${issue.title}" and its comments will be permanently deleted. This cannot be undone.`}
      </div>
    </Dialog>
  ) : null

  const menuItems: MenuItem[] = issue
    ? [
        { id: 'copy-id', label: 'Copy ID', icon: <Copy size={14} />, kbd: '⌘.' },
        { id: 'copy-link', label: 'Copy issue URL', icon: <Link2 size={14} />, kbd: '⌘⇧C' },
        { id: 'copy-branch', label: 'Copy git branch name', icon: <GitBranch size={14} />, kbd: '⌘⇧,' },
        {
          id: 'favorite',
          label: t.isFavorite('issue', issue.id) ? 'Remove from favorites' : 'Add to favorites',
          icon: <Star size={14} />,
        },
        { id: 'archive', label: issue.archived_at ? 'Restore' : 'Archive', icon: <Archive size={14} />, kbd: '⌘⇧A' },
        { id: 'delete', label: 'Delete…', icon: <Trash2 size={14} />, kbd: '⌘⌫', color: 'var(--danger)' },
      ]
    : []

  const onMenu = (id: string) => {
    if (!issue) return
    switch (id) {
      case 'copy-id':
        return copyId()
      case 'copy-link':
        return copyLink()
      case 'copy-branch':
        return copyBranch()
      case 'favorite':
        return void t.toggleFavorite('issue', issue.id).catch(notifyError)
      case 'archive':
        return toggleArchive()
      case 'delete':
        return setConfirmDelete(true)
    }
  }

  return {
    copyId,
    copyBranch,
    copyLink,
    toggleArchive,
    askDelete: () => setConfirmDelete(true),
    deleteDialog,
    menuItems,
    onMenu,
  }
}

function MoreMenu({ items, onSelect }: { items: MenuItem[]; onSelect: (id: string) => void }) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)
  return (
    <>
      <IconButton
        label="More actions"
        icon={<MoreHorizontal size={16} />}
        onClick={(e: MouseEvent<HTMLElement>) => setAnchor(e.currentTarget)}
      />
      <Popover open={anchor !== null} onOpenChange={(o) => !o && setAnchor(null)} anchor={anchor} align="end" width={240}>
        <MenuList
          items={items}
          onSelect={(item: MenuItem | string) => {
            setAnchor(null)
            onSelect(menuId(item))
          }}
        />
      </Popover>
    </>
  )
}

interface ViewProps {
  idOrIdentifier: string
  mode: 'page' | 'peek'
  onClose?: () => void
}

function IssueView({ idOrIdentifier, mode, onClose }: ViewProps) {
  const t = useTracker()
  const handle = useIssueDetail(idOrIdentifier)
  const { detail, issue, notFound, error, reload, setDetail } = handle
  const title = useRef<TitleHandle>(null)
  const composer = useRef<ComposerHandle>(null)
  const { picker, setPicker, close: closePicker } = usePickerState()
  const { anchors, register } = usePropertyAnchors()
  const chipRow = useRef<HTMLDivElement>(null)

  const leave = () => {
    if (mode === 'peek') onClose?.()
    else if (window.history.length > 1) window.history.back()
    else navigate(issue ? `/team/${t.teamsById.get(issue.team_id)?.key ?? ''}/all` : '/')
  }

  const actions = useIssueActions(issue, () => {
    if (mode === 'peek') onClose?.()
    else navigate(issue ? `/team/${t.teamsById.get(issue.team_id)?.key ?? ''}/all` : '/', { replace: true })
  })

  const siblings = useMemo(() => {
    if (!issue) return []
    const pool = t.issues.filter((i) => i.team_id === issue.team_id && !i.archived_at)
    return sortIssues(pool, 'priority', t.statesById)
  }, [t.issues, t.statesById, issue?.team_id])
  const index = issue ? siblings.findIndex((i) => i.id === issue.id) : -1

  const goTo = (offset: number) => {
    if (index < 0) return
    const target = siblings[index + offset]
    if (!target) return
    if (mode === 'peek') t.setPeekIssueId(target.id)
    else navigate({ name: 'issue', identifier: target.identifier })
  }

  const openPicker = (kind: PropertyKind) => {
    const anchor =
      anchors.current.get(kind) ??
      (chipRow.current?.querySelector<HTMLElement>(`[data-kind="${kind}"]`) ?? chipRow.current ?? null)
    setPicker({ kind, anchor })
  }

  const ready = issue !== null
  const hotkeyOpts = { enabled: ready && picker === null }
  useHotkey('s', () => openPicker('status'), hotkeyOpts)
  useHotkey('p', () => openPicker('priority'), hotkeyOpts)
  useHotkey('a', () => openPicker('assignee'), hotkeyOpts)
  useHotkey('l', () => openPicker('labels'), hotkeyOpts)
  useHotkey('shift+p', () => openPicker('project'), hotkeyOpts)
  useHotkey('shift+c', () => openPicker('cycle'), hotkeyOpts)
  useHotkey('shift+e', () => openPicker('estimate'), hotkeyOpts)
  useHotkey(['d', 'shift+d'], () => openPicker('due'), hotkeyOpts)
  useHotkey(
    'i',
    () => {
      if (issue && t.me) t.updateIssues([issue.id], { assignee_id: t.me.id }).catch(notifyError)
    },
    hotkeyOpts,
  )
  useHotkey('e', () => title.current?.focus(), hotkeyOpts)
  useHotkey('m', () => composer.current?.focus(), hotkeyOpts)
  useHotkey('mod+.', actions.copyId, { enabled: ready })
  useHotkey(['mod+shift+,', 'mod+shift+<'], actions.copyBranch, { enabled: ready })
  useHotkey('mod+shift+c', actions.copyLink, { enabled: ready })
  useHotkey('mod+shift+a', actions.toggleArchive, { enabled: ready })
  useHotkey('mod+backspace', actions.askDelete, { enabled: ready })
  useHotkey(['j', 'arrowdown'], () => goTo(1), { enabled: ready && mode === 'page' && picker === null })
  useHotkey(['k', 'arrowup'], () => goTo(-1), { enabled: ready && mode === 'page' && picker === null })
  useHotkey('escape', leave, {
    enabled: picker === null && t.createDraft === null && (mode === 'peek' || t.peekIssueId === null),
  })

  const subscribed = detail && t.me ? detail.subscriber_ids.includes(t.me.id) : false
  const toggleSubscribe = async () => {
    if (!detail) return
    try {
      const result = await trackerApi.subscribe(detail.issue.id, !subscribed)
      setDetail({
        ...detail,
        subscriber_ids: result.subscribed
          ? Array.from(new Set([...detail.subscriber_ids, t.me?.id ?? '']))
          : detail.subscriber_ids.filter((id) => id !== t.me?.id),
      })
      toast(result.subscribed ? 'Subscribed to issue' : 'Unsubscribed from issue')
    } catch (err) {
      notifyError(err)
    }
  }

  const team = issue ? t.teamsById.get(issue.team_id) : undefined
  const parent = issue?.parent_id ? t.issuesById.get(issue.parent_id) : undefined
  const changed = () => void reload()

  const header = (
    <header className="ld-header">
      <nav className="ld-crumbs" aria-label="Breadcrumb">
        {team && (
          <a className="ld-crumb" href={hrefFor({ name: 'team-issues', teamKey: team.key, tab: 'all' })}>
            <span className="ld-team-badge" style={{ background: team.color }}>
              {team.key.slice(0, 1)}
            </span>
            {team.name}
          </a>
        )}
        {parent && (
          <>
            <span aria-hidden>›</span>
            <a className="ld-crumb" href={hrefFor({ name: 'issue', identifier: parent.identifier })}>
              {parent.identifier}
            </a>
          </>
        )}
        {issue && (
          <>
            <span aria-hidden>›</span>
            <span className="ld-crumb ld-crumb-current">{issue.identifier}</span>
            <span className="ld-crumb-title">{issue.title}</span>
          </>
        )}
      </nav>
      <div className="ld-header-actions">
        {issue && (
          <>
            {index >= 0 && siblings.length > 1 && (
              <>
                <span className="ld-nav-count">
                  {index + 1} / {siblings.length}
                </span>
                <IconButton label="Previous issue" icon={<ChevronUp size={16} />} disabled={index <= 0} onClick={() => goTo(-1)} />
                <IconButton
                  label="Next issue"
                  icon={<ChevronDown size={16} />}
                  disabled={index >= siblings.length - 1}
                  onClick={() => goTo(1)}
                />
              </>
            )}
            <IconButton
              label={subscribed ? 'Unsubscribe' : 'Subscribe'}
              icon={subscribed ? <BellOff size={16} /> : <Bell size={16} />}
              active={subscribed}
              onClick={() => void toggleSubscribe()}
            />
            <IconButton label="Copy issue URL" icon={<Link2 size={16} />} onClick={actions.copyLink} />
            <MoreMenu items={actions.menuItems} onSelect={actions.onMenu} />
            {mode === 'peek' && (
              <IconButton
                label="Open issue"
                icon={<Maximize2 size={15} />}
                onClick={() => {
                  onClose?.()
                  navigate({ name: 'issue', identifier: issue.identifier })
                }}
              />
            )}
          </>
        )}
        {mode === 'peek' && <IconButton label="Close" icon={<X size={16} />} onClick={onClose} />}
      </div>
    </header>
  )

  let body: ReactNode
  if (!issue) {
    body = notFound ? (
      <div className="ld-empty">
        <div>
          <div className="t-1 w-510" style={{ fontSize: 15 }}>
            Issue not found
          </div>
          <div style={{ marginTop: 4 }}>It may have been deleted or you followed a stale link.</div>
        </div>
      </div>
    ) : error ? (
      <div className="ld-empty">
        <div>
          <div className="ld-error">{error}</div>
          <Button variant="secondary" size="sm" onClick={() => void reload()} style={{ marginTop: 10 }}>
            Retry
          </Button>
        </div>
      </div>
    ) : (
      <div className="ld-main">
        <div className="ld-main-inner" aria-busy>
          <div className="ld-skeleton" style={{ width: '60%', height: 24 }} />
          <div className="ld-skeleton" style={{ width: '90%' }} />
          <div className="ld-skeleton" style={{ width: '75%' }} />
        </div>
      </div>
    )
  } else {
    const compact = mode === 'peek'
    const showSidebar = mode === 'page'
    const viewDetail: IssueDetail | null = detail ? { ...detail, issue } : null
    body = (
      <div className="ld-body">
        <div className="ld-main">
          <div className="ld-main-inner">
            <TitleEditor key={issue.id} ref={title} issue={issue} />
            {compact && (
              <div className="ld-peek-props" ref={chipRow}>
                {(['status', 'priority', 'assignee', 'labels', 'project', 'cycle', 'estimate', 'due'] as const).map((kind) => (
                  <span key={kind} data-kind={kind}>
                    <PropertyChip kind={kind} issue={issue} />
                  </span>
                ))}
              </div>
            )}
            <DescriptionEditor key={`d${issue.id}`} issue={issue} compact={compact} />
            {viewDetail ? (
              <>
                <SubIssues detail={viewDetail} onChanged={changed} />
                <RelationsAndLinks detail={viewDetail} onChanged={changed} />
                <AgentPanel detail={viewDetail} onDetail={setDetail} compact={compact} />
                <ActivityFeed
                  ref={composer}
                  detail={viewDetail}
                  subscribed={subscribed}
                  onToggleSubscribe={() => void toggleSubscribe()}
                  onChanged={changed}
                  compact={compact}
                />
              </>
            ) : (
              <div className="ld-skeleton" style={{ width: '40%', marginTop: 28 }} />
            )}
          </div>
        </div>
        {showSidebar && (
          <aside className="ld-side" aria-label="Issue properties">
            <PropertiesSidebar issue={issue} openPicker={(kind, anchor) => setPicker({ kind, anchor })} register={register} />
          </aside>
        )}
      </div>
    )
  }

  return (
    <>
      {header}
      {body}
      {issue && <PropertyPickers issue={issue} picker={picker} onClose={closePicker} />}
      {actions.deleteDialog}
    </>
  )
}

export function IssuePage({ idOrIdentifier }: { idOrIdentifier: string }) {
  return (
    <div className="ld-page">
      <IssueView key={idOrIdentifier} idOrIdentifier={idOrIdentifier} mode="page" />
    </div>
  )
}

export function IssuePeek() {
  const t = useTracker()
  const id = t.peekIssueId
  const close = () => t.setPeekIssueId(null)
  useHotkey('space', close, { enabled: id !== null })
  if (!id) return null
  return (
    <>
      <div className="ld-peek-overlay" onMouseDown={close} aria-hidden />
      <div className="ld-peek anim-slide-right" role="dialog" aria-modal="false" aria-label="Issue preview">
        <IssueView key={id} idOrIdentifier={id} mode="peek" onClose={close} />
      </div>
    </>
  )
}
