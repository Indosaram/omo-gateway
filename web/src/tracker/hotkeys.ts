import { useEffect, useRef } from 'react'

export interface HotkeyOptions {
  enabled?: boolean
  allowInInputs?: boolean
  preventDefault?: boolean
  scope?: string
}

interface Binding {
  combo: ParsedCombo
  handler: (event: KeyboardEvent) => void
  opts: HotkeyOptions
  order: number
}

interface ParsedCombo {
  key: string
  mod: boolean
  shift: boolean
  alt: boolean
}

const bindings = new Set<Binding>()
const sequences = new Set<{ keys: string[]; handler: () => void; opts: HotkeyOptions; order: number }>()
let installed = false
let counter = 0
let buffer: string[] = []
let bufferTimer: number | undefined
const scopeStack: string[] = []

export function pushHotkeyScope(scope: string): () => void {
  scopeStack.push(scope)
  return () => {
    const index = scopeStack.lastIndexOf(scope)
    if (index >= 0) scopeStack.splice(index, 1)
  }
}

export function activeHotkeyScope(): string | null {
  return scopeStack.length ? scopeStack[scopeStack.length - 1] : null
}

function parseCombo(combo: string): ParsedCombo {
  const parts = combo.toLowerCase().split('+').map((p) => p.trim())
  const key = parts[parts.length - 1]
  return {
    key: key === 'space' ? ' ' : key,
    mod: parts.includes('mod') || parts.includes('cmd') || parts.includes('ctrl'),
    shift: parts.includes('shift'),
    alt: parts.includes('alt') || parts.includes('option'),
  }
}

function eventKey(event: KeyboardEvent): string {
  if (event.key === ' ' || event.code === 'Space') return ' '
  if (event.code?.startsWith('Key') && event.code.length === 4) return event.code.slice(3).toLowerCase()
  if (event.code?.startsWith('Digit')) return event.code.slice(5)
  return event.key.toLowerCase()
}

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  const tag = target.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

function matches(combo: ParsedCombo, event: KeyboardEvent): boolean {
  const key = eventKey(event)
  const mod = event.metaKey || event.ctrlKey
  if (combo.key === '?') {
    return event.key === '?' && mod === combo.mod
  }
  return key === combo.key && mod === combo.mod && event.shiftKey === combo.shift && event.altKey === combo.alt
}

function scopeAllows(opts: HotkeyOptions): boolean {
  const active = activeHotkeyScope()
  if (!active) return !opts.scope || opts.scope === 'global'
  return opts.scope === active
}

function onKeyDown(event: KeyboardEvent) {
  if (event.defaultPrevented) return
  const editable = isEditableTarget(event.target)
  const ordered = [...bindings].sort((a, b) => b.order - a.order)
  for (const binding of ordered) {
    if (binding.opts.enabled === false) continue
    if (editable && !binding.opts.allowInInputs) continue
    if (!scopeAllows(binding.opts)) continue
    if (matches(binding.combo, event)) {
      if (binding.opts.preventDefault !== false) event.preventDefault()
      binding.handler(event)
      buffer = []
      return
    }
  }
  if (editable || event.metaKey || event.ctrlKey || event.altKey) return
  const key = eventKey(event)
  if (key.length !== 1) return
  buffer.push(key)
  window.clearTimeout(bufferTimer)
  bufferTimer = window.setTimeout(() => {
    buffer = []
  }, 900)
  const orderedSeq = [...sequences].sort((a, b) => b.order - a.order)
  for (const seq of orderedSeq) {
    if (seq.opts.enabled === false || !scopeAllows(seq.opts)) continue
    const tail = buffer.slice(-seq.keys.length)
    if (tail.length === seq.keys.length && tail.every((k, i) => k === seq.keys[i])) {
      event.preventDefault()
      buffer = []
      seq.handler()
      return
    }
  }
}

function install() {
  if (installed) return
  installed = true
  window.addEventListener('keydown', onKeyDown)
}

export function useHotkey(combo: string | string[], handler: (event: KeyboardEvent) => void, opts: HotkeyOptions = {}) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler
  const combos = Array.isArray(combo) ? combo : [combo]
  const key = combos.join('|')
  const { enabled, allowInInputs, preventDefault, scope } = opts
  useEffect(() => {
    install()
    const created = key.split('|').map((c) => ({
      combo: parseCombo(c),
      handler: (event: KeyboardEvent) => handlerRef.current(event),
      opts: { enabled, allowInInputs, preventDefault, scope },
      order: ++counter,
    }))
    created.forEach((b) => bindings.add(b))
    return () => created.forEach((b) => bindings.delete(b))
  }, [key, enabled, allowInInputs, preventDefault, scope])
}

export function useSequence(keys: string[], handler: () => void, opts: HotkeyOptions = {}) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler
  const signature = keys.join(' ')
  const { enabled, scope } = opts
  useEffect(() => {
    install()
    const entry = {
      keys: signature.split(' '),
      handler: () => handlerRef.current(),
      opts: { enabled, scope },
      order: ++counter,
    }
    sequences.add(entry)
    return () => {
      sequences.delete(entry)
    }
  }, [signature, enabled, scope])
}

export function useHotkeyScope(scope: string, active: boolean) {
  useEffect(() => {
    if (!active) return
    return pushHotkeyScope(scope)
  }, [scope, active])
}
