import React from 'react'
import {
  BookOpen,
  Bot,
  Box,
  Briefcase,
  Code,
  Compass,
  Cpu,
  Database,
  Flag,
  Globe,
  Heart,
  Hexagon,
  Layers,
  Lightbulb,
  Map as MapIcon,
  Package,
  Puzzle,
  Rocket,
  Shield,
  Sparkles,
  Star,
  Target,
  Users,
  Zap,
  type LucideIcon,
} from 'lucide-react'
import type { Priority, StateType } from '../types'

interface SvgBaseProps {
  size?: number
  className?: string
  style?: React.CSSProperties
  title?: string
}

function Svg({ size = 14, className, style, title, children, viewBox = '0 0 14 14' }: SvgBaseProps & { children: React.ReactNode; viewBox?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox={viewBox}
      fill="none"
      className={className}
      style={{ flex: 'none', display: 'block', ...style }}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
    >
      {title && <title>{title}</title>}
      {children}
    </svg>
  )
}

export interface StatusIconProps extends SvgBaseProps {
  type: StateType
  color: string
  progress?: number
}

function piePath(progress: number): string {
  const p = Math.min(0.999, Math.max(0.001, progress))
  const r = 3
  const cx = 7
  const cy = 7
  const angle = p * Math.PI * 2 - Math.PI / 2
  const x = cx + r * Math.cos(angle)
  const y = cy + r * Math.sin(angle)
  const large = p > 0.5 ? 1 : 0
  return `M ${cx} ${cy} L ${cx} ${cy - r} A ${r} ${r} 0 ${large} 1 ${x.toFixed(3)} ${y.toFixed(3)} Z`
}

export function StatusIcon({ type, color, progress, size = 14, className, style, title }: StatusIconProps) {
  switch (type) {
    case 'backlog':
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6" stroke={color} strokeWidth="1.5" strokeDasharray="1.4 1.74" strokeDashoffset="0.65" />
        </Svg>
      )
    case 'unstarted':
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6" stroke={color} strokeWidth="1.5" />
        </Svg>
      )
    case 'started': {
      const value = progress ?? 0.5
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6" stroke={color} strokeWidth="1.5" />
          {value >= 0.999 ? <circle cx="7" cy="7" r="3" fill={color} /> : <path d={piePath(value)} fill={color} />}
        </Svg>
      )
    }
    case 'completed':
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6.25" fill={color} stroke={color} strokeWidth="1.5" />
          <path d="M4.4 7.2 6.2 9 9.7 5.3" stroke="var(--panel, #fff)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </Svg>
      )
    case 'canceled':
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6.25" fill={color} stroke={color} strokeWidth="1.5" />
          <path d="M5 5 9 9M9 5 5 9" stroke="var(--panel, #fff)" strokeWidth="1.5" strokeLinecap="round" />
        </Svg>
      )
    case 'triage':
      return (
        <Svg size={size} className={className} style={style} title={title}>
          <circle cx="7" cy="7" r="6.25" fill={color} stroke={color} strokeWidth="1.5" />
          <path
            d="M3.6 5.6h5.2M7.3 4l1.6 1.6-1.6 1.6M10.4 8.4H5.2M6.7 6.8 5.1 8.4 6.7 10"
            stroke="var(--panel, #fff)"
            strokeWidth="1.2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </Svg>
      )
  }
}

export interface PriorityIconProps extends SvgBaseProps {
  priority: Priority
}

