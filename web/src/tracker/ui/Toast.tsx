import { useEffect, useState, useSyncExternalStore } from 'react'
import { CircleAlert, CircleCheck, X } from 'lucide-react'
import { Button, IconButton } from './Button'
import { Portal } from './layer'

export interface ToastOptions {
  action?: { label: string; onClick: () => void }
  kind?: 'info' | 'success' | 'error'
  duration?: number
}

interface ToastEntry extends ToastOptions {
  id: number
  message: string
}

let toasts: ToastEntry[] = []
let nextId = 0
const listeners = new Set<() => void>()

function emit() {
  for (const listener of listeners) listener()
}

export function dismissToast(id: number) {
  toasts = toasts.filter((t) => t.id !== id)
  emit()
}

export function toast(message: string, opts: ToastOptions = {}): number {
  const id = ++nextId
  toasts = [...toasts.slice(-4), { id, message, ...opts }]
  emit()
  return id
}

toast.error = (message: string, opts: Omit<ToastOptions, 'kind'> = {}) => toast(message, { ...opts, kind: 'error' })
toast.success = (message: string, opts: Omit<ToastOptions, 'kind'> = {}) => toast(message, { ...opts, kind: 'success' })

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function snapshot() {
  return toasts
}

function ToastItem({ entry }: { entry: ToastEntry }) {
  const [paused, setPaused] = useState(false)
  const duration = entry.duration ?? (entry.action ? 6000 : 4000)
  useEffect(() => {
    if (paused || duration <= 0) return
    const timer = window.setTimeout(() => dismissToast(entry.id), duration)
    return () => window.clearTimeout(timer)
  }, [paused, duration, entry.id])

  return (
    <div
      className={`lui-toast${entry.kind ? ` lui-toast--${entry.kind}` : ''}`}
      role={entry.kind === 'error' ? 'alert' : 'status'}
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
    >
      {entry.kind === 'error' && <CircleAlert size={16} className="lui-toast-icon" />}
      {entry.kind === 'success' && <CircleCheck size={16} className="lui-toast-icon" />}
      <span className="lui-toast-msg">{entry.message}</span>
      {entry.action && (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            entry.action?.onClick()
            dismissToast(entry.id)
          }}
        >
          {entry.action.label}
        </Button>
      )}
      <IconButton label="Dismiss" size="sm" icon={<X size={14} />} onClick={() => dismissToast(entry.id)} />
    </div>
  )
}

export function Toaster() {
  const list = useSyncExternalStore(subscribe, snapshot, snapshot)
  if (list.length === 0) return null
  return (
    <Portal>
      <div className="lui-toaster" aria-live="polite">
        {list.map((entry) => (
          <ToastItem key={entry.id} entry={entry} />
        ))}
      </div>
    </Portal>
  )
}
