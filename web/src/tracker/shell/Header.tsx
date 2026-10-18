import React, { createContext, useContext, useEffect, useState } from 'react'
import { ChevronRight, Menu } from 'lucide-react'
import { hrefFor } from '../router'
import { useTracker } from '../store'

export interface Crumb {
  label: React.ReactNode
  href?: string
  icon?: React.ReactNode
  muted?: boolean
}

export interface HeaderTab {
  id: string
  label: React.ReactNode
  href?: string
  onClick?: () => void
  active?: boolean
  icon?: React.ReactNode
  count?: number
}

export interface PageHeaderProps {
  title?: React.ReactNode
  icon?: React.ReactNode
  crumbs?: Crumb[]
  breadcrumbs?: Crumb[]
  actions?: React.ReactNode
  right?: React.ReactNode
  tabs?: HeaderTab[]
  activeTab?: string
  tabsRight?: React.ReactNode
  children?: React.ReactNode
}

const MOBILE_QUERY = '(max-width: 767px)'

export function useIsMobile(): boolean {
  const [mobile, setMobile] = useState(() => typeof window !== 'undefined' && window.matchMedia(MOBILE_QUERY).matches)
  useEffect(() => {
    const mql = window.matchMedia(MOBILE_QUERY)
    const onChange = () => setMobile(mql.matches)
    onChange()
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [])
  return mobile
}

export const HeaderPresenceContext = createContext<((delta: number) => void) | null>(null)

export function NavToggleButton({ className }: { className?: string }) {
  const { mobileNavOpen, setMobileNavOpen } = useTracker()
  return (
    <button
      type="button"
      className={`ls-icon-btn ${className ?? ''}`}
      aria-label="Open navigation"
      aria-expanded={mobileNavOpen}
      onClick={() => setMobileNavOpen(!mobileNavOpen)}
    >
      <Menu size={16} />
    </button>
  )
}

export function PageHeader(props: PageHeaderProps) {
  const register = useContext(HeaderPresenceContext)
  const mobile = useIsMobile()
  useEffect(() => {
    if (!register) return
    register(1)
    return () => register(-1)
  }, [register])

  const crumbs = props.crumbs ?? props.breadcrumbs ?? []
  const actions = props.actions ?? props.right ?? props.children

  return (
    <>
      <header className="ls-header">
        {mobile ? <NavToggleButton /> : null}
        <nav className="ls-crumbs" aria-label="Breadcrumb">
          {crumbs.map((crumb, index) => (
            <React.Fragment key={index}>
              {index > 0 ? <ChevronRight size={13} className="ls-crumb-sep" /> : null}
              {crumb.href ? (
                <a className={`ls-crumb ${crumb.muted ? 'is-muted' : ''}`} href={crumb.href.startsWith('#') ? crumb.href : hrefFor(crumb.href)}>
                  {crumb.icon}
                  {crumb.label}
                </a>
              ) : (
                <span className={`ls-crumb ${crumb.muted ? 'is-muted' : ''}`}>
                  {crumb.icon}
                  {crumb.label}
                </span>
              )}
            </React.Fragment>
          ))}
          {props.title ? (
            <>
              {crumbs.length > 0 ? <ChevronRight size={13} className="ls-crumb-sep" /> : null}
              <span className="ls-crumb">
                {props.icon}
                {props.title}
              </span>
            </>
          ) : null}
        </nav>
        {actions ? <div className="ls-header-actions">{actions}</div> : null}
      </header>
      {props.tabs && props.tabs.length > 0 ? (
        <div className="ls-tabs" role="tablist">
          {props.tabs.map((tab) => {
            const active = tab.active ?? tab.id === props.activeTab
            const content = (
              <>
                {tab.icon}
                {tab.label}
                {tab.count !== undefined ? <span className="ls-count">{tab.count}</span> : null}
              </>
            )
            return tab.href ? (
              <a
                key={tab.id}
                role="tab"
                aria-selected={active}
                className={`ls-tab ${active ? 'is-active' : ''}`}
                href={tab.href.startsWith('#') ? tab.href : hrefFor(tab.href)}
              >
                {content}
              </a>
            ) : (
              <button key={tab.id} type="button" role="tab" aria-selected={active} className={`ls-tab ${active ? 'is-active' : ''}`} onClick={tab.onClick}>
                {content}
              </button>
            )
          })}
          {props.tabsRight ? <div style={{ marginLeft: 'auto', display: 'flex', gap: 4 }}>{props.tabsRight}</div> : null}
        </div>
      ) : null}
    </>
  )
}

export default PageHeader
