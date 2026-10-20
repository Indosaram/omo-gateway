import { ArrowLeft, BellOff, Check, CheckCheck, Clock, Inbox, Mail, MailOpen, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useState, type MouseEvent } from 'react'
import { trackerApi } from '../api'
import { IssuePage } from '../detail'
import { compactRelative, relativeTime } from '../format'
import { useHotkey } from '../hotkeys'
import {
  INBOX_FILTERS,
  describe,
  filterCount,
  filterNotifications,
  kindIcon,
  notificationTitle,
  snoozeOptions,
  type InboxFilter,
} from '../inbox/model'
import { hrefFor, navigate } from '../router'
import { PageHeader } from '../shell/Header'
import { useTracker } from '../store'
import type { Notification } from '../types'
import { Avatar, Button, ContextMenu, IconButton, MenuList, Popover, ProjectIcon, StatusIcon, toast } from '../ui'
import type { MenuItem } from '../ui'

function useIsMobile(): boolean {
  const query = '(max-width: 767px)'
  const [mobile, setMobile] = useState(() => typeof window !== 'undefined' && window.matchMedia(query).matches)
  useEffect(() => {
    const mql = window.matchMedia(query)
    const onChange = () => setMobile(mql.matches)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [])
  return mobile
}

export function InboxPage({ notificationId }: { notificationId?: string | null }) {
  const t = useTracker()
  const mobile = useIsMobile()
  const [filter, setFilter] = useState<InboxFilter>('all')
  const [snooze, setSnooze] = useState<{ anchor: HTMLElement; id: string } | null>(null)

  const list = useMemo(() => filterNotifications(t.notifications, filter), [t.notifications, filter])
  const selected = notificationId ? t.notifications.find((n) => n.id === notificationId) ?? null : null
  const selectedIndex = selected ? list.findIndex((n) => n.id === selected.id) : -1

  const patch = async (n: Notification, input: { read?: boolean; snoozed_until?: string | null; archived?: boolean }) => {
    const optimistic: Notification = {
      ...n,
      read_at: input.read === undefined ? n.read_at : input.read ? new Date().toISOString() : null,
      snoozed_until: input.snoozed_until === undefined ? n.snoozed_until : input.snoozed_until,
      archived_at: input.archived === undefined ? n.archived_at : input.archived ? new Date().toISOString() : null,
    }
    t.upsertNotification(optimistic)
    try {
      t.upsertNotification(await trackerApi.updateNotification(n.id, input))
    } catch (err) {
      t.upsertNotification(n)
      t.reportError(err)
    }
  }

  const open = (n: Notification) => {
    navigate(`/inbox/${n.id}`, { replace: Boolean(notificationId) })
    if (!n.read_at) void patch(n, { read: true })
  }

  const neighbourAfterRemoval = (n: Notification): Notification | null => {
    const index = list.findIndex((x) => x.id === n.id)
    return list[index + 1] ?? list[index - 1] ?? null
  }

  const removeFromList = (n: Notification, input: { archived?: boolean; snoozed_until?: string }, message: string) => {
    const next = selected?.id === n.id ? neighbourAfterRemoval(n) : null
    void patch(n, input)
    if (selected?.id === n.id) navigate(next ? `/inbox/${next.id}` : '/inbox', { replace: true })
    toast(message, {
      action: {
        label: 'Undo',
        onClick: () => void patch({ ...n }, input.archived ? { archived: false } : { snoozed_until: null }),
      },
    })
  }

  const remove = (n: Notification) => removeFromList(n, { archived: true }, 'Notification deleted')
  const snoozeUntil = (n: Notification, until: string) => removeFromList(n, { snoozed_until: until }, 'Notification snoozed')

  const markAllRead = async () => {
    try {
      const result = await trackerApi.readAllNotifications()
      await t.refreshNotifications()
      toast(result.updated === 1 ? '1 notification marked as read' : `${result.updated} notifications marked as read`)
    } catch (err) {
      t.reportError(err)
    }
  }

  const step = (delta: number) => {
    if (list.length === 0) return
    const index = selectedIndex < 0 ? (delta > 0 ? 0 : list.length - 1) : Math.min(list.length - 1, Math.max(0, selectedIndex + delta))
    open(list[index])
  }

  useHotkey('e', () => selected && void patch(selected, { read: true }), { enabled: Boolean(selected) })
  useHotkey('u', () => selected && void patch(selected, { read: !selected.read_at }), { enabled: Boolean(selected) })
  useHotkey(['backspace', 'delete'], () => selected && remove(selected), { enabled: Boolean(selected) })
  useHotkey(['j', 'arrowdown'], () => step(1))
  useHotkey(['k', 'arrowup'], () => step(-1))
  useHotkey('h', () => {
    if (!selected) return
    const anchor = document.querySelector<HTMLElement>(`[data-notification="${selected.id}"]`)
    if (anchor) setSnooze({ anchor, id: selected.id })
  }, { enabled: Boolean(selected) })
  useHotkey('mod+shift+u', () => void markAllRead())
  useHotkey('escape', () => selected && navigate('/inbox', { replace: true }), { enabled: Boolean(selected), preventDefault: false })

  const unread = t.notifications.filter((n) => !n.read_at && !n.archived_at).length
  const showList = !mobile || !selected
  const showDetail = !mobile || Boolean(selected)
  const snoozeTarget = snooze ? t.notifications.find((n) => n.id === snooze.id) ?? null : null

  return (
    <div className="flex h-full min-h-0">
      {showList && (
        <div className="flex min-h-0 w-[380px] shrink-0 flex-col border-r max-md:w-full max-md:border-r-0">
          <PageHeader
            title="Inbox"
            actions={
              <IconButton
                label="Mark all as read"
                icon={<CheckCheck size={14} />}
                onClick={() => void markAllRead()}
                disabled={unread === 0}
              />
            }
          />
          <div className="flex shrink-0 items-center gap-1 overflow-x-auto px-3 py-2">
            {INBOX_FILTERS.map((f) => {
              const count = filterCount(t.notifications, f.id)
              return (
                <button
                  key={f.id}
                  type="button"
                  onClick={() => setFilter(f.id)}
                  className="inline-flex h-[26px] shrink-0 items-center gap-1.5 rounded-full border px-2.5 text-[12px] w-510"
                  style={{
                    background: filter === f.id ? 'var(--bg-active)' : 'transparent',
                    borderColor: filter === f.id ? 'var(--border-strong)' : 'var(--border)',
                    color: filter === f.id ? 'var(--text-1)' : 'var(--text-3)',
                  }}
                >
                  {f.label}
                  {count > 0 && <span className="t-4 tabular">{count}</span>}
                </button>
              )
            })}
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-1.5 pb-2" role="listbox" aria-label="Notifications">
            {list.length === 0 ? (
              <div className="flex flex-col items-center gap-2 px-6 py-16 text-center">
                <Inbox size={28} className="t-4" />
                <p className="text-[13px] t-3">{filter === 'snoozed' ? 'No snoozed notifications' : 'You’re all caught up'}</p>
              </div>
            ) : (
              list.map((n) => (
                <NotificationRow
                  key={n.id}
                  notification={n}
                  selected={n.id === selected?.id}
                  onOpen={() => open(n)}
                  onToggleRead={() => void patch(n, { read: !n.read_at })}
                  onDelete={() => remove(n)}
                  onSnooze={(anchor) => setSnooze({ anchor, id: n.id })}
                />
              ))
            )}
          </div>
        </div>
      )}
      {showDetail && (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          {selected ? (
            <>
              <div className="flex h-11 shrink-0 items-center gap-1 border-b px-3">
                {mobile && (
                  <IconButton label="Back to inbox" icon={<ArrowLeft size={15} />} onClick={() => navigate('/inbox', { replace: true })} />
                )}
                <span className="min-w-0 flex-1 truncate text-[12px] t-3">{relativeTime(selected.created_at)}</span>
                <IconButton
                  label={selected.read_at ? 'Mark as unread' : 'Mark as read'}
                  icon={selected.read_at ? <Mail size={14} /> : <MailOpen size={14} />}
                  onClick={() => void patch(selected, { read: !selected.read_at })}
                />
                <IconButton
                  label="Snooze"
                  icon={<Clock size={14} />}
                  onClick={(e: MouseEvent<HTMLButtonElement>) => setSnooze({ anchor: e.currentTarget, id: selected.id })}
                />
                <IconButton label="Delete notification" icon={<Trash2 size={14} />} onClick={() => remove(selected)} />
              </div>
              <div className="min-h-0 flex-1 overflow-auto">
                <NotificationTarget notification={selected} />
              </div>
            </>
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
              <Inbox size={36} className="t-4" />
              <p className="text-[13px] t-3">{unread > 0 ? `${unread} unread ${unread === 1 ? 'notification' : 'notifications'}` : 'No notification selected'}</p>
            </div>
          )}
        </div>
      )}
      <Popover open={snoozeTarget !== null} onOpenChange={(v) => !v && setSnooze(null)} anchor={snooze?.anchor ?? null} width={220}>
        {snoozeTarget && (
          <MenuList
            items={[
              ...snoozeOptions().map((o) => ({ id: o.id, label: o.label, icon: <Clock size={14} /> })),
              ...(snoozeTarget.snoozed_until ? [{ id: 'unsnooze', label: 'Unsnooze', icon: <BellOff size={14} /> }] : []),
            ]}
            onSelect={(item: MenuItem) => {
              setSnooze(null)
              if (item.id === 'unsnooze') {
                void patch(snoozeTarget, { snoozed_until: null })
                return
              }
              const option = snoozeOptions().find((o) => o.id === item.id)
              if (option) snoozeUntil(snoozeTarget, option.until)
            }}
          />
        )}
      </Popover>
    </div>
  )
}

function NotificationRow({
  notification: n,
  selected,
  onOpen,
  onToggleRead,
  onDelete,
  onSnooze,
}: {
  notification: Notification
  selected: boolean
  onOpen: () => void
  onToggleRead: () => void
  onDelete: () => void
  onSnooze: (anchor: HTMLElement) => void
}) {
  const t = useTracker()
  const actor = n.actor_id ? t.usersById.get(n.actor_id) ?? null : null
  const issue = n.issue_id ? t.issuesById.get(n.issue_id) : undefined
  const project = n.project_id ? t.projectsById.get(n.project_id) : undefined
  const state = issue ? t.statesById.get(issue.state_id) : undefined
  const { id, title } = notificationTitle(n, issue, project)
  const unread = !n.read_at

  return (
    <ContextMenu
      items={[
        { id: 'read', label: unread ? 'Mark as read' : 'Mark as unread', kbd: 'U' },
        { id: 'snooze', label: 'Snooze', kbd: 'H' },
        { id: 'delete', label: 'Delete notification', kbd: '⌫' },
      ]}
      onSelect={(item) => {
        if (item.id === 'read') onToggleRead()
        if (item.id === 'delete') onDelete()
        if (item.id === 'snooze') {
          const el = document.querySelector<HTMLElement>(`[data-notification="${n.id}"]`)
          if (el) onSnooze(el)
        }
      }}
    >
      <a
        href={hrefFor(`/inbox/${n.id}`)}
        data-notification={n.id}
        role="option"
        aria-selected={selected}
        onClick={(e) => {
          e.preventDefault()
          onOpen()
        }}
        className="group relative flex items-start gap-3 rounded-[6px] px-2.5 py-2"
        style={{ background: selected ? 'var(--bg-selected)' : undefined }}
        onMouseEnter={(e) => {
          if (!selected) e.currentTarget.style.background = 'var(--bg-hover)'
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = selected ? 'var(--bg-selected)' : ''
        }}
      >
        <span className="relative mt-0.5 shrink-0">
          <Avatar user={actor} size={32} />
          <span
            className="absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full t-2"
            style={{ background: 'var(--popover)', boxShadow: '0 0 0 2px var(--panel)' }}
          >
            {kindIcon(n.kind)}
          </span>
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-center gap-1.5 text-[13px]">
            {unread && <span className="h-2 w-2 shrink-0 rounded-full" style={{ background: 'var(--accent)' }} aria-label="Unread" />}
            {id && <span className={`shrink-0 tabular ${unread ? 't-2' : 't-3'}`}>{id}</span>}
            <span className={`truncate ${unread ? 'w-510 t-1' : 't-3'}`}>{title}</span>
          </span>
          <span className={`truncate text-[12px] ${unread ? 't-3' : 't-4'}`}>{describe(n, actor)}</span>
        </span>
        <span className="flex shrink-0 flex-col items-end gap-1">
          {state ? (
            <StatusIcon type={state.type} color={state.color} size={14} />
          ) : project ? (
            <ProjectIcon icon={project.icon} color={project.color} />
          ) : (
            <span className="h-3.5" />
          )}
          <span className="text-[12px] t-4 tabular group-hover:hidden">{compactRelative(n.created_at)}</span>
          <span className="hidden items-center gap-0.5 group-hover:flex">
            <button
              type="button"
              aria-label={unread ? 'Mark as read' : 'Mark as unread'}
              title={unread ? 'Mark as read' : 'Mark as unread'}
              className="flex h-5 w-5 items-center justify-center rounded-[4px] t-3 hover:text-[var(--text-1)]"
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onToggleRead()
              }}
            >
              <Check size={12} />
            </button>
            <button
              type="button"
              aria-label="Delete notification"
              title="Delete notification"
              className="flex h-5 w-5 items-center justify-center rounded-[4px] t-3 hover:text-[var(--text-1)]"
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onDelete()
              }}
            >
              <Trash2 size={12} />
            </button>
          </span>
        </span>
      </a>
    </ContextMenu>
  )
}

function NotificationTarget({ notification: n }: { notification: Notification }) {
  const t = useTracker()
  if (n.issue_id) {
    const issue = t.issuesById.get(n.issue_id)
    return <IssuePage key={n.issue_id} idOrIdentifier={issue?.identifier ?? n.issue_id} embedded />
  }
  const project = n.project_id ? t.projectsById.get(n.project_id) : undefined
  return (
    <div className="mx-auto flex max-w-[640px] flex-col gap-4 px-6 py-10">
      {project && (
        <div className="flex items-center gap-2 text-[15px] w-510 t-1">
          <ProjectIcon icon={project.icon} color={project.color} />
          {project.name}
        </div>
      )}
      <p className="whitespace-pre-wrap text-[14px] t-2">{n.body || 'No details for this notification.'}</p>
      {project && (
        <div>
          <Button variant="secondary" size="sm" onClick={() => navigate(`/project/${project.id}/updates`)}>
            Open project
          </Button>
        </div>
      )}
    </div>
  )
}

export default InboxPage
