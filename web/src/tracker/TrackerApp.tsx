import React, { useCallback, useEffect, useMemo, useState } from 'react'
import { PanelLeft } from 'lucide-react'
import './linear.css'
import './shell/shell.css'
import { TrackerProvider, useTracker } from './store'
import { navigate, useRoute, type Route } from './router'
import { Toaster, toast } from './ui'
import { Sidebar, useSidebarWidth } from './shell/Sidebar'
import { HeaderPresenceContext, NavToggleButton, PageHeader, useIsMobile } from './shell/Header'
import { CommandPalette } from './shell/CommandPalette'
import { ShortcutsDialog } from './shell/ShortcutsDialog'
import { GlobalHotkeys } from './shell/GlobalHotkeys'
import { SettingsPage } from './pages/SettingsPage'
import { SearchPage } from './pages/SearchPage'
import { MyIssuesPage, TeamIssuesPage, ViewPage, ViewsPage } from './issues'
import { IssuePage, IssuePeek } from './detail'
import { CreateIssueModal } from './create'
import { ProjectsPage } from './pages/ProjectsPage'
import { ProjectPage } from './pages/ProjectPage'
import { CyclesPage } from './pages/CyclesPage'
import { CyclePage } from './pages/CyclePage'
import { InboxPage } from './pages/InboxPage'
import { SessionsPage } from './pages/SessionsPage'

function routeKey(route: Route): string {
  switch (route.name) {
    case 'team-issues':
      return `team:${route.teamKey}:${route.tab}`
    case 'team-cycles':
      return `cycles:${route.teamKey}`
    case 'issue':
      return `issue:${route.identifier}`
    case 'project':
      return `project:${route.projectId}`
    case 'cycle':
      return `cycle:${route.cycleId}`
    case 'view':
      return `view:${route.viewId}`
    default:
      return route.name
  }
}

function NotFound({ message }: { message: string }) {
  return (
    <>
      <PageHeader crumbs={[{ label: 'Not found' }]} />
      <div className="ls-empty">
        <div>
          <div className="t-1 w-510" style={{ fontSize: 15, marginBottom: 4 }}>
            {message}
          </div>
          <a className="t-3" href="#/tracker/my-issues/assigned">
            Go to My issues
          </a>
        </div>
      </div>
    </>
  )
}

function RouteView({ route }: { route: Route }) {
  const t = useTracker()
  useEffect(() => {
    if (route.name === 'home') navigate('/sessions', { replace: true })
  }, [route.name])

  switch (route.name) {
    case 'home':
      return null
    case 'sessions':
      return <SessionsPage sessionKey={route.sessionKey} />
    case 'inbox':
      return <InboxPage notificationId={route.notificationId ?? undefined} />
    case 'my-issues':
      return <MyIssuesPage tab={route.tab} />
    case 'team-issues':
      return t.teamsByKey.has(route.teamKey) ? (
        <TeamIssuesPage teamKey={route.teamKey} tab={route.tab} />
      ) : (
        <NotFound message={`Team ${route.teamKey} not found`} />
      )
    case 'team-cycles':
      return t.teamsByKey.has(route.teamKey) ? <CyclesPage teamKey={route.teamKey} /> : <NotFound message={`Team ${route.teamKey} not found`} />
    case 'cycle':
      return <CyclePage cycleId={route.cycleId} />
    case 'projects':
      return <ProjectsPage />
    case 'project':
      return <ProjectPage projectId={route.projectId} tab={route.tab} />
    case 'views':
      return <ViewsPage />
    case 'view':
      return <ViewPage viewId={route.viewId} />
    case 'issue':
      return <IssuePage idOrIdentifier={route.identifier} />
    case 'search':
      return <SearchPage q={route.q} />
    case 'settings':
      return <SettingsPage section={route.section} teamKey={route.teamKey} />
  }
}

function LoadingState({ error, onRetry }: { error: string | null; onRetry: () => void }) {
  return (
    <div className="ls-loading">
      {error ? (
        <div style={{ textAlign: 'center' }}>
          <div className="t-1 w-510" style={{ marginBottom: 4 }}>
            Could not load the workspace
          </div>
          <div className="t-3" style={{ marginBottom: 12, maxWidth: 420 }}>
            {error}
          </div>
          <button type="button" className="ls-tab is-active" onClick={onRetry}>
            Try again
          </button>
        </div>
      ) : (
        <div style={{ display: 'grid', gap: 10, width: 240 }} aria-label="Loading" role="status">
          <div className="ls-skel" style={{ width: '70%' }} />
          <div className="ls-skel" />
          <div className="ls-skel" style={{ width: '50%' }} />
        </div>
      )}
    </div>
  )
}

