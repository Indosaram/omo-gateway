import React, { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'

export function portalRoot(): HTMLElement {
  return (document.querySelector('.lin') as HTMLElement | null) ?? document.body
}

export function Portal({ children }: { children: React.ReactNode }) {
  return createPortal(children, portalRoot())
}

interface Layer {
  id: number
  element: () => HTMLElement | null
}

const stack: Layer[] = []
let seq = 0

function indexOf(id: number): number {
  return stack.findIndex((layer) => layer.id === id)
}

function insideLayerAtOrAbove(id: number, target: Node): boolean {
  const start = indexOf(id)
  if (start < 0) return false
  for (let i = start; i < stack.length; i++) {
    const el = stack[i].element()
    if (el && el.contains(target)) return true
  }
  return false
}

/**
 * Registers a dismissable layer (popover, dialog). Only the top-most layer reacts to Escape, and a
 * pointer-down counts as "outside" only when it lands outside this layer and every layer stacked above it.
 */
export function useDismissableLayer(opts: {
  open: boolean
  element: () => HTMLElement | null
  onDismiss: () => void
  ignore?: () => (Node | null)[]
  closeOnOutside?: boolean
}) {
  const { open, closeOnOutside = true } = opts
  const ref = useRef(opts)
  ref.current = opts

  useEffect(() => {
    if (!open) return
    const id = ++seq
    stack.push({ id, element: () => ref.current.element() })

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      if (stack[stack.length - 1]?.id !== id) return
      event.preventDefault()
      event.stopPropagation()
      ref.current.onDismiss()
    }
    const onPointerDown = (event: PointerEvent) => {
      if (!closeOnOutside) return
      const target = event.target as Node | null
      if (!target) return
      if (insideLayerAtOrAbove(id, target)) return
      const ignored = ref.current.ignore?.() ?? []
      if (ignored.some((node) => node && node.contains(target))) return
      ref.current.onDismiss()
    }

    document.addEventListener('keydown', onKeyDown, true)
    document.addEventListener('pointerdown', onPointerDown, true)
    return () => {
      document.removeEventListener('keydown', onKeyDown, true)
      document.removeEventListener('pointerdown', onPointerDown, true)
      const index = indexOf(id)
      if (index >= 0) stack.splice(index, 1)
    }
  }, [open, closeOnOutside])
}

export const stopPropagationHandlers = {
  onClick: (e: React.SyntheticEvent) => e.stopPropagation(),
  onDoubleClick: (e: React.SyntheticEvent) => e.stopPropagation(),
  onMouseDown: (e: React.SyntheticEvent) => e.stopPropagation(),
  onPointerDown: (e: React.SyntheticEvent) => e.stopPropagation(),
  onContextMenu: (e: React.SyntheticEvent) => e.stopPropagation(),
  onKeyDown: (e: React.SyntheticEvent) => e.stopPropagation(),
}
