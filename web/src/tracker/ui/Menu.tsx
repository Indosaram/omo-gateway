import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { Check, ChevronRight, Search } from 'lucide-react'
import { Checkbox } from './Checkbox'
import { Kbd } from './Kbd'
import { Portal, useDismissableLayer } from './layer'
import type { Side } from './Tooltip'

export type Align = 'start' | 'center' | 'end'
export type Anchor = HTMLElement | DOMRect | null

export interface PopoverProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  anchor: Anchor
  align?: Align
  side?: Side
  width?: number | string
  offset?: number
  className?: string
  children: React.ReactNode
}

const MARGIN = 8

function anchorRect(anchor: Anchor): DOMRect | null {
  if (!anchor) return null
  if (anchor instanceof DOMRect) return anchor
  if (!anchor.isConnected) return null
  return anchor.getBoundingClientRect()
}

function place(rect: DOMRect, size: { width: number; height: number }, side: Side, align: Align, offset: number) {
  const vw = window.innerWidth
  const vh = window.innerHeight
  let resolved = side
  if (side === 'bottom' && rect.bottom + offset + size.height > vh - MARGIN && rect.top - offset - size.height > MARGIN) resolved = 'top'
  else if (side === 'top' && rect.top - offset - size.height < MARGIN && rect.bottom + offset + size.height < vh - MARGIN) resolved = 'bottom'
  else if (side === 'right' && rect.right + offset + size.width > vw - MARGIN && rect.left - offset - size.width > MARGIN) resolved = 'left'
  else if (side === 'left' && rect.left - offset - size.width < MARGIN && rect.right + offset + size.width < vw - MARGIN) resolved = 'right'

  let left: number
  let top: number
  if (resolved === 'bottom' || resolved === 'top') {
    top = resolved === 'bottom' ? rect.bottom + offset : rect.top - offset - size.height
    left = align === 'start' ? rect.left : align === 'end' ? rect.right - size.width : rect.left + rect.width / 2 - size.width / 2
  } else {
    left = resolved === 'right' ? rect.right + offset : rect.left - offset - size.width
    top = align === 'start' ? rect.top : align === 'end' ? rect.bottom - size.height : rect.top + rect.height / 2 - size.height / 2
  }
  left = Math.min(Math.max(MARGIN, left), Math.max(MARGIN, vw - size.width - MARGIN))
  top = Math.min(Math.max(MARGIN, top), Math.max(MARGIN, vh - size.height - MARGIN))
  return { left, top, side: resolved }
}

export function Popover({ open, onOpenChange, anchor, align = 'start', side = 'bottom', width, offset = 4, className, children }: PopoverProps) {
  if (!open) return null
  return (
    <PopoverSurface onOpenChange={onOpenChange} anchor={anchor} align={align} side={side} width={width} offset={offset} className={className}>
      {children}
    </PopoverSurface>
  )
}

function PopoverSurface({
  onOpenChange,
  anchor,
  align,
  side,
  width,
  offset,
  className,
  children,
}: Omit<PopoverProps, 'open'> & { align: Align; side: Side; offset: number }) {
  const ref = useRef<HTMLDivElement | null>(null)
  const [pos, setPos] = useState<{ left: number; top: number; side: Side } | null>(null)
  const close = useCallback(() => onOpenChange(false), [onOpenChange])

  useDismissableLayer({
    open: true,
    element: () => ref.current,
    onDismiss: close,
    ignore: () => [anchor instanceof HTMLElement ? anchor : null],
  })

  const reposition = useCallback(() => {
    const rect = anchorRect(anchor)
    const el = ref.current
    if (!el) return
    if (!rect) {
      setPos({ left: (window.innerWidth - el.offsetWidth) / 2, top: Math.max(MARGIN, window.innerHeight * 0.2), side })
      return
    }
    setPos(place(rect, { width: el.offsetWidth, height: el.offsetHeight }, side, align, offset))
  }, [anchor, side, align, offset])

  useLayoutEffect(() => {
    reposition()
  }, [reposition])

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new ResizeObserver(() => reposition())
    observer.observe(el)
    window.addEventListener('resize', reposition)
    window.addEventListener('scroll', reposition, true)
    return () => {
      observer.disconnect()
      window.removeEventListener('resize', reposition)
      window.removeEventListener('scroll', reposition, true)
    }
  }, [reposition])

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null
    const el = ref.current
    if (el && !el.contains(document.activeElement)) {
      const target = el.querySelector<HTMLElement>('[data-autofocus], input, textarea, [tabindex="-1"], button:not([disabled])')
      ;(target ?? el).focus({ preventScroll: true })
    }
    return () => {
      const active = document.activeElement
      const lostFocus = !active || active === document.body || (el !== null && el.contains(active))
      if (lostFocus && previous && previous.isConnected) previous.focus({ preventScroll: true })
    }
  }, [])

  return (
    <Portal>
      <div
        ref={ref}
        tabIndex={-1}
        className={className ? `lui-popover ${className}` : 'lui-popover'}
        data-side={pos?.side ?? side}
        style={{
          width,
          left: pos ? pos.left : -9999,
          top: pos ? pos.top : -9999,
          visibility: pos ? 'visible' : 'hidden',
        }}
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
        onContextMenu={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key !== 'Escape') e.stopPropagation()
        }}
      >
        {children}
      </div>
    </Portal>
  )
}

