/**
 * Draft retention for the issue description editor.
 *
 * Deliberately framework-free: the peek unmounts the editor while a save can still be in flight, so
 * the rules about what counts as saved, what survives the unmount and which text a failed request
 * must keep are testable without a DOM (see descriptionDraft.test.ts).
 */

export type SaveState = 'idle' | 'saving' | 'saved' | 'failed'

export interface DraftSnapshot {
  value: string
  state: SaveState
  /** True while the text differs from the last text the server acknowledged. */
  dirty: boolean
}

/**
 * Drafts that must outlive the editor, keyed by issue id.
 *
 * - `text` is the newest text the user typed, which the server has not acknowledged.
 * - `baseline` is the last text the server *did* acknowledge. It has to be persisted: on remount the
 *   store may still hold the optimistic copy of the pending text, so the component's `initial` is
 *   not a reliable baseline (taking it made a retained draft look saved, which turned Retry into a
 *   no-op and let the rollback overwrite the draft).
 * - `owner` is the id of the saver that wrote the entry: reopening the same issue creates a newer
 *   saver, and the older one must not overwrite or clear text the newer one now owns.
 */
const retained = new Map<string, { text: string; baseline: string; owner: number }>()
/** Highest saver id ever created per issue, kept even after the entry is cleared. */
const latestOwner = new Map<string, number>()
let nextOwner = 0

/** Text a closed editor left unsaved, or undefined when the issue has nothing pending. */
export function retainedDraft(issueId: string): string | undefined {
  return retained.get(issueId)?.text
}

/**
 * The save a closing editor still owes, scheduled so a *simulated* unmount does not fire it.
 *
 * React StrictMode mounts, unmounts and remounts every effect in development, and the simulated
 * unmount runs the same cleanup as a real one. Firing the owed save there started a request while
 * the peek was still open; `flush()` then handed that in-flight promise back to the user's Retry
 * instead of issuing a request, so Retry looked like a no-op (no PATCH, no toast, bar never
 * cleared). The save is therefore deferred by one microtask and dropped when the component comes
 * straight back, which only ever happens for the simulated unmount.
 */
export function createUnmountFlush(flush: () => void): { attach: () => void; detach: () => void } {
  let pending: { cancel: boolean } | null = null
  return {
    /** Effect body: cancel the flush owed by a previous unmount that has already been undone. */
    attach: () => {
      if (pending) pending.cancel = true
      pending = null
    },
    /** Effect cleanup: schedule the owed save unless the component is about to come back. */
    detach: () => {
      const token = { cancel: false }
      pending = token
      queueMicrotask(() => {
        if (!token.cancel) flush()
      })
    },
  }
}

export interface DraftSaver {
  value: () => string
  state: () => SaveState
  dirty: () => boolean
  /** Records a keystroke; keeps the newest text, also outside the component. */
  edit: (text: string) => void
  /** Adopts the server copy, unless local text is still pending. */
  sync: (serverText: string) => void
  /** Saves the newest text, one request at a time. Resolves false when it failed and kept it. */
  flush: () => Promise<boolean>
  /** Called before the editor goes away: keeps the newest text synchronously. */
  dispose: () => void
}

export function createDraftSaver({
  issueId,
  initial,
  save,
  onChange,
  onError,
}: {
  issueId: string
  /** The description the server currently has. */
  initial: string
  save: (text: string) => Promise<unknown>
  onChange?: (snapshot: DraftSnapshot) => void
  onError?: (error: unknown) => void
}): DraftSaver {
  const owner = ++nextOwner
  const entry = retained.get(issueId)
  // The retained baseline, not `initial`: while a write is pending the store holds the optimistic
  // copy of the draft, so `initial` can equal the draft even though the server never took it.
  let lastSaved = entry?.baseline ?? initial
  let draft = entry?.text ?? initial
  let state: SaveState = entry === undefined ? 'idle' : 'failed'
  let inFlight: Promise<boolean> | null = null
  // This instance is the newest for the issue: it owns whatever draft it inherited.
  latestOwner.set(issueId, owner)
  if (entry) retained.set(issueId, { text: entry.text, baseline: entry.baseline, owner })

  const dirty = () => draft !== lastSaved
  const emit = () => onChange?.({ value: draft, state, dirty: dirty() })
  const isCurrent = () => latestOwner.get(issueId) === owner
  // Keep the newest text outside the component, or drop the entry when there is nothing to save.
  // Only the newest saver for an issue may touch the entry, so a late response from a disposed
  // instance can neither overwrite nor resurrect a draft the newer instance already saved.
  const retain = () => {
    if (!isCurrent()) return
    if (!dirty()) retained.delete(issueId)
    else retained.set(issueId, { text: draft, baseline: lastSaved, owner })
  }

  const run = async (): Promise<boolean> => {
    while (dirty()) {
      const sending = draft
      state = 'saving'
      emit()
      try {
        await save(sending)
      } catch (err) {
        // The user may have typed while the request was out: keep the newest text, not `sending`.
        retain()
        state = 'failed'
        emit()
        onError?.(err)
        return false
      }
      lastSaved = sending
      // Only clear retention when nothing newer is waiting: a stale success must not drop a newer draft.
      retain()
    }
    state = 'saved'
    emit()
    return true
  }

  const flush = (): Promise<boolean> => {
    if (inFlight) return inFlight
    const tracked = run().finally(() => {
      inFlight = null
    })
    inFlight = tracked
    return tracked
  }

  return {
    value: () => draft,
    state: () => state,
    dirty,
    edit: (text) => {
      draft = text
      retain()
      state = 'idle'
      emit()
    },
    sync: (serverText) => {
      // Only `run` advances the baseline, from a request that actually resolved: the store also
      // reports the optimistic copy of a pending write here, and treating that as acknowledged is
      // what let the rollback overwrite a retained draft.
      if (dirty()) return
      lastSaved = serverText
      draft = serverText
      if (isCurrent()) retained.delete(issueId)
      emit()
    },
    flush,
    dispose: () => {
      // Synchronous on purpose: the component is going away right now.
      retain()
    },
  }
}
