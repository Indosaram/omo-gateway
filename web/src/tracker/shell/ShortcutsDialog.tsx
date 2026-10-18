import { Dialog, Kbd } from '../ui'
import { useTracker } from '../store'

interface Shortcut {
  label: string
  keys: string[][]
}

const GROUPS: { title: string; items: Shortcut[] }[] = [
  {
    title: 'General',
    items: [
      { label: 'Open command menu', keys: [['⌘', 'K']] },
      { label: 'Create new issue', keys: [['C']] },
      { label: 'Search', keys: [['/']] },
      { label: 'Keyboard shortcuts', keys: [['?'], ['⌘', '⇧', '?']] },
      { label: 'Toggle sidebar', keys: [['[']] },
      { label: 'Toggle light / dark theme', keys: [['⌘', '⇧', 'L']] },
    ],
  },
  {
    title: 'Navigation',
    items: [
      { label: 'Go to Inbox', keys: [['G', 'I']] },
      { label: 'Go to My issues', keys: [['G', 'M']] },
      { label: 'Go to active issues', keys: [['G', 'A']] },
      { label: 'Go to backlog', keys: [['G', 'B']] },
      { label: 'Go to Projects', keys: [['G', 'P']] },
      { label: 'Go to Cycles', keys: [['G', 'V']] },
      { label: 'Go to Settings', keys: [['G', 'S']] },
    ],
  },
  {
    title: 'Issue list',
    items: [
      { label: 'Move down / up', keys: [['J'], ['K']] },
      { label: 'Select issue', keys: [['X']] },
      { label: 'Select range', keys: [['⇧', 'X']] },
      { label: 'Select all', keys: [['⌘', 'A']] },
      { label: 'Open issue', keys: [['Enter'], ['O']] },
      { label: 'Peek issue', keys: [['Space']] },
      { label: 'Toggle list / board', keys: [['⌘', 'B']] },
      { label: 'Filter', keys: [['F']] },
      { label: 'Close / clear selection', keys: [['Esc']] },
    ],
  },
  {
    title: 'Issue actions',
    items: [
      { label: 'Change status', keys: [['S']] },
      { label: 'Change priority', keys: [['P']] },
      { label: 'Assign to', keys: [['A']] },
      { label: 'Assign to me', keys: [['I']] },
      { label: 'Change labels', keys: [['L']] },
      { label: 'Move to project', keys: [['⇧', 'P']] },
      { label: 'Move to cycle', keys: [['⇧', 'C']] },
      { label: 'Set estimate', keys: [['⇧', 'E']] },
      { label: 'Set due date', keys: [['⇧', 'D']] },
      { label: 'Edit title', keys: [['E']] },
      { label: 'Add comment', keys: [['M']] },
      { label: 'Copy issue ID', keys: [['⌘', '.']] },
      { label: 'Copy git branch name', keys: [['⌘', '⇧', ',']] },
      { label: 'Archive', keys: [['⌘', '⇧', 'A']] },
      { label: 'Delete', keys: [['⌘', '⌫']] },
    ],
  },
]

export function ShortcutsDialog() {
  const { shortcutsOpen, setShortcutsOpen } = useTracker()
  return (
    <Dialog open={shortcutsOpen} onOpenChange={setShortcutsOpen} title="Keyboard shortcuts" width={560}>
      <div style={{ maxHeight: '64vh', overflowY: 'auto', paddingRight: 4 }}>
        {GROUPS.map((group) => (
          <section key={group.title} className="ls-sc-group">
            <h3 className="ls-sc-title">{group.title}</h3>
            {group.items.map((item) => (
              <div key={item.label} className="ls-sc-row">
                <span>{item.label}</span>
                <span className="ls-sc-keys">
                  {item.keys.map((combo, index) => (
                    <span key={index} style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
                      {index > 0 ? <span>or</span> : null}
                      {combo.length === 2 && combo[0] === 'G' ? (
                        <>
                          <Kbd keys={[combo[0]]} />
                          <span>then</span>
                          <Kbd keys={[combo[1]]} />
                        </>
                      ) : (
                        <Kbd keys={combo} />
                      )}
                    </span>
                  ))}
                </span>
              </div>
            ))}
          </section>
        ))}
      </div>
    </Dialog>
  )
}

export default ShortcutsDialog
