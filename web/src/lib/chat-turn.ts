/**
 * Pure helpers for the dashboard chat playground turn lifecycle.
 *
 * A turn opens one WebSocket to /api/sessions/{id}/ws. The backend never sends a
 * dedicated "turn finished" frame for every turn shape. A stream chunk with `is_final`
 * set is NOT a turn end; streamed turns end when the actor releases typing. Mid-turn
 * status text (the agent narrating before a tool call) arrives as
 * `{type:"event",event:{type:"send_message",content}}`; it is not persisted, so the
 * page shows it as its own message. Direct replies are acknowledged with
 * `{type:"accepted",direct_reply:true}`, silent turns only release typing
 * (`{type:"event",event:{type:"typing",active:false}}`), and failures arrive as
 * `{type:"error"}`. Every one of those must complete the turn exactly once.
 */

export type ChatFrame =
  | { kind: 'chunk'; content: string; replace: boolean }
  | { kind: 'message'; content: string }
  | { kind: 'end' }
  | { kind: 'error'; message: string }
  | { kind: 'ignore' }

type Obj = Record<string, unknown>

const isObj = (value: unknown): value is Obj => typeof value === 'object' && value !== null

function errorText(source: Obj): string {
  for (const field of ['message', 'error']) {
    const value = source[field]
    if (typeof value === 'string' && value) return value
  }
  return 'unknown error'
}

export function classifyChatFrame(raw: string): ChatFrame {
  let data: unknown
  try {
    data = JSON.parse(raw)
  } catch {
    return { kind: 'chunk', content: raw, replace: false }
  }
  if (!isObj(data)) return { kind: 'chunk', content: raw, replace: false }

  const type = data.type
  if (type === 'error' || (type === undefined && data.error !== undefined)) {
    return { kind: 'error', message: errorText(data) }
  }
  if (type === 'final' || type === 'stopped') return { kind: 'end' }
  if (type === 'accepted') return data.direct_reply === true ? { kind: 'end' } : { kind: 'ignore' }
  if (type === 'stream_chunk') {
    return { kind: 'chunk', content: String(data.content ?? ''), replace: true }
  }
  if (type === 'event' && isObj(data.event)) {
    const ev = data.event
    if (ev.type === 'error') return { kind: 'error', message: errorText(ev) }
    if (ev.type === 'stream' && isObj(ev.chunk)) {
      return { kind: 'chunk', content: String(ev.chunk.content ?? ''), replace: true }
    }
    if (ev.type === 'send_message') {
      const content = String(ev.content ?? '')
      return content.trim() ? { kind: 'message', content } : { kind: 'ignore' }
    }
    // The actor releases typing at the end of every turn, including silent ones.
    if (ev.type === 'typing' && ev.active === false) return { kind: 'end' }
    if (ev.type === 'final') return { kind: 'end' }
  }
  return { kind: 'ignore' }
}

export interface ChatTurnHooks {
  isCurrent: () => boolean
  onChunk: (content: string) => void
  onStatus: (content: string) => void
  onDone: (outcome: { error?: string }) => void
  close: () => void
  fallback: () => Promise<unknown>
  /** Resolves once the reply of a queued (HTTP fallback) turn is available. */
  awaitReply?: () => Promise<void>
}

export interface ChatTurn {
  readonly finished: boolean
  opened: () => void
  frame: (raw: string) => void
  closed: () => void
  socketError: () => Promise<void>
  abandon: () => void
}

export function createChatTurn(hooks: ChatTurnHooks): ChatTurn {
  let finished = false
  let sent = false
  let fallingBack = false
  let accumulated = ''

  const finish = (outcome: { error?: string } | null) => {
    if (finished) return
    finished = true
    hooks.close()
    if (outcome && hooks.isCurrent()) hooks.onDone(outcome)
  }

  return {
    get finished() {
      return finished
    },
    opened() {
      sent = true
    },
    frame(raw) {
      if (finished) return
      if (!hooks.isCurrent()) {
        finish(null)
        return
      }
      const frame = classifyChatFrame(raw)
      switch (frame.kind) {
        case 'chunk':
          accumulated = frame.replace ? frame.content : accumulated + frame.content
          hooks.onChunk(accumulated)
          return
        case 'message':
          hooks.onStatus(frame.content)
          return
        case 'end':
          finish({})
          return
        case 'error':
          finish({ error: frame.message })
          return
        case 'ignore':
          return
      }
    },
    closed() {
      if (fallingBack) return
      finish({})
    },
    async socketError() {
      if (finished || fallingBack) return
      if (sent) {
        finish({ error: 'chat connection lost' })
        return
      }
      fallingBack = true
      try {
        const ack = await hooks.fallback()
        // The POST only queues the turn (202); the reply is produced afterwards.
        if (hooks.awaitReply && isObj(ack) && ack.queued === true) await hooks.awaitReply()
        fallingBack = false
        finish({})
      } catch (err) {
        fallingBack = false
        finish({ error: err instanceof Error ? err.message : String(err) })
      }
    },
    abandon() {
      finish(null)
    },
  }
}

export function latestMessagePages(total: number, perPage: number): number[] {
  const last = Math.max(1, Math.ceil(total / perPage))
  if (last > 1 && total % perPage !== 0) return [last - 1, last]
  return [last]
}

/** Whether the loaded messages hold a final assistant reply newer than `baseSequence`. */
export function hasAssistantReplyAfter(
  items: ReadonlyArray<{ role: string; content?: string; sequence: number }>,
  baseSequence: number,
): boolean {
  return items.some(
    (item) => item.role === 'assistant' && item.sequence > baseSequence && Boolean(item.content?.trim()),
  )
}

/** Places non-persisted status messages right after the newest user message. */
export function mergeStatusMessages<T extends { role: string }>(items: T[], status: T[]): T[] {
  if (status.length === 0) return items
  let at = items.length
  for (let i = items.length - 1; i >= 0; i -= 1) {
    if (items[i].role === 'user') {
      at = i + 1
      break
    }
  }
  return [...items.slice(0, at), ...status, ...items.slice(at)]
}

/**
 * The stop socket reports `stopped` (or an error) only after the stopped turn has
 * released typing, so the playground must stay locked until then: an earlier typing
 * release would otherwise end the next turn.
 */
export function isStopSettled(raw: string): boolean {
  try {
    const data: unknown = JSON.parse(raw)
    return isObj(data) && (data.type === 'stopped' || data.type === 'error')
  } catch {
    return false
  }
}
