import { navigate, useRoute } from '../router'
import { useHotkey, useSequence } from '../hotkeys'
import { useTracker } from '../store'

export function GlobalHotkeys() {
  const t = useTracker()
  const route = useRoute()
  const overlayOpen = t.paletteOpen || t.shortcutsOpen || t.createDraft !== null

  const routeTeamKey = route.name === 'team-issues' || route.name === 'team-cycles' ? route.teamKey : null
  const fallbackTeam = t.teams.find((team) => !team.system) ?? t.teams[0]
  const teamKey = routeTeamKey ?? fallbackTeam?.key ?? null
  const teamId = routeTeamKey ? t.teamsByKey.get(routeTeamKey)?.id : undefined

  useHotkey('mod+k', () => t.setPaletteOpen(!t.paletteOpen), { allowInInputs: true })
  useHotkey(
    'c',
    () => {
      if (route.name === 'project') t.openCreate({ ...(teamId ? { team_id: teamId } : {}), project_id: route.projectId })
      else if (route.name === 'cycle') {
        const cycle = t.cyclesById.get(route.cycleId)
        t.openCreate(cycle ? { team_id: cycle.team_id, cycle_id: cycle.id } : undefined)
      } else t.openCreate(teamId ? { team_id: teamId } : undefined)
    },
    { enabled: !overlayOpen },
  )
  useHotkey(['?', 'mod+shift+?'], () => t.setShortcutsOpen(!t.shortcutsOpen), { enabled: !t.paletteOpen })
  useHotkey('/', () => navigate('/search'), { enabled: !overlayOpen })
  useHotkey('[', () => t.setSidebarCollapsed(!t.sidebarCollapsed), { enabled: !overlayOpen })
  useHotkey('mod+shift+l', () => t.setTheme(t.theme === 'dark' ? 'light' : 'dark'), { allowInInputs: true })

  const seqOpts = { enabled: !overlayOpen }
  useSequence(['g', 'i'], () => navigate('/inbox'), seqOpts)
  useSequence(['g', 'm'], () => navigate('/my-issues/assigned'), seqOpts)
  useSequence(['g', 'a'], () => teamKey && navigate(`/team/${teamKey}/active`), seqOpts)
  useSequence(['g', 'b'], () => teamKey && navigate(`/team/${teamKey}/backlog`), seqOpts)
  useSequence(['g', 'p'], () => navigate('/projects'), seqOpts)
  useSequence(['g', 'v'], () => teamKey && navigate(`/team/${teamKey}/cycles`), seqOpts)
  useSequence(['g', 's'], () => navigate('/settings/preferences'), seqOpts)
  useSequence(['g', 't'], () => {
    if (route.name === 'issue' || t.teams.length === 0) return
    const index = routeTeamKey ? t.teams.findIndex((team) => team.key === routeTeamKey) : -1
    const next = t.teams[(index + 1) % t.teams.length]
    navigate(`/team/${next.key}/all`)
  }, seqOpts)

  return null
}

export default GlobalHotkeys
