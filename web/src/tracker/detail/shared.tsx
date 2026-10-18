import { useEffect, useLayoutEffect, useState } from 'react'
import type { RefObject } from 'react'
import ReactMarkdown from 'react-markdown'
import { TrackerApiError } from '../api'
import type { TrackerState } from '../store'
import { cycleLabel } from '../format'
import { toast } from '../ui'
import type { MenuItem } from '../ui'

const MOBILE_QUERY = '(max-width: 767px)'

export function useIsMobile(): boolean {
  const [mobile, setMobile] = useState(() => typeof window !== 'undefined' && window.matchMedia(MOBILE_QUERY).matches)
  useEffect(() => {
    const mq = window.matchMedia(MOBILE_QUERY)
    const onChange = () => setMobile(mq.matches)
    onChange()
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [])
  return mobile
}

export function useAutosize(ref: RefObject<HTMLTextAreaElement>, value: string, minHeight = 0) {
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    el.style.height = '0px'
    el.style.height = `${Math.max(minHeight, el.scrollHeight)}px`
  }, [ref, value, minHeight])
}

export function errorMessage(err: unknown): string {
  return err instanceof TrackerApiError || err instanceof Error ? err.message : String(err)
}

export function notifyError(err: unknown) {
  toast(errorMessage(err))
}

export async function copyText(text: string, confirmation: string) {
  try {
    await navigator.clipboard.writeText(text)
    toast(confirmation)
  } catch (err) {
    notifyError(err)
  }
}

// FE-A's MenuList contract does not pin whether onSelect receives the item or its id; accept both.
export function menuId(arg: MenuItem | string): string {
  return typeof arg === 'string' ? arg : arg.id
}

export const REACTION_EMOJIS = ['👍', '👎', '❤️', '😄', '🎉', '😕', '🚀', '👀']

export function Markdown({ source, className = '' }: { source: string; className?: string }) {
  return (
    <div className={`lin-markdown ${className}`}>
      <ReactMarkdown
        components={{
          a: ({ node: _node, ...props }) => <a {...props} target="_blank" rel="noreferrer" />,
        }}
      >
        {source}
      </ReactMarkdown>
    </div>
  )
}

export function resolveValue(value: string | null, t: TrackerState): string {
  if (value === null || value === '') return 'none'
  const state = t.statesById.get(value)
  if (state) return state.name
  const user = t.usersById.get(value)
  if (user) return user.name
  const label = t.labelsById.get(value)
  if (label) return label.name
  const project = t.projectsById.get(value)
  if (project) return project.name
  const cycle = t.cyclesById.get(value)
  if (cycle) return cycleLabel(cycle)
  const issue = t.issuesById.get(value)
  if (issue) return issue.identifier
  const milestone = t.milestones.find((m) => m.id === value)
  if (milestone) return milestone.name
  return value
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host
  } catch {
    return url
  }
}
