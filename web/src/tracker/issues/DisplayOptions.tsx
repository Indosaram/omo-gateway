import { ChartGantt, Columns3, List } from 'lucide-react'
import type { DisplayProperty, Grouping, Ordering, ViewDisplay } from '../types'
import { DEFAULT_DISPLAY } from '../types'
import { Button, Popover, type Anchor } from '../ui'
import type { CollectionLayout } from './model'

const GROUPINGS: { id: Grouping; label: string }[] = [
  { id: 'status', label: 'Status' },
  { id: 'assignee', label: 'Assignee' },
  { id: 'project', label: 'Project' },
  { id: 'priority', label: 'Priority' },
  { id: 'cycle', label: 'Cycle' },
  { id: 'label', label: 'Label' },
  { id: 'team', label: 'Team' },
  { id: 'none', label: 'No grouping' },
]

const ORDERINGS: { id: Ordering; label: string }[] = [
  { id: 'manual', label: 'Manual' },
  { id: 'priority', label: 'Priority' },
  { id: 'status', label: 'Status' },
  { id: 'updated', label: 'Last updated' },
  { id: 'created', label: 'Last created' },
  { id: 'due', label: 'Due date' },
  { id: 'title', label: 'Title' },
]

const COMPLETED: { id: ViewDisplay['show_completed']; label: string }[] = [
  { id: 'all', label: 'All' },
  { id: 'month', label: 'Past month' },
  { id: 'week', label: 'Past week' },
  { id: 'none', label: 'None' },
]

const PROPERTIES: { id: DisplayProperty; label: string }[] = [
  { id: 'id', label: 'ID' },
  { id: 'status', label: 'Status' },
  { id: 'assignee', label: 'Assignee' },
  { id: 'priority', label: 'Priority' },
  { id: 'project', label: 'Project' },
  { id: 'cycle', label: 'Cycle' },
  { id: 'labels', label: 'Labels' },
  { id: 'estimate', label: 'Estimate' },
  { id: 'due', label: 'Due date' },
  { id: 'sub_issues', label: 'Sub-issues' },
  { id: 'lane', label: 'Agent lane' },
  { id: 'created', label: 'Created' },
  { id: 'updated', label: 'Updated' },
]

function Select<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T
  options: { id: T; label: string }[]
  onChange: (value: T) => void
  label: string
}) {
  return (
    <select className="lic-select" aria-label={label} value={value} onChange={(e) => onChange(e.target.value as T)}>
      {options.map((o) => (
        <option key={o.id} value={o.id}>
          {o.label}
        </option>
      ))}
    </select>
  )
}

function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`lic-switch${checked ? ' lic-switch--on' : ''}`}
      onClick={() => onChange(!checked)}
    >
      <span className="lic-switch-knob" />
    </button>
  )
}

export function DisplayOptions({
  open,
  onOpenChange,
  anchor,
  display,
  layout,
  onLayout,
  onChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  anchor: Anchor
  display: ViewDisplay
  layout: CollectionLayout
  onLayout: (layout: CollectionLayout) => void
  onChange: (display: ViewDisplay) => void
}) {
  const set = <K extends keyof ViewDisplay>(key: K, value: ViewDisplay[K]) => onChange({ ...display, [key]: value })
  const toggleProperty = (p: DisplayProperty) =>
    set('properties', display.properties.includes(p) ? display.properties.filter((x) => x !== p) : [...display.properties, p])
  return (
    <Popover open={open} onOpenChange={onOpenChange} anchor={anchor} align="end" width={340}>
      <div className="lic-display" data-display-options>
        <div className="lic-layout-tabs" role="tablist">
          {(
            [
              { id: 'list', label: 'List', icon: <List size={16} /> },
              { id: 'board', label: 'Board', icon: <Columns3 size={16} /> },
              { id: 'timeline', label: 'Timeline', icon: <ChartGantt size={16} /> },
            ] as const
          ).map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={layout === tab.id}
              className={`lic-layout-tab${layout === tab.id ? ' lic-layout-tab--on' : ''}`}
              onClick={() => onLayout(tab.id)}
            >
              {tab.icon}
              {tab.label}
            </button>
          ))}
        </div>
        <div className="lic-display-sec">
          <div className="lic-display-row">
            <span className="t-2">{layout === 'board' ? 'Columns' : 'Grouping'}</span>
            <Select
              label="Grouping"
              value={display.grouping}
              options={layout === 'board' ? GROUPINGS.filter((g) => g.id !== 'none') : GROUPINGS}
              onChange={(v) => set('grouping', v)}
            />
          </div>
          {layout === 'list' && (
            <div className="lic-display-row">
              <span className="t-2">Sub-grouping</span>
              <Select
                label="Sub-grouping"
                value={display.sub_grouping}
                options={GROUPINGS.filter((g) => g.id === 'none' || g.id !== display.grouping)}
                onChange={(v) => set('sub_grouping', v)}
              />
            </div>
          )}
          <div className="lic-display-row">
            <span className="t-2">Ordering</span>
            <Select label="Ordering" value={display.ordering} options={ORDERINGS} onChange={(v) => set('ordering', v)} />
          </div>
        </div>
        <div className="lic-display-sec">
          <div className="lic-display-row">
            <span className="t-2">Completed issues</span>
            <Select label="Completed issues" value={display.show_completed} options={COMPLETED} onChange={(v) => set('show_completed', v)} />
          </div>
          <div className="lic-display-row">
            <span className="t-2">Show sub-issues</span>
            <Toggle label="Show sub-issues" checked={display.show_sub_issues} onChange={(v) => set('show_sub_issues', v)} />
          </div>
          <div className="lic-display-row">
            <span className="t-2">Show empty groups</span>
            <Toggle label="Show empty groups" checked={display.show_empty_groups} onChange={(v) => set('show_empty_groups', v)} />
          </div>
        </div>
        <div className="lic-display-sec">
          <div className="t-3 lic-display-label">Display properties</div>
          <div className="lic-prop-pills">
            {PROPERTIES.map((p) => (
              <button
                key={p.id}
                type="button"
                aria-pressed={display.properties.includes(p.id)}
                className={`lic-prop-pill${display.properties.includes(p.id) ? ' lic-prop-pill--on' : ''}`}
                onClick={() => toggleProperty(p.id)}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>
        <div className="lic-display-foot">
          <Button variant="ghost" size="sm" onClick={() => onChange({ ...DEFAULT_DISPLAY, layout: display.layout })}>
            Reset to default
          </Button>
        </div>
      </div>
    </Popover>
  )
}
