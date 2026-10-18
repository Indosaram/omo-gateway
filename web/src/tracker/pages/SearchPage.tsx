import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Search } from 'lucide-react'
import { trackerApi } from '../api'
import { hrefFor, navigate } from '../router'
import { useTracker } from '../store'
import type { Issue, Project } from '../types'
import { PROJECT_STATUS_LABELS } from '../types'
import { shortDate, stateProgress } from '../format'
import { PageHeader } from '../shell/Header'
import { Avatar, PriorityIcon, ProjectIcon, StatusIcon } from '../ui'
import { fuzzyScore } from '../shell/CommandPalette'

function Highlight({ text, query }: { text: string; query: string }) {
  const q = query.trim()
  if (!q) return <>{text}</>
  const index = text.toLowerCase().indexOf(q.toLowerCase())
  if (index < 0) return <>{text}</>
  return (
    <>
      {text.slice(0, index)}
      <mark className="ls-mark">{text.slice(index, index + q.length)}</mark>
      {text.slice(index + q.length)}
    </>
  )
}

export function SearchPage({ q }: { q: string }) {
  const t = useTracker()
  const [input, setInput] = useState(q)
  const [remote, setRemote] = useState<{ q: string; issues: Issue[]; projects: Project[] } | null>(null)
  const [loading, setLoading] = useState(false)
  const [active, setActive] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => setInput(q), [q])
  useEffect(() => inputRef.current?.focus(), [])

  useEffect(() => {
    const handle = window.setTimeout(() => {
      if (input !== q) navigate(input.trim() ? `/search?q=${encodeURIComponent(input)}` : '/search', { replace: true })
    }, 250)
    return () => window.clearTimeout(handle)
  }, [input, q])

  useEffect(() => {
    const query = q.trim()
    if (!query) {
      setRemote(null)
      return
    }
    let cancelled = false
    setLoading(true)
    trackerApi
      .search(query)
      .then((result) => {
        if (!cancelled) setRemote({ q: query, ...result })
      })
      .catch((err) => {
        if (!cancelled) t.reportError(err)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [q, t.reportError])

  const issues = useMemo(() => {
    const query = input.trim()
    if (!query) return []
    const local = t.issues
      .map((issue) => ({ issue, score: Math.max(fuzzyScore(query, `${issue.identifier} ${issue.title}`), issue.description.toLowerCase().includes(query.toLowerCase()) ? 50 : -1) }))
      .filter((x) => x.score > 0)
    const seen = new Set(local.map((x) => x.issue.id))
    const fromServer = (remote?.issues ?? []).filter((i) => !seen.has(i.id)).map((issue) => ({ issue: t.issuesById.get(issue.id) ?? issue, score: 10 }))
    return [...local, ...fromServer]
      .sort((a, b) => b.score - a.score || b.issue.updated_at.localeCompare(a.issue.updated_at))
      .slice(0, 100)
      .map((x) => x.issue)
  }, [input, t.issues, t.issuesById, remote])

  const projects = useMemo(() => {
    const query = input.trim().toLowerCase()
    if (!query) return []
    const byId = new Map<string, Project>()
    for (const p of t.projects) if (p.name.toLowerCase().includes(query)) byId.set(p.id, p)
    for (const p of remote?.projects ?? []) if (!byId.has(p.id)) byId.set(p.id, t.projectsById.get(p.id) ?? p)
    return [...byId.values()].slice(0, 10)
  }, [input, t.projects, t.projectsById, remote])

  const total = projects.length + issues.length
  useEffect(() => setActive(0), [input])

  const openAt = (index: number) => {
    if (index < projects.length) navigate(`/project/${projects[index].id}/overview`)
    else {
      const issue = issues[index - projects.length]
      if (issue) navigate(`/issue/${issue.identifier}`)
    }
  }

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive((i) => Math.min(total - 1, i + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((i) => Math.max(0, i - 1))
    } else if (event.key === 'Enter' && total > 0) {
      event.preventDefault()
      openAt(active)
    } else if (event.key === 'Escape') {
      ;(event.target as HTMLInputElement).blur()
    }
  }

  return (
    <>
      <PageHeader crumbs={[{ label: 'Search' }]} />
      <div className="ls-search-bar">
        <Search size={16} className="t-3" />
        <input
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Search issues and projects…"
          aria-label="Search"
        />
        {loading ? <span className="t-4" style={{ fontSize: 12 }}>Searching…</span> : null}
        {input.trim() ? <span className="t-3" style={{ fontSize: 12 }}>{total} results</span> : null}
      </div>
      <div className="ls-scroll">
        {!input.trim() ? (
          <div className="ls-empty">
            <div>
              <div className="t-1 w-510" style={{ fontSize: 15, marginBottom: 4 }}>Search your workspace</div>
              <div>Find issues by identifier, title or description, and projects by name.</div>
            </div>
          </div>
        ) : total === 0 && !loading ? (
          <div className="ls-empty">
            <div>
              <div className="t-1 w-510" style={{ fontSize: 15, marginBottom: 4 }}>No results</div>
              <div>Nothing matches “{input.trim()}”.</div>
            </div>
          </div>
        ) : (
          <>
            {projects.length > 0 ? (
              <>
                <div className="ls-palette-group" style={{ padding: '12px 20px 6px' }}>Projects</div>
                {projects.map((project, index) => (
                  <a
                    key={project.id}
                    className={`ls-result ${active === index ? 'is-active' : ''}`}
                    href={hrefFor(`/project/${project.id}/overview`)}
                    onMouseMove={() => setActive(index)}
                  >
                    <ProjectIcon icon={project.icon} color={project.color} />
                    <span className="ls-result-title">
                      <Highlight text={project.name} query={input} />
                    </span>
                    <span className="ls-result-meta">{PROJECT_STATUS_LABELS[project.status]}</span>
                  </a>
                ))}
              </>
            ) : null}
            {issues.length > 0 ? (
              <>
                <div className="ls-palette-group" style={{ padding: '12px 20px 6px' }}>Issues</div>
                {issues.map((issue, i) => {
                  const index = projects.length + i
                  const state = t.statesById.get(issue.state_id)
                  const assignee = issue.assignee_id ? t.usersById.get(issue.assignee_id) ?? null : null
                  return (
                    <a
                      key={issue.id}
                      className={`ls-result ${active === index ? 'is-active' : ''}`}
                      href={hrefFor(`/issue/${issue.identifier}`)}
                      onMouseMove={() => setActive(index)}
                    >
                      <PriorityIcon priority={issue.priority} size={14} />
                      <span className="ls-result-id mono">
                        <Highlight text={issue.identifier} query={input} />
                      </span>
                      {state ? <StatusIcon type={state.type} color={state.color} progress={stateProgress(state, t.teamStates(state.team_id))} size={14} /> : null}
                      <span className="ls-result-title" style={issue.archived_at ? { opacity: 0.6 } : undefined}>
                        <Highlight text={issue.title} query={input} />
                      </span>
                      {issue.archived_at ? <span className="ls-result-meta">Archived</span> : null}
                      <span className="ls-result-meta ls-hide-mobile">{shortDate(issue.updated_at)}</span>
                      <Avatar user={assignee} size={20} />
                    </a>
                  )
                })}
              </>
            ) : null}
          </>
        )}
      </div>
    </>
  )
}

export default SearchPage