export interface MenuItem {
  id: string
  label: string
  icon?: React.ReactNode
  kbd?: string | string[]
  hint?: React.ReactNode
  color?: string
  group?: string
  disabled?: boolean
  danger?: boolean
  keywords?: string
  children?: MenuItem[]
}

export interface MenuListProps {
  items: MenuItem[]
  onSelect: (item: MenuItem) => void
  searchable?: boolean
  placeholder?: string
  multi?: boolean
  selectedIds?: string[]
  emptyText?: string
  numberShortcuts?: boolean
  query?: string
  onQueryChange?: (query: string) => void
  footer?: React.ReactNode
  header?: React.ReactNode
  maxHeight?: number
  onBack?: () => void
}

interface Row {
  item: MenuItem
  parentLabel?: string
}

function matchesQuery(item: MenuItem, q: string, parentLabel?: string): boolean {
  const hay = `${item.label} ${item.keywords ?? ''} ${parentLabel ?? ''}`.toLowerCase()
  return q
    .split(/\s+/)
    .filter(Boolean)
    .every((part) => hay.includes(part))
}

function kbdKeys(kbd: string | string[] | undefined): string[] | null {
  if (kbd === undefined) return null
  return Array.isArray(kbd) ? kbd : [kbd]
}

export function MenuList({
  items,
  onSelect,
  searchable,
  placeholder = 'Search…',
  multi,
  selectedIds,
  emptyText = 'No results',
  numberShortcuts = true,
  query: controlledQuery,
  onQueryChange,
  footer,
  header,
  maxHeight,
  onBack,
}: MenuListProps) {
  const [innerQuery, setInnerQuery] = useState('')
  const query = controlledQuery ?? innerQuery
  const setQuery = useCallback(
    (value: string) => {
      if (onQueryChange) onQueryChange(value)
      if (controlledQuery === undefined) setInnerQuery(value)
    },
    [onQueryChange, controlledQuery],
  )
  const [active, setActive] = useState(0)
  const [submenu, setSubmenu] = useState<{ id: string; element: HTMLElement } | null>(null)
  const rootRef = useRef<HTMLDivElement | null>(null)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const typeahead = useRef<{ text: string; timer?: number }>({ text: '' })
  const selected = useMemo(() => new Set(selectedIds ?? []), [selectedIds])

  const rows: Row[] = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return items.map((item) => ({ item }))
    const out: Row[] = []
    for (const item of items) {
      if (matchesQuery(item, q)) out.push({ item })
      for (const child of item.children ?? []) {
        if (matchesQuery(child, q, item.label)) out.push({ item: child, parentLabel: item.label })
      }
    }
    return out
  }, [items, query])

  const selectable = useMemo(() => rows.map((row, index) => ({ row, index })).filter(({ row }) => !row.item.disabled), [rows])

  const explicitDigits = useMemo(
    () => rows.some(({ item }) => typeof item.kbd === 'string' && /^\d$/.test(item.kbd)),
    [rows],
  )

  const digitFor = useCallback(
    (row: Row, position: number): string | null => {
      if (!numberShortcuts) return null
      if (explicitDigits) return typeof row.item.kbd === 'string' && /^\d$/.test(row.item.kbd) ? row.item.kbd : null
      if (query.trim()) return null
      if (position < 9) return String(position + 1)
      if (position === 9) return '0'
      return null
    },
    [numberShortcuts, explicitDigits, query],
  )

  useEffect(() => {
    setActive(0)
  }, [query])

  useEffect(() => {
    if (active >= selectable.length) setActive(Math.max(0, selectable.length - 1))
  }, [active, selectable.length])

  useEffect(() => {
    const row = selectable[active]
    if (!row || !scrollRef.current) return
    const el = scrollRef.current.querySelector<HTMLElement>(`[data-row-index="${row.index}"]`)
    el?.scrollIntoView({ block: 'nearest' })
  }, [active, selectable])

  useEffect(() => {
    if (searchable) return
    rootRef.current?.focus({ preventScroll: true })
  }, [searchable])

  const rowElement = (index: number) =>
    scrollRef.current?.querySelector<HTMLElement>(`[data-row-index="${index}"]`) ?? null

  const choose = useCallback(
    (row: Row, index: number) => {
      if (row.item.disabled) return
      if (row.item.children && row.item.children.length > 0) {
        const el = rowElement(index)
        if (el) setSubmenu({ id: row.item.id, element: el })
        return
      }
      onSelect(row.item)
    },
    [onSelect],
  )

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (submenu) return
    const count = selectable.length
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        if (count) setActive((a) => (a + 1) % count)
        return
      case 'ArrowUp':
        event.preventDefault()
        if (count) setActive((a) => (a - 1 + count) % count)
        return
      case 'Home':
        if (!searchable) {
          event.preventDefault()
          setActive(0)
        }
        return
      case 'End':
        if (!searchable) {
          event.preventDefault()
          setActive(Math.max(0, count - 1))
        }
        return
      case 'Enter': {
        event.preventDefault()
        const target = selectable[active]
        if (target) choose(target.row, target.index)
        return
      }
      case 'ArrowRight': {
        const target = selectable[active]
        if (target?.row.item.children?.length) {
          event.preventDefault()
          choose(target.row, target.index)
        }
        return
      }
      case 'ArrowLeft':
        if (onBack && (!searchable || query === '')) {
          event.preventDefault()
          onBack()
        }
        return
      case 'Tab':
        event.preventDefault()
        if (count) setActive((a) => (event.shiftKey ? (a - 1 + count) % count : (a + 1) % count))
        return
    }
    if (event.metaKey || event.ctrlKey || event.altKey) return
    if (/^\d$/.test(event.key) && (!searchable || query === '')) {
      const position = selectable.findIndex(({ row }, i) => digitFor(row, i) === event.key)
      if (position >= 0) {
        event.preventDefault()
        const target = selectable[position]
        setActive(position)
        choose(target.row, target.index)
        return
      }
    }
    if (!searchable && event.key.length === 1 && event.key !== ' ') {
      const state = typeahead.current
      window.clearTimeout(state.timer)
      state.text += event.key.toLowerCase()
      state.timer = window.setTimeout(() => {
        state.text = ''
      }, 700)
      const position = selectable.findIndex(({ row }) => row.item.label.toLowerCase().startsWith(state.text))
      if (position >= 0) {
        event.preventDefault()
        setActive(position)
      }
    }
  }

  let lastGroup: string | undefined
  let selectablePosition = -1
  const submenuItem = submenu ? rows.find((r) => r.item.id === submenu.id)?.item : undefined

  return (
    <div
      ref={rootRef}
      className="lui-menu"
      role="menu"
      tabIndex={-1}
      onKeyDown={onKeyDown}
      aria-multiselectable={multi || undefined}
    >
      {header}
      {searchable && (
        <div className="lui-menu-search">
          <Search size={14} color="var(--text-4)" />
          <input
            data-autofocus
            autoFocus
            value={query}
            placeholder={placeholder}
            onChange={(e) => setQuery(e.target.value)}
            aria-label={placeholder}
            spellCheck={false}
            autoComplete="off"
          />
        </div>
      )}
      <div className="lui-menu-scroll" ref={scrollRef} style={maxHeight ? { maxHeight } : undefined}>
        {rows.length === 0 && <div className="lui-menu-empty">{emptyText}</div>}
        {rows.map((row, index) => {
          const { item } = row
          const groupHeader = item.group && item.group !== lastGroup ? item.group : null
          const separator = !groupHeader && lastGroup !== undefined && item.group === undefined && index > 0 && rows[index - 1].item.group !== undefined
          lastGroup = item.group
          if (!item.disabled) selectablePosition += 1
          const position = item.disabled ? -1 : selectablePosition
          const isActive = position >= 0 && position === active
          const isSelected = selected.has(item.id)
          const digit = position >= 0 ? digitFor(row, position) : null
          const keys = kbdKeys(item.kbd)
          const showKbd = keys && !(keys.length === 1 && /^\d$/.test(keys[0]))
          return (
            <React.Fragment key={`${row.parentLabel ?? ''}/${item.id}`}>
              {groupHeader && <div className="lui-menu-group">{groupHeader}</div>}
              {separator && <div className="lui-menu-sep" />}
              <div
                role={multi ? 'menuitemcheckbox' : 'menuitem'}
                aria-checked={multi ? isSelected : undefined}
                aria-disabled={item.disabled || undefined}
                aria-haspopup={item.children?.length ? 'menu' : undefined}
                data-row-index={index}
                data-active={isActive}
                className="lui-menu-item"
                style={item.danger ? { color: 'var(--danger)' } : undefined}
                onMouseMove={() => {
                  if (position >= 0 && position !== active) setActive(position)
                  if (item.children?.length && submenu?.id !== item.id) {
                    const el = rowElement(index)
                    if (el) setSubmenu({ id: item.id, element: el })
                  } else if (!item.children?.length && submenu) {
                    setSubmenu(null)
                  }
                }}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => choose(row, index)}
              >
                {multi && (
                  <Checkbox checked={isSelected} onChange={() => choose(row, index)} label={item.label} />
                )}
                {item.icon !== undefined && <span className="lui-menu-item-icon">{item.icon}</span>}
                <span className="lui-menu-item-label">
                  {row.parentLabel && <span style={{ color: 'var(--text-3)' }}>{row.parentLabel} › </span>}
                  {item.label}
                </span>
                {item.hint !== undefined && <span className="lui-menu-item-hint">{item.hint}</span>}
                {!multi && isSelected && (
                  <span className="lui-menu-item-check">
                    <Check size={14} />
                  </span>
                )}
                {showKbd && keys && <Kbd keys={keys} />}
                {digit && <span className="lui-menu-item-kbd">{digit}</span>}
                {item.children?.length ? <ChevronRight size={14} className="lui-menu-item-chevron" /> : null}
              </div>
            </React.Fragment>
          )
        })}
      </div>
      {footer}
      {submenu && submenuItem?.children && (
        <Popover
          open
          onOpenChange={(open) => {
            if (!open) {
              setSubmenu(null)
              rootRef.current?.focus({ preventScroll: true })
            }
          }}
          anchor={submenu.element}
          side="right"
          align="start"
          offset={6}
          width={220}
        >
          <MenuList
            items={submenuItem.children}
            onSelect={onSelect}
            multi={multi}
            selectedIds={selectedIds}
            numberShortcuts={numberShortcuts}
            searchable={submenuItem.children.length > 8}
            onBack={() => {
              setSubmenu(null)
              rootRef.current?.focus({ preventScroll: true })
            }}
          />
        </Popover>
      )}
    </div>
  )
}