export function PriorityIcon({ priority, size = 14, className, style, title }: PriorityIconProps) {
  if (priority === 0) {
    return (
      <Svg size={size} className={className} style={style} title={title ?? 'No priority'} viewBox="0 0 16 16">
        <rect x="1.5" y="7.25" width="3" height="1.5" rx="0.5" fill="var(--text-3)" />
        <rect x="6.5" y="7.25" width="3" height="1.5" rx="0.5" fill="var(--text-3)" />
        <rect x="11.5" y="7.25" width="3" height="1.5" rx="0.5" fill="var(--text-3)" />
      </Svg>
    )
  }
  if (priority === 1) {
    return (
      <Svg size={size} className={className} style={style} title={title ?? 'Urgent'} viewBox="0 0 16 16">
        <rect x="1" y="1" width="14" height="14" rx="3" fill="var(--urgent)" />
        <path d="M8 4v5" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" />
        <circle cx="8" cy="11.6" r="1.05" fill="#fff" />
      </Svg>
    )
  }
  const filled = priority === 2 ? 3 : priority === 3 ? 2 : 1
  const bars = [
    { x: 1.5, y: 8, h: 6 },
    { x: 6.5, y: 5, h: 9 },
    { x: 11.5, y: 2, h: 12 },
  ]
  const label = priority === 2 ? 'High' : priority === 3 ? 'Medium' : 'Low'
  return (
    <Svg size={size} className={className} style={style} title={title ?? label} viewBox="0 0 16 16">
      {bars.map((bar, i) => (
        <rect
          key={i}
          x={bar.x}
          y={bar.y}
          width="3"
          height={bar.h}
          rx="1"
          fill="var(--text-2)"
          fillOpacity={i < filled ? 1 : 0.28}
        />
      ))}
    </Svg>
  )
}

export function LabelDot({ color, size = 8, className }: { color: string; size?: number; className?: string }) {
  return (
    <span
      aria-hidden
      className={className}
      style={{ display: 'inline-block', flex: 'none', width: size, height: size, borderRadius: 9999, background: color }}
    />
  )
}

const PROJECT_ICONS: Record<string, LucideIcon> = {
  box: Box,
  rocket: Rocket,
  target: Target,
  flag: Flag,
  star: Star,
  zap: Zap,
  layers: Layers,
  code: Code,
  globe: Globe,
  book: BookOpen,
  heart: Heart,
  compass: Compass,
  briefcase: Briefcase,
  map: MapIcon,
  users: Users,
  hexagon: Hexagon,
  cpu: Cpu,
  database: Database,
  shield: Shield,
  sparkles: Sparkles,
  puzzle: Puzzle,
  package: Package,
  lightbulb: Lightbulb,
  bot: Bot,
}

export const PROJECT_ICON_NAMES = Object.keys(PROJECT_ICONS)

export function iconComponent(name: string | null | undefined): LucideIcon {
  return (name && PROJECT_ICONS[name]) || Box
}

export function ProjectIcon({ icon, color, size = 14, className }: { icon: string; color: string; size?: number; className?: string }) {
  const Icon = iconComponent(icon)
  return <Icon size={size} color={color} strokeWidth={2} className={className} style={{ flex: 'none' }} aria-hidden />
}

export function CycleIcon({ progress, size = 14, className, color = 'var(--text-3)' }: { progress: number; size?: number; className?: string; color?: string }) {
  const p = Math.min(1, Math.max(0, progress))
  const r = 5.25
  const circumference = 2 * Math.PI * r
  return (
    <Svg size={size} className={className}>
      <circle cx="7" cy="7" r="6" stroke={color} strokeWidth="1.5" strokeDasharray="2 1.6" opacity={0.6} />
      {p > 0 && (
        <circle
          cx="7"
          cy="7"
          r={r / 2}
          stroke={color}
          strokeWidth={r}
          strokeDasharray={`${(p * Math.PI * r).toFixed(3)} ${circumference.toFixed(3)}`}
          transform="rotate(-90 7 7)"
          fill="none"
        />
      )}
    </Svg>
  )
}

export function EstimateIcon({ size = 14, className, color = 'var(--text-3)' }: { size?: number; className?: string; color?: string }) {
  return (
    <Svg size={size} className={className}>
      <path d="M7 1.8 12.2 11a.6.6 0 0 1-.52.9H2.32a.6.6 0 0 1-.52-.9L7 1.8Z" stroke={color} strokeWidth="1.4" strokeLinejoin="round" />
    </Svg>
  )
}

export function AgentGlyph({ size = 14, className, color = 'currentColor' }: { size?: number; className?: string; color?: string }) {
  return <Bot size={size} color={color} strokeWidth={2} className={className} style={{ flex: 'none' }} aria-hidden />
}

export function TeamIcon({ color, name, size = 16 }: { color: string; name: string; size?: number }) {
  return (
    <span
      aria-hidden
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        flex: 'none',
        width: size,
        height: size,
        borderRadius: 4,
        background: color,
        color: '#fff',
        fontSize: Math.round(size * 0.56),
        fontWeight: 590,
        lineHeight: 1,
      }}
    >
      {name.trim().charAt(0).toUpperCase()}
    </span>
  )
}
