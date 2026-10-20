import { describe, expect, test } from 'bun:test'
import { MutationGuard, readGuarded } from './store'

/**
 * Deterministic check for the D12 race: the 10s poll must never apply rows the database held
 * before a local write committed. Every scenario is ordered by resolving deferreds by hand, so
 * there are no timers, sleeps or polling loops involved.
 */

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('poll versus local write', () => {
  test('a read issued while a write is in flight is not even started', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    let readCalls = 0
    const rows = deferred<string[]>()

    guard.begin()
    const read = readGuarded(
      guard,
      () => {
        readCalls += 1
        return rows.promise
      },
      (value) => applied.push(value),
    )

    // Resolving the body proves nothing was awaiting it: the read was suppressed, not queued.
    rows.resolve(['pre-commit'])
    expect(await read).toBe(false)
    expect(readCalls).toBe(0)
    expect(applied).toEqual([])
  })

  test('a read already in flight is discarded when a write starts and settles before it returns', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))

    // The write begins after the read was issued and settles before the read comes back: the rows
    // in flight were produced before that commit, so they must not overwrite the settled state.
    guard.begin()
    guard.settle()
    rows.resolve(['pre-commit'])

    expect(await read).toBe(false)
    expect(applied).toEqual([])
  })

  test('a write that only started (not yet settled) still discards the read', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))
    guard.begin()
    rows.resolve(['pre-commit'])

    expect(await read).toBe(false)
    expect(applied).toEqual([])
  })

  test('a completed local upsert invalidates a read already in flight', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))
    guard.touch()
    rows.resolve(['pre-upsert'])

    expect(await read).toBe(false)
    expect(applied).toEqual([])
  })

  test('an uninterrupted read still applies', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))
    rows.resolve(['current'])

    expect(await read).toBe(true)
    expect(applied).toEqual([['current']])
  })

  test('a read issued after the write settled applies the committed rows', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    guard.begin()
    guard.settle()
    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))
    rows.resolve(['committed'])

    expect(await read).toBe(true)
    expect(applied).toEqual([['committed']])
  })

  test('overlapping writes keep reads suppressed until the last one settles', async () => {
    const guard = new MutationGuard()
    const applied: string[][] = []
    const rows = deferred<string[]>()

    guard.begin()
    guard.begin()
    guard.settle()
    expect(guard.inFlight).toBe(true)

    const read = readGuarded(guard, () => rows.promise, (value) => applied.push(value))
    rows.resolve(['pre-commit'])
    expect(await read).toBe(false)

    guard.settle()
    expect(guard.inFlight).toBe(false)
    expect(applied).toEqual([])
  })
})