export interface ContextMenuProps {
  items: MenuItem[] | (() => MenuItem[])
  onSelect: (item: MenuItem) => void
  children: React.ReactNode
  disabled?: boolean
  width?: number
  onOpenChange?: (open: boolean) => void
}

export function ContextMenu({ items, onSelect, children, disabled, width = 240, onOpenChange }: ContextMenuProps) {
  const [point, setPoint] = useState<DOMRect | null>(null)
  const resolved = useMemo(() => (point ? (typeof items === 'function' ? items() : items) : []), [items, point])
  const setOpen = (open: boolean) => {
    if (!open) setPoint(null)
    onOpenChange?.(open)
  }
  return (
    <div
      style={{ display: 'contents' }}
      onContextMenu={(event) => {
        if (disabled) return
        event.preventDefault()
        event.stopPropagation()
        setPoint(new DOMRect(event.clientX, event.clientY, 0, 0))
        onOpenChange?.(true)
      }}
    >
      {children}
      <Popover open={point !== null} onOpenChange={setOpen} anchor={point} side="bottom" align="start" offset={0} width={width}>
        <MenuList
          items={resolved}
          numberShortcuts={false}
          onSelect={(item) => {
            setOpen(false)
            onSelect(item)
          }}
        />
      </Popover>
    </div>
  )
}
