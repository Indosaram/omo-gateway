import { isMac } from '../format'

const GLYPHS: Record<string, string> = {
  mod: '⌘',
  cmd: '⌘',
  meta: '⌘',
  shift: '⇧',
  alt: '⌥',
  option: '⌥',
  ctrl: '⌃',
  enter: '↵',
  return: '↵',
  backspace: '⌫',
  delete: '⌫',
  esc: 'Esc',
  escape: 'Esc',
  tab: 'Tab',
  space: 'Space',
  up: '↑',
  down: '↓',
  left: '←',
  right: '→',
}

export function keyGlyph(key: string): string {
  const lower = key.toLowerCase()
  if ((lower === 'mod' || lower === 'cmd' || lower === 'meta') && !isMac()) return 'Ctrl'
  if (lower === 'alt' && !isMac()) return 'Alt'
  const glyph = GLYPHS[lower]
  if (glyph) return glyph
  return key.length === 1 ? key.toUpperCase() : key
}

export function Kbd({ keys, className }: { keys: string[]; className?: string }) {
  return (
    <span className={className ? `lui-kbd ${className}` : 'lui-kbd'}>
      {keys.map((key, index) =>
        key === 'then' ? (
          <span key={index} className="lui-kbd-then">
            then
          </span>
        ) : (
          <kbd key={index} className="lui-kbd-key">
            {keyGlyph(key)}
          </kbd>
        ),
      )}
    </span>
  )
}
