import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from 'react'
import { useTracker } from '../store'
import type { Issue } from '../types'
import { createDraftSaver, createUnmountFlush, retainedDraft, type DraftSaver, type SaveState } from './descriptionDraft'
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
  const [value, setValue] = useState(() => retainedDraft(issue.id) ?? issue.description)
  const [saveState, setSaveState] = useState<SaveState>(() =>
    retainedDraft(issue.id) === undefined ? 'idle' : 'failed',
  )
  const [pending, setPending] = useState(() => retainedDraft(issue.id) !== undefined)
  const el = useRef<HTMLTextAreaElement>(null)
  const timer = useRef<number | undefined>(undefined)
  const issueId = issue.id
  const updateIssues = t.updateIssues
  useAutosize(el, value, compact ? 80 : 120)

  // The saver owns the draft (the newest text, what the server acknowledged, and what must survive
  // this component), so the rules stay testable without a DOM: descriptionDraft.test.ts.
  const updateIssuesRef = useRef(updateIssues)
  updateIssuesRef.current = updateIssues
  const saverRef = useRef<DraftSaver | null>(null)
  if (saverRef.current === null) {
    saverRef.current = createDraftSaver({
      issueId,
      initial: issue.description,
      save: (text) => updateIssuesRef.current([issueId], { description: text }),
      onChange: (snapshot) => {
        setValue(snapshot.value)
        setSaveState(snapshot.state)
        setPending(snapshot.dirty)
      },
      onError: notifyError,
    })
  }
  const saver = saverRef.current

  useEffect(() => {
    // Adopts the server copy only when nothing typed is waiting: a failed save rolls the issue back
    // in the store, and adopting that value would throw the user's draft away.
    saver.sync(issue.description)
  }, [saver, issue.description])

  const flush = () => {
    window.clearTimeout(timer.current)
    return saver.flush()
  }

  // Closing the peek by clicking outside unmounts the editor before blur fires: keep the newest text
  // synchronously and still try to save it, so nothing typed is dropped. `createUnmountFlush` drops
  // that save when StrictMode's simulated unmount immediately remounts us, which is what used to
  // leave a phantom request in flight that swallowed the user's Retry.
  const flushRef = useRef(flush)
  flushRef.current = flush
  const unmountFlush = useRef<{ attach: () => void; detach: () => void } | null>(null)
  if (unmountFlush.current === null) {
    unmountFlush.current = createUnmountFlush(() => {
      if (saver.dirty()) void flushRef.current()
    })
  }
  const owed = unmountFlush.current
  useEffect(() => {
    owed.attach()
    return () => {
      window.clearTimeout(timer.current)
      saver.dispose()
      owed.detach()
    }
  }, [saver, owed])

  const onChange = (next: string) => {
    saver.edit(next)
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => void flushRef.current(), AUTOSAVE_MS)
  }

  const finish = () => {
    setTab('write')
    void flush().then((saved) => {
      // A failed save keeps the editor open with the text and a Retry button instead of discarding it.
      if (saved) setEditing(false)
    })
  }

  if (!editing) {
    return (
      <>
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
        {pending && (
          <div className="ld-desc-pending">
            Unsaved changes
            <button type="button" className="ld-tab" onClick={() => void flush()}>
              Retry
            </button>
          </div>
        )}
      </>
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
          {saveState === 'saving'
            ? 'Saving…'
            : saveState === 'saved'
              ? 'Saved'
              : saveState === 'failed'
                ? 'Not saved'
                : 'Markdown supported'}
        </span>
        {saveState === 'failed' && (
          <button type="button" className="ld-tab" onClick={() => void flush()}>
            Retry
          </button>
        )}
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
