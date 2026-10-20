import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from 'react'
import { useTracker } from '../store'
import type { Issue } from '../types'
import { Markdown, notifyError, useAutosize } from './shared'

export interface TitleHandle {
  focus: () => void
}

export const TitleEditor = forwardRef<TitleHandle, { issue: Issue }>(function TitleEditor({ issue }, ref) {
  const t = useTracker()
  const [value, setValue] = useState(issue.title)
  const [editing, setEditing] = useState(false)
  const el = useRef<HTMLTextAreaElement>(null)
  useAutosize(el, value)

  useEffect(() => {
    if (!editing) setValue(issue.title)
  }, [issue.title, editing])

  useImperativeHandle(ref, () => ({
    focus: () => {
      const node = el.current
      if (!node) return
      node.focus()
      node.setSelectionRange(node.value.length, node.value.length)
    },
  }))

  const commit = () => {
    setEditing(false)
    const next = value.replace(/\s+/g, ' ').trim()
    if (!next) {
      setValue(issue.title)
      return
    }
    if (next !== issue.title) t.updateIssues([issue.id], { title: next }).catch(notifyError)
  }

  return (
    <textarea
      ref={el}
      className="ld-title"
      rows={1}
      value={value}
      aria-label="Issue title"
      placeholder="Issue title"
      onFocus={() => setEditing(true)}
      onChange={(e) => setValue(e.target.value.replace(/\n/g, ''))}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault()
          e.currentTarget.blur()
        } else if (e.key === 'Escape') {
          e.preventDefault()
          e.stopPropagation()
          setValue(issue.title)
          setEditing(false)
          requestAnimationFrame(() => el.current?.blur())
        }
      }}
    />
  )
})

const AUTOSAVE_MS = 800

export function DescriptionEditor({ issue, compact = false }: { issue: Issue; compact?: boolean }) {
  const t = useTracker()
  const [editing, setEditing] = useState(false)
  const [tab, setTab] = useState<'write' | 'preview'>('write')
  const [value, setValue] = useState(issue.description)
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved'>('idle')
  const el = useRef<HTMLTextAreaElement>(null)
  const timer = useRef<number | undefined>(undefined)
  const lastSaved = useRef(issue.description)
  const issueId = issue.id
  const updateIssues = t.updateIssues
  useAutosize(el, value, compact ? 80 : 120)

  useEffect(() => {
    if (!editing) {
      setValue(issue.description)
      lastSaved.current = issue.description
    }
  }, [issue.description, editing])

  const save = (next: string) => {
    window.clearTimeout(timer.current)
    if (next === lastSaved.current) return
    const previous = lastSaved.current
    lastSaved.current = next
    setSaveState('saving')
    updateIssues([issueId], { description: next })
      .then(() => setSaveState('saved'))
      .catch((err) => {
        // Not saved: let the next blur or keystroke retry instead of treating it as saved.
        lastSaved.current = previous
        setSaveState('idle')
        notifyError(err)
      })
  }

  // Closing the peek by clicking outside unmounts the editor before blur fires: flush the
  // pending autosave instead of dropping the last keystrokes.
  const latest = useRef(value)
  latest.current = value
  const saveRef = useRef(save)
  saveRef.current = save
  useEffect(
    () => () => {
      window.clearTimeout(timer.current)
      if (latest.current !== lastSaved.current) saveRef.current(latest.current)
    },
    [],
  )

  const onChange = (next: string) => {
    setValue(next)
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => save(next), AUTOSAVE_MS)
  }

  const finish = () => {
    save(value)
    setEditing(false)
    setTab('write')
  }

  if (!editing) {
    return (
      <div
        className="ld-desc ld-desc-view"
        role="button"
        tabIndex={0}
        aria-label="Edit description"
        onClick={(e) => {
          if ((e.target as HTMLElement).closest('a')) return
          setEditing(true)
          requestAnimationFrame(() => el.current?.focus())
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            setEditing(true)
            requestAnimationFrame(() => el.current?.focus())
          }
        }}
      >
        {value.trim() ? <Markdown source={value} /> : <div className="ld-desc-placeholder">Add description…</div>}
      </div>
    )
  }

  return (
    <div
      className="ld-desc ld-desc-editor"
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) finish()
      }}
    >
      <div className="ld-desc-tabs" role="tablist">
        <button type="button" role="tab" className="ld-tab" aria-selected={tab === 'write'} onClick={() => setTab('write')}>
          Write
        </button>
        <button
          type="button"
          role="tab"
          className="ld-tab"
          aria-selected={tab === 'preview'}
          onClick={() => setTab('preview')}
        >
          Preview
        </button>
        <span className="ld-desc-status">
          {saveState === 'saving' ? 'Saving…' : saveState === 'saved' ? 'Saved' : 'Markdown supported'}
        </span>
      </div>
      {tab === 'write' ? (
        <textarea
          ref={el}
          className="ld-desc-textarea"
          value={value}
          placeholder="Add description…"
          aria-label="Issue description"
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape' || (e.key === 'Enter' && (e.metaKey || e.ctrlKey))) {
              e.preventDefault()
              e.stopPropagation()
              finish()
              ;(document.activeElement as HTMLElement | null)?.blur()
            }
          }}
        />
      ) : (
        <div className="ld-desc-preview" tabIndex={-1}>
          {value.trim() ? <Markdown source={value} /> : <div className="ld-desc-placeholder">Nothing to preview</div>}
        </div>
      )}
    </div>
  )
}
