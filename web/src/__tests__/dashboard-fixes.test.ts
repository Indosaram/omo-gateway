import { describe, expect, test } from 'bun:test'
import {
  classifyChatFrame,
  createChatTurn,
  hasAssistantReplyAfter,
  isStopSettled,
  latestMessagePages,
  mergeStatusMessages,
} from '../lib/chat-turn'
import { buildBotUpdatePayload } from '../lib/bot-payload'
import { appendLogs } from '../lib/log-merge'
import { approvalSessionLabel, diskUsage } from '../lib/status-view'
import type { LogEntry } from '../api'

const j = (value: unknown) => JSON.stringify(value)

function harness(
  current = { value: true },
  fallback: () => Promise<unknown> = async () => undefined,
  awaitReply?: () => Promise<void>,
) {
  const log = { chunks: [] as string[], status: [] as string[], done: [] as { error?: string }[], closes: 0 }
  const turn = createChatTurn({
    isCurrent: () => current.value,
    onChunk: (content) => log.chunks.push(content),
    onStatus: (content) => log.status.push(content),
    awaitReply,
    onDone: (outcome) => log.done.push(outcome),
    close: () => {
      log.closes += 1
    },
    fallback,
  })
  return { turn, log }
}

describe('chat turn completion', () => {
  test('direct-reply ack ends the turn', () => {
    expect(classifyChatFrame(j({ type: 'accepted', direct_reply: true, session_id: 's' }))).toEqual({ kind: 'end' })
  })

  test('plain accepted ack does not end the turn', () => {
    expect(classifyChatFrame(j({ type: 'accepted', session_id: 's' }))).toEqual({ kind: 'ignore' })
  })

  test('typing release (silent turn) ends the turn', () => {
    expect(classifyChatFrame(j({ type: 'event', event: { type: 'typing', active: false } }))).toEqual({ kind: 'end' })
  })

  test('error frame without message field still ends the turn with an error', () => {
    expect(classifyChatFrame(j({ error: 'boom' }))).toEqual({ kind: 'error', message: 'boom' })
    expect(classifyChatFrame(j({ type: 'event', event: { type: 'error', message: 'x' } }))).toEqual({
      kind: 'error',
      message: 'x',
    })
  })

  test('final stream chunk is a mid-turn message, not a turn end', () => {
    expect(
      classifyChatFrame(j({ type: 'event', event: { type: 'stream', chunk: { content: 'hi', is_final: true } } })),
    ).toEqual({ kind: 'chunk', content: 'hi', replace: true })
  })

  test('socket close without a terminal frame completes and closes once', () => {
    const { turn, log } = harness()
    turn.opened()
    turn.frame(j({ type: 'ready' }))
    turn.closed()
    turn.closed()
    expect(log.done).toEqual([{}])
    expect(log.closes).toBe(1)
  })

  test('terminal frame closes the socket and completes exactly once', () => {
    const { turn, log } = harness()
    turn.opened()
    turn.frame(j({ type: 'event', event: { type: 'stream', chunk: { content: 'par', is_final: false } } }))
    turn.frame(j({ type: 'error', message: 'nope' }))
    turn.closed()
    expect(log.chunks).toEqual(['par'])
    expect(log.done).toEqual([{ error: 'nope' }])
    expect(log.closes).toBe(1)
  })

  test('HTTP fallback completes the turn only after the POST resolves', async () => {
    let resolvePost!: () => void
    const post = new Promise<void>((resolve) => {
      resolvePost = resolve
    })
    const { turn, log } = harness({ value: true }, () => post)
    const pending = turn.socketError()
    turn.closed()
    expect(log.done).toEqual([])
    resolvePost()
    await pending
    expect(log.done).toEqual([{}])
    expect(log.closes).toBe(1)
  })
})

describe('chat turn session binding', () => {
  test('events for a session that is no longer open are ignored and the socket is closed', () => {
    const current = { value: true }
    const { turn, log } = harness(current)
    turn.opened()
    current.value = false
    turn.frame(j({ type: 'event', event: { type: 'stream', chunk: { content: 'leak', is_final: false } } }))
    turn.frame(j({ type: 'final', content: 'leak' }))
    expect(log.chunks).toEqual([])
    expect(log.done).toEqual([])
    expect(log.closes).toBe(1)
    expect(turn.finished).toBe(true)
  })

  test('abandoning a turn closes it without completing the new session', () => {
    const { turn, log } = harness()
    turn.abandon()
    turn.closed()
    expect(log.done).toEqual([])
    expect(log.closes).toBe(1)
  })
})

describe('latest message pages', () => {
  test('short session loads the single page', () => {
    expect(latestMessagePages(10, 50)).toEqual([1])
  })

  test('long session loads the newest pages, not page 1', () => {
    expect(latestMessagePages(120, 50)).toEqual([2, 3])
    expect(latestMessagePages(150, 50)).toEqual([3])
  })
})

describe('bot update payload', () => {
  test('clearing the persona sends an explicit empty string, not null', () => {
    const payload = buildBotUpdatePayload({ name: 'b', model: 'm', prompt: '   ', toolsets: 'terminal' })
    expect(payload.system_prompt).toBe('')
  })

  test('clearing model and toolsets sends explicit empty values the backend clears', () => {
    const payload = buildBotUpdatePayload({ name: 'b', model: '  ', prompt: 'p', toolsets: ' , ' })
    expect(payload.model).toBe('')
    expect(payload.enabled_toolsets).toEqual([])
  })
})