function Shell() {
  const t = useTracker()
  const route = useRoute()
  const mobile = useIsMobile()
  const [width, setWidth] = useSidebarWidth()
  const [headers, setHeaders] = useState(0)
  const registerHeader = useCallback((delta: number) => setHeaders((n) => n + delta), [])
  const { setMobileNavOpen, mobileNavOpen, setPeekIssueId, setSelection } = t
  const key = useMemo(() => routeKey(route), [route])

  useEffect(() => {
    setMobileNavOpen(false)
    setPeekIssueId(null)
    setSelection(new Set())
  }, [key, setMobileNavOpen, setPeekIssueId, setSelection])

  useEffect(() => {
    if (t.lastError) toast(t.lastError.message)
  }, [t.lastError])

  useEffect(() => {
    if (!mobile) setMobileNavOpen(false)
  }, [mobile, setMobileNavOpen])

  useEffect(() => {
    if (!mobileNavOpen) return
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        setMobileNavOpen(false)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [mobileNavOpen, setMobileNavOpen])

  useEffect(() => {
    const title = (() => {
      switch (route.name) {
        case 'sessions':
          return 'Sessions'
        case 'inbox':
          return 'Inbox'
        case 'my-issues':
          return 'My issues'
        case 'team-issues':
          return t.teamsByKey.get(route.teamKey)?.name ?? 'Issues'
        case 'issue': {
          const issue = t.issuesByIdentifier.get(route.identifier)
          return issue ? `${issue.identifier} ${issue.title}` : route.identifier
        }
        case 'projects':
          return 'Projects'
        case 'views':
          return 'Views'
        case 'search':
          return 'Search'
        case 'settings':
          return 'Settings'
        default:
          return 'Issues'
      }
    })()
    document.title = `${title} - Omon`
  }, [route, t.teamsByKey, t.issuesByIdentifier])

  const showDesktopSidebar = !mobile && !t.sidebarCollapsed

  return (
    <div className="ls-root">
      {showDesktopSidebar ? <Sidebar route={route} width={width} onResize={setWidth} /> : null}
      {mobile && mobileNavOpen ? (
        <>
          <div className="ls-scrim" onClick={() => setMobileNavOpen(false)} aria-hidden />
          <div className="ls-drawer" role="dialog" aria-modal="true" aria-label="Navigation">
            <Sidebar route={route} width={Math.min(300, window.innerWidth * 0.86)} onNavigate={() => setMobileNavOpen(false)} />
          </div>
        </>
      ) : null}
      <main className={`ls-main ${showDesktopSidebar ? '' : 'is-full'}`}>
        <div className="ls-panel">
          {!mobile && t.sidebarCollapsed ? (
            <button
              type="button"
              className="ls-icon-btn"
              aria-label="Show sidebar"
              title="Show sidebar ["
              style={{ position: 'absolute', left: 6, bottom: 6, zIndex: 3 }}
              onClick={() => t.setSidebarCollapsed(false)}
            >
              <PanelLeft size={15} />
            </button>
          ) : null}
          {mobile && headers === 0 ? <NavToggleButton className="ls-fab-nav" /> : null}
          <HeaderPresenceContext.Provider value={registerHeader}>
            <div className="ls-page" key={key}>
              {t.ready ? <RouteView route={route} /> : <LoadingState error={t.error} onRetry={() => void t.refresh()} />}
            </div>
          </HeaderPresenceContext.Provider>
        </div>
      </main>
      {t.ready ? (
        <>
          <IssuePeek />
          <CreateIssueModal />
          <CommandPalette />
          <ShortcutsDialog />
          <GlobalHotkeys />
        </>
      ) : null}
      <Toaster />
    </div>
  )
}

function Themed({ children }: { children: React.ReactNode }) {
  const { theme } = useTracker()
  return (
    <div className="lin" data-theme={theme}>
      {children}
    </div>
  )
}

export default function TrackerApp() {
  return (
    <TrackerProvider>
      <Themed>
        <Shell />
      </Themed>
    </TrackerProvider>
  )
}
