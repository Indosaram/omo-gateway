import { describe, expect, test } from 'bun:test'
import { createDraftSaver, createUnmountFlush, retainedDraft, type DraftSnapshot } from './descriptionDraft'

/** A request the test holds open and then resolves or rejects: no timers, no sleeps. */
function deferred() {
  let resolve!: () => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<void>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

let counter = 0

function harness(initial: string, issueId = `issue-${++counter}`) {
  const calls: string[] = []
  const gates: ReturnType<typeof deferred>[] = []
  const snapshots: DraftSnapshot[] = []
  const waiters: (() => void)[] = []
  const saver = createDraftSaver({
    issueId,
    initial,
    save: (text) => {
      calls.push(text)
      for (const wake of waiters.splice(0)) wake()
      const gate = deferred()
      gates.push(gate)
      return gate.promise
    },
    onChange: (snapshot) => snapshots.push(snapshot),
  })
  /** Resolves once the n-th save (0-based) has been issued, without polling. */
  const waitForCall = (index: number) =>
    calls.length > index ? Promise.resolve() : new Promise<void>((resolve) => waiters.push(resolve))
  return { saver, calls, gates, snapshots, waitForCall, issueId }
}

describe('description draft retention', () => {
  test('keeps text typed while a failing request was in flight, not the text it carried', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const flushing = h.saver.flush()
    expect(h.calls).toEqual(['X'])
    h.saver.edit('Y') // typed during the round trip
    h.saver.dispose() // peek closed, editor unmounts
    h.gates[0].reject(new Error('offline'))
    expect(await flushing).toBe(false)
    expect(retainedDraft(h.issueId)).toBe('Y')
    expect(h.saver.state()).toBe('failed')
    expect(h.saver.dirty()).toBe(true)
  })

  test('a stale success does not clear a newer draft', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const flushing = h.saver.flush()
    h.saver.edit('Y')
    h.gates[0].resolve()
    await h.waitForCall(1)
    expect(h.calls).toEqual(['X', 'Y'])
    expect(retainedDraft(h.issueId)).toBe('Y')
    h.gates[1].reject(new Error('offline'))
    expect(await flushing).toBe(false)
    expect(retainedDraft(h.issueId)).toBe('Y')
    expect(h.saver.state()).toBe('failed')
  })

  test('clears retention and reports saved once the newest text is acknowledged', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const flushing = h.saver.flush()
    h.gates[0].resolve()
    expect(await flushing).toBe(true)
    expect(h.calls).toEqual(['X'])
    expect(retainedDraft(h.issueId)).toBeUndefined()
    expect(h.saver.state()).toBe('saved')
    expect(h.saver.dirty()).toBe(false)
  })

  test('runs one request at a time and always sends the newest text', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const first = h.saver.flush()
    h.saver.edit('Y')
    const second = h.saver.flush()
    expect(second).toBe(first)
    expect(h.calls).toEqual(['X'])
    h.gates[0].resolve()
    await h.waitForCall(1)
    expect(h.calls).toEqual(['X', 'Y'])
    h.gates[1].resolve()
    expect(await first).toBe(true)
    expect(retainedDraft(h.issueId)).toBeUndefined()
    expect(h.saver.value()).toBe('Y')
  })

  test('ignores the store rollback while a draft is pending, then retries it', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const flushing = h.saver.flush()
    h.gates[0].reject(new Error('offline'))
    expect(await flushing).toBe(false)
    h.saver.sync('original') // optimistic update rolled back in the store
    expect(h.saver.value()).toBe('X')
    expect(h.saver.dirty()).toBe(true)
    const retry = h.saver.flush()
    h.gates[1].resolve()
    expect(await retry).toBe(true)
    expect(h.calls).toEqual(['X', 'X'])
    expect(retainedDraft(h.issueId)).toBeUndefined()
    h.saver.sync('X')
    expect(h.saver.value()).toBe('X')
    expect(h.saver.dirty()).toBe(false)
  })

  test('keeps text that never reached the network when the editor unmounts', () => {
    const h = harness('original')
    h.saver.edit('typed')
    h.saver.dispose()
    expect(retainedDraft(h.issueId)).toBe('typed')
    expect(h.calls).toEqual([])
  })

  test('reopening the editor restores the retained draft and retries it', async () => {
    const h = harness('original')
    h.saver.edit('kept')
    const flushing = h.saver.flush()
    h.gates[0].reject(new Error('offline'))
    await flushing
    const reopened = harness('original', h.issueId)
    expect(reopened.saver.value()).toBe('kept')
    expect(reopened.saver.state()).toBe('failed')
    expect(reopened.saver.dirty()).toBe(true)
    const retry = reopened.saver.flush()
    reopened.gates[0].resolve()
    expect(await retry).toBe(true)
    expect(retainedDraft(h.issueId)).toBeUndefined()
  })

  test('a remounted instance keeps its draft when the old request succeeds later', async () => {
    const old = harness('original')
    old.saver.edit('X')
    const stale = old.saver.flush()
    old.saver.dispose() // peek closes while X is still out
    const next = harness('original', old.issueId)
    expect(next.saver.value()).toBe('X')
    next.saver.edit('Y') // typed in the reopened editor
    old.gates[0].resolve() // the stale request lands after the remount
    expect(await stale).toBe(true)
    expect(retainedDraft(old.issueId)).toBe('Y')
    expect(next.saver.value()).toBe('Y')
    const flushing = next.saver.flush()
    next.gates[0].resolve()
    expect(await flushing).toBe(true)
    expect(next.calls).toEqual(['Y'])
    expect(retainedDraft(old.issueId)).toBeUndefined()
  })

  test('a remounted instance keeps its draft when the old request fails later', async () => {
    const old = harness('original')
    old.saver.edit('X')
    const stale = old.saver.flush()
    old.saver.dispose()
    const next = harness('original', old.issueId)
    next.saver.edit('Y')
    old.gates[0].reject(new Error('offline'))
    expect(await stale).toBe(false)
    expect(retainedDraft(old.issueId)).toBe('Y')
    expect(next.saver.state()).toBe('idle')
    expect(next.saver.dirty()).toBe(true)
  })

  // The browser QA failure (case B): at reopen the store still holds the optimistic copy of the
  // pending text, so the remounted saver is constructed with initial === the draft. Taking that as
  // the baseline made dirty() false, so Retry issued no request and sync() adopted the rollback.
  test('a remount over the optimistic text still reports the draft as unsaved', async () => {
    const old = harness('original')
    old.saver.edit('X')
    const stale = old.saver.flush()
    old.saver.dispose()
    // Reopen while the write is in flight: the component's `initial` is the optimistic 'X'.
    const next = harness('X', old.issueId)
    expect(next.saver.value()).toBe('X')
    expect(next.saver.dirty()).toBe(true)
    expect(next.saver.state()).toBe('failed')
    // Retry issues a real request instead of finding draft === lastSaved and returning early.
    const retry = next.saver.flush()
    expect(next.calls).toEqual(['X'])
    // The in-flight write fails and the store rolls back to the server text.
    old.gates[0].reject(new Error('offline'))
    expect(await stale).toBe(false)
    next.saver.sync('original')
    expect(next.saver.value()).toBe('X')
    expect(next.saver.dirty()).toBe(true)
    expect(retainedDraft(old.issueId)).toBe('X')
    // Retry after recovery saves it once and clears the bar.
    next.gates[0].resolve()
    expect(await retry).toBe(true)
    expect(next.calls).toEqual(['X'])
    expect(retainedDraft(old.issueId)).toBeUndefined()
    next.saver.sync('X')
    expect(next.saver.dirty()).toBe(false)
    expect(next.saver.state()).toBe('saved')
  })

  // The browser QA failure (case C): the stale instance's rollback must not clear the newer draft.
  test('a remount over the optimistic text survives the old failure landing after the retry', async () => {
    const old = harness('original')
    old.saver.edit('X')
    const stale = old.saver.flush()
    old.saver.dispose()
    const next = harness('X', old.issueId)
    const retry = next.saver.flush()
    old.gates[0].reject(new Error('offline'))
    expect(await stale).toBe(false)
    next.saver.sync('original') // rollback arrives after the remount
    expect(next.saver.value()).toBe('X')
    expect(retainedDraft(old.issueId)).toBe('X')
    expect(next.saver.state()).toBe('saving')
    next.gates[0].reject(new Error('still offline'))
    expect(await retry).toBe(false)
    expect(next.saver.value()).toBe('X')
    expect(retainedDraft(old.issueId)).toBe('X')
    expect(next.saver.state()).toBe('failed')
  })

  test('an acknowledged write from the old instance keeps the draft until this instance saves it', async () => {    const old = harness('original')
    old.saver.edit('X')
    const stale = old.saver.flush()
    old.saver.dispose()
    const next = harness('X', old.issueId)
    old.gates[0].resolve() // the write did land after all
    expect(await stale).toBe(true)
    next.saver.sync('X') // the store now holds the server's copy
    // Still shown as pending: only this instance's own resolved request proves it, and re-sending
    // the same text is harmless, whereas adopting on equality would lose a draft in the failing case.
    expect(next.saver.value()).toBe('X')
    expect(next.saver.dirty()).toBe(true)
    const flushing = next.saver.flush()
    next.gates[0].resolve()
    expect(await flushing).toBe(true)
    expect(next.calls).toEqual(['X'])
    expect(retainedDraft(old.issueId)).toBeUndefined()
    expect(next.saver.dirty()).toBe(false)
    expect(next.saver.state()).toBe('saved')
  })

  // The browser QA failure (case B): in the reopened peek, Retry dispatched its click but issued no
  // PATCH and the "Unsaved changes" bar never cleared. Cause: StrictMode's simulated unmount ran
  // the cleanup, which fired the owed save; that request stayed in flight, and `flush()` handed the
  // in-flight promise back to the Retry click instead of starting a request.
  test('a simulated unmount fires no save, so the Retry click still issues a request', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const owed = createUnmountFlush(() => {
      if (h.saver.dirty()) void h.saver.flush()
    })
    // StrictMode: effect, cleanup, effect - all before the queued microtask can run.
    owed.attach()
    owed.detach()
    owed.attach()
    await Promise.resolve()
    await Promise.resolve()
    expect(h.calls).toEqual([])
    expect(retainedDraft(h.issueId)).toBe('X')
    // The Retry click is not swallowed by a phantom request.
    const retry = h.saver.flush()
    expect(h.calls).toEqual(['X'])
    h.gates[0].resolve()
    expect(await retry).toBe(true)
    expect(retainedDraft(h.issueId)).toBeUndefined()
    expect(h.saver.dirty()).toBe(false)
  })

  test('a real unmount still saves the owed draft', async () => {
    const h = harness('original')
    h.saver.edit('X')
    const owed = createUnmountFlush(() => {
      if (h.saver.dirty()) void h.saver.flush()
    })
    owed.attach()
    owed.detach() // the peek really closed: nothing remounts it
    await Promise.resolve()
    await Promise.resolve()
    expect(h.calls).toEqual(['X'])
    h.gates[0].resolve()
    await Promise.resolve()
    expect(h.saver.dirty()).toBe(false)
  })
})