describe('mid-turn status messages', () => {
  test('send_message event is its own message, not a chunk or a turn end', () => {
    expect(classifyChatFrame(j({ type: 'event', event: { type: 'send_message', content: 'checking files' } }))).toEqual({
      kind: 'message',
      content: 'checking files',
    })
    expect(classifyChatFrame(j({ type: 'event', event: { type: 'send_message', content: '  ' } }))).toEqual({
      kind: 'ignore',
    })
  })

  test('status text is surfaced and the turn keeps running', () => {
    const { turn, log } = harness()
    turn.opened()
    turn.frame(j({ type: 'event', event: { type: 'send_message', content: 'first' } }))
    turn.frame(j({ type: 'event', event: { type: 'send_message', content: 'second' } }))
    expect(log.status).toEqual(['first', 'second'])
    expect(log.chunks).toEqual([])
    expect(log.done).toEqual([])
  })

  test('status messages are placed after the newest user message', () => {
    const items = [
      { role: 'user', id: 'u1' },
      { role: 'assistant', id: 'a1' },
      { role: 'user', id: 'u2' },
      { role: 'assistant', id: 'a2' },
    ]
    const merged = mergeStatusMessages(items, [{ role: 'assistant', id: 's1' }])
    expect(merged.map((m) => m.id)).toEqual(['u1', 'a1', 'u2', 's1', 'a2'])
    expect(mergeStatusMessages(items, [])).toBe(items)
  })
})

describe('HTTP fallback queued turn', () => {
  test('a queued 202 waits for the reply before completing', async () => {
    let replyReady!: () => void
    const reply = new Promise<void>((resolve) => {
      replyReady = resolve
    })
    const { turn, log } = harness({ value: true }, async () => ({ queued: true }), () => reply)
    const pending = turn.socketError()
    await Promise.resolve()
    expect(log.done).toEqual([])
    replyReady()
    await pending
    expect(log.done).toEqual([{}])
  })

  test('a direct reply (not queued) completes without waiting', async () => {
    let waited = false
    const { turn, log } = harness(
      { value: true },
      async () => ({ queued: false, direct_reply: true }),
      async () => {
        waited = true
      },
    )
    await turn.socketError()
    expect(waited).toBe(false)
    expect(log.done).toEqual([{}])
  })

  test('reply detection only counts assistant messages newer than the sent turn', () => {
    const items = [
      { role: 'user', content: 'q', sequence: 4 },
      { role: 'assistant', content: 'old', sequence: 3 },
    ]
    expect(hasAssistantReplyAfter(items, 4)).toBe(false)
    expect(hasAssistantReplyAfter([...items, { role: 'assistant', content: 'new', sequence: 6 }], 4)).toBe(true)
    expect(hasAssistantReplyAfter([{ role: 'assistant', content: ' ', sequence: 7 }], 4)).toBe(false)
  })
})

describe('stop settlement', () => {
  test('only the stopped ack or an error settles the stop socket', () => {
    expect(isStopSettled(j({ type: 'stopped', stopped: true }))).toBe(true)
    expect(isStopSettled(j({ type: 'error', message: 'x' }))).toBe(true)
    expect(isStopSettled(j({ type: 'ready' }))).toBe(false)
    expect(isStopSettled(j({ type: 'event', event: { type: 'typing', active: false } }))).toBe(false)
    expect(isStopSettled('not json')).toBe(false)
  })

  test('an abandoned (stopped) turn ignores its later typing release', () => {
    const { turn, log } = harness()
    turn.opened()
    turn.abandon()
    turn.frame(j({ type: 'event', event: { type: 'typing', active: false } }))
    expect(log.done).toEqual([])
    expect(log.closes).toBe(1)
  })
})

describe('log merging', () => {
  const entry = (id: number): LogEntry => ({
    id,
    timestamp: '2026-10-13T00:00:00Z',
    level: 'INFO',
    target: 't',
    message: `m${id}`,
    fields: {},
  })

  test('socket replay of the HTTP seed does not duplicate ids', () => {
    const seeded = appendLogs([], [entry(1), entry(2), entry(3)])
    const replayed = appendLogs(seeded, [entry(2), entry(3), entry(4)])
    expect(replayed.map((e) => e.id)).toEqual([1, 2, 3, 4])
  })

  test('late HTTP seed keeps entries already received live', () => {
    const live = appendLogs([], [entry(3), entry(4)])
    expect(appendLogs([entry(1), entry(2), entry(3)], live).map((e) => e.id)).toEqual([1, 2, 3, 4])
  })
})

describe('overview and approval views', () => {
  test('approval label comes from the session object', () => {
    expect(approvalSessionLabel({ platform: 'discord', channel_id: 'c1', user_id: 'u1' })).toBe('discord:c1 (u1)')
    expect(approvalSessionLabel(undefined)).toBe('unknown session')
  })

  test('unreported disk sample is unknown, not 100% used', () => {
    expect(diskUsage({ workspace_total_bytes: null, workspace_available_bytes: null })).toBeNull()
    expect(diskUsage({ workspace_total_bytes: 200, workspace_available_bytes: 50 })).toEqual({
      used: 150,
      total: 200,
      percent: 75,
    })
  })
})
