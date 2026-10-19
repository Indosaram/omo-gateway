import { useEffect, useState } from 'react'

export type IssueTab = 'all' | 'active' | 'backlog' | 'triage'
export type MyIssuesTab = 'assigned' | 'created' | 'subscribed' | 'activity'
export type ProjectTab = 'overview' | 'issues' | 'updates'
export type SettingsSection =
  | 'account'
  | 'preferences'
  | 'teams'
  | 'team'
  | 'labels'
  | 'members'
  | 'workflow'

export type Route =
  | { name: 'inbox'; notificationId: string | null }
  | { name: 'my-issues'; tab: MyIssuesTab }
  | { name: 'team-issues'; teamKey: string; tab: IssueTab }
  | { name: 'team-cycles'; teamKey: string }
  | { name: 'cycle'; cycleId: string }
  | { name: 'projects' }
  | { name: 'project'; projectId: string; tab: ProjectTab }
  | { name: 'views' }
  | { name: 'view'; viewId: string }
  | { name: 'issue'; identifier: string }
  | { name: 'search'; q: string }
  | { name: 'settings'; section: SettingsSection; teamKey: string | null }
  | { name: 'sessions'; sessionKey: string | null }
  | { name: 'home' }

const PREFIX = '#/tracker'

export function currentPath(): string {
  const hash = window.location.hash
  if (!hash.startsWith(PREFIX)) return '/'
  const rest = hash.slice(PREFIX.length)
  return rest === '' ? '/' : rest
}

export function parseRoute(path: string): Route {
  const [pathname, query = ''] = path.split('?')
  const params = new URLSearchParams(query)
  const parts = pathname.split('/').filter(Boolean).map(decodeURIComponent)
  const [head, a, b] = parts
  switch (head) {
    case 'sessions':
      return { name: 'sessions', sessionKey: a ?? null }
    case 'inbox':
      return { name: 'inbox', notificationId: a ?? null }
    case 'my-issues': {
      const tab = (['assigned', 'created', 'subscribed', 'activity'] as const).find((t) => t === a) ?? 'assigned'
      return { name: 'my-issues', tab }
    }
    case 'team': {
      if (!a) return { name: 'home' }
      if (b === 'cycles') return { name: 'team-cycles', teamKey: a }
      const tab = (['all', 'active', 'backlog', 'triage'] as const).find((t) => t === b) ?? 'all'
      return { name: 'team-issues', teamKey: a, tab }
    }
    case 'cycle':
      return a ? { name: 'cycle', cycleId: a } : { name: 'home' }
    case 'projects':
      return { name: 'projects' }
    case 'project': {
      if (!a) return { name: 'projects' }
      const tab = (['overview', 'issues', 'updates'] as const).find((t) => t === b) ?? 'overview'
      return { name: 'project', projectId: a, tab }
    }
    case 'views':
      return { name: 'views' }
    case 'view':
      return a ? { name: 'view', viewId: a } : { name: 'views' }
    case 'issue':
      return a ? { name: 'issue', identifier: a } : { name: 'home' }
    case 'search':
      return { name: 'search', q: params.get('q') ?? '' }
    case 'settings': {
      const sections: SettingsSection[] = ['account', 'preferences', 'teams', 'team', 'labels', 'members', 'workflow']
      const section = sections.find((s) => s === a) ?? 'preferences'
      return { name: 'settings', section, teamKey: b ?? null }
    }
    default:
      return { name: 'home' }
  }
}

export function routePath(route: Route): string {
  switch (route.name) {
    case 'inbox':
      return route.notificationId ? `/inbox/${route.notificationId}` : '/inbox'
    case 'my-issues':
      return `/my-issues/${route.tab}`
    case 'team-issues':
      return `/team/${route.teamKey}/${route.tab}`
    case 'team-cycles':
      return `/team/${route.teamKey}/cycles`
    case 'cycle':
      return `/cycle/${route.cycleId}`
    case 'projects':
      return '/projects'
    case 'project':
      return `/project/${route.projectId}/${route.tab}`
    case 'views':
      return '/views'
    case 'view':
      return `/view/${route.viewId}`
    case 'issue':
      return `/issue/${route.identifier}`
    case 'search':
      return route.q ? `/search?q=${encodeURIComponent(route.q)}` : '/search'
    case 'settings':
      return route.teamKey ? `/settings/${route.section}/${route.teamKey}` : `/settings/${route.section}`
    case 'sessions':
      return route.sessionKey ? `/sessions/${encodeURIComponent(route.sessionKey)}` : '/sessions'
    case 'home':
      return '/'
  }
}

export function navigate(target: string | Route, opts: { replace?: boolean } = {}) {
  const path = typeof target === 'string' ? target : routePath(target)
  const hash = `${PREFIX}${path.startsWith('/') ? path : `/${path}`}`
  if (opts.replace) {
    window.history.replaceState(null, '', hash)
    window.dispatchEvent(new HashChangeEvent('hashchange'))
  } else {
    window.location.hash = hash
  }
}

export function hrefFor(target: string | Route): string {
  const path = typeof target === 'string' ? target : routePath(target)
  return `${PREFIX}${path}`
}

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(currentPath()))
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(currentPath()))
    window.addEventListener('hashchange', onChange)
    return () => window.removeEventListener('hashchange', onChange)
  }, [])
  return route
}

export function isTrackerHash(hash = window.location.hash): boolean {
  return hash.startsWith(PREFIX)
}
