import React, { useEffect, useRef } from 'react'
import { X } from 'lucide-react'
import { IconButton } from './Button'
import { Portal, useDismissableLayer } from './layer'

export interface DialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title?: React.ReactNode
  width?: number | string
  footer?: React.ReactNode
  hideClose?: boolean
  closeOnOutside?: boolean
  className?: string
  children: React.ReactNode
}

export function Dialog(props: DialogProps) {
  if (!props.open) return null
  return <DialogSurface {...props} />
}

const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"]), [contenteditable="true"]'

function DialogSurface({ onOpenChange, title, width = 560, footer, hideClose, closeOnOutside = true, className, children }: DialogProps) {
  const ref = useRef<HTMLDivElement | null>(null)

  useDismissableLayer({ open: true, element: () => ref.current, onDismiss: () => onOpenChange(false), closeOnOutside })

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null
    const el = ref.current
    if (el && !el.contains(document.activeElement)) {
      const auto = el.querySelector<HTMLElement>('[data-autofocus], [autofocus]')
      ;(auto ?? el).focus({ preventScroll: true })
    }
    return () => {
      if (previous && previous.isConnected) previous.focus({ preventScroll: true })
    }
  }, [])

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key !== 'Tab' || !ref.current) return
    const nodes = Array.from(ref.current.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((n) => n.offsetParent !== null)
    if (nodes.length === 0) {
      event.preventDefault()
      return
    }
    const first = nodes[0]
    const last = nodes[nodes.length - 1]
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }

  return (
    <Portal>
      <div className="lui-overlay">
        <div
          ref={ref}
          role="dialog"
          aria-modal="true"
          tabIndex={-1}
          className={className ? `lui-dialog ${className}` : 'lui-dialog'}
          style={{ maxWidth: width }}
          onKeyDown={onKeyDown}
          onClick={(e) => e.stopPropagation()}
        >
          {(title !== undefined || !hideClose) && (
            <div className="lui-dialog-header">
              {title !== undefined ? <h2 className="lui-dialog-title">{title}</h2> : <span />}
              {!hideClose && <IconButton label="Close" size="sm" icon={<X size={14} />} onClick={() => onOpenChange(false)} />}
            </div>
          )}
          <div className="lui-dialog-body">{children}</div>
          {footer && <div className="lui-dialog-footer">{footer}</div>}
        </div>
      </div>
    </Portal>
  )
}
