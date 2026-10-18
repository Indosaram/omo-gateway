import React, { cloneElement, isValidElement, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { Kbd } from './Kbd'
import { Portal } from './layer'

export type Side = 'top' | 'bottom' | 'left' | 'right'

interface TooltipProps {
  content: React.ReactNode
  kbd?: string[]
  side?: Side
  delay?: number
  disabled?: boolean
  children: React.ReactNode
}

const GAP = 6

export function Tooltip({ content, kbd, side = 'bottom', delay = 500, disabled, children }: TooltipProps) {
  const [open, setOpen] = useState(false)
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null)
  const triggerRef = useRef<HTMLElement | null>(null)
  const tipRef = useRef<HTMLDivElement | null>(null)
  const timer = useRef<number | undefined>(undefined)

  const show = useCallback(() => {
    if (disabled) return
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => setOpen(true), delay)
  }, [delay, disabled])
  const hide = useCallback(() => {
    window.clearTimeout(timer.current)
    setOpen(false)
    setPos(null)
  }, [])

  useEffect(() => () => window.clearTimeout(timer.current), [])
  useEffect(() => {
    if (disabled) hide()
  }, [disabled, hide])

  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !tipRef.current) return
    const r = triggerRef.current.getBoundingClientRect()
    const t = tipRef.current.getBoundingClientRect()
    let left = r.left + r.width / 2 - t.width / 2
    let top = r.bottom + GAP
    if (side === 'top') top = r.top - t.height - GAP
    if (side === 'left') {
      left = r.left - t.width - GAP
      top = r.top + r.height / 2 - t.height / 2
    }
    if (side === 'right') {
      left = r.right + GAP
      top = r.top + r.height / 2 - t.height / 2
    }
    if (side === 'bottom' && top + t.height > window.innerHeight - 4) top = r.top - t.height - GAP
    if (side === 'top' && top < 4) top = r.bottom + GAP
    left = Math.min(Math.max(4, left), window.innerWidth - t.width - 4)
    top = Math.min(Math.max(4, top), window.innerHeight - t.height - 4)
    setPos({ left, top })
  }, [open, side])

  const hasContent = content !== null && content !== undefined && content !== ''
  const tip =
    open && hasContent ? (
      <Portal>
        <div
          ref={tipRef}
          role="tooltip"
          className="lui-tooltip"
          style={pos ? { left: pos.left, top: pos.top } : { left: -9999, top: -9999 }}
        >
          <span>{content}</span>
          {kbd && kbd.length > 0 && <Kbd keys={kbd} />}
        </div>
      </Portal>
    ) : null

  const handlers = {
    onMouseEnter: show,
    onMouseLeave: hide,
    onFocus: show,
    onBlur: hide,
    onPointerDown: hide,
  }

  if (isValidElement(children) && typeof children.type === 'string') {
    const child = children as React.ReactElement<Record<string, unknown>>
    const props = child.props as Record<string, unknown>
    const originalRef = (child as unknown as { ref?: React.Ref<HTMLElement> }).ref
    const chain = (name: keyof typeof handlers) => (event: unknown) => {
      const original = props[name] as ((e: unknown) => void) | undefined
      original?.(event)
      handlers[name]()
    }
    return (
      <>
        {cloneElement(child, {
          ref: (node: HTMLElement | null) => {
            triggerRef.current = node
            if (typeof originalRef === 'function') originalRef(node)
            else if (originalRef && typeof originalRef === 'object') {
              ;(originalRef as React.MutableRefObject<HTMLElement | null>).current = node
            }
          },
          onMouseEnter: chain('onMouseEnter'),
          onMouseLeave: chain('onMouseLeave'),
          onFocus: chain('onFocus'),
          onBlur: chain('onBlur'),
          onPointerDown: chain('onPointerDown'),
        })}
        {tip}
      </>
    )
  }

  return (
    <span className="lui-tooltip-trigger" ref={(node) => {
        triggerRef.current = node
      }} {...handlers}>
      {children}
      {tip}
    </span>
  )
}
