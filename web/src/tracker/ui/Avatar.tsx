import { CircleUserRound } from 'lucide-react'
import { initials } from '../format'
import type { User } from '../types'
import { AgentGlyph } from './icons'

export type AvatarSize = 16 | 18 | 20 | 24 | 32

export interface AvatarProps {
  user: User | null | undefined
  size?: AvatarSize
  className?: string
  title?: string
}

export function Avatar({ user, size = 20, className, title }: AvatarProps) {
  if (!user) {
    return (
      <span
        className={['lui-avatar', 'lui-avatar--none', className ?? ''].filter(Boolean).join(' ')}
        style={{ width: size, height: size }}
        title={title ?? 'No assignee'}
        aria-label={title ?? 'No assignee'}
      >
        <CircleUserRound size={size} strokeWidth={1.5} />
      </span>
    )
  }
  const agent = user.kind === 'agent'
  const label = title ?? user.name
  return (
    <span
      className={['lui-avatar', agent ? 'lui-avatar--agent' : '', className ?? ''].filter(Boolean).join(' ')}
      style={{ width: size, height: size, background: user.avatar_color, fontSize: Math.max(8, Math.round(size * 0.42)) }}
      title={label}
      aria-label={label}
      role="img"
    >
      {agent ? <AgentGlyph size={Math.round(size * 0.66)} color="#fff" /> : initials(user.display_name || user.name)}
    </span>
  )
}
