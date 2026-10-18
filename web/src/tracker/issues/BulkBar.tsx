import { Box, CircleUser, Hexagon, Signal, Tag, Trash2, X } from 'lucide-react'
import { useState } from 'react'
import type { TrackerState } from '../store'
import type { ID } from '../types'
import {
  AssigneePicker,
  Button,
  CycleIcon,
  CyclePicker,
  Dialog,
  LabelPicker,
  PriorityPicker,
  ProjectPicker,
  StatusPicker,
  toast,
} from '../ui'
import { errorMessage, type PickerKind } from './model'

type BulkPicker = Extract<PickerKind, 'status' | 'priority' | 'assignee' | 'labels' | 'project' | 'cycle'>

const ACTIONS: { kind: BulkPicker; label: string; icon: React.ReactNode }[] = [
  { kind: 'status', label: 'Status', icon: <Hexagon size={14} /> },
  { kind: 'priority', label: 'Priority', icon: <Signal size={14} /> },
  { kind: 'assignee', label: 'Assignee', icon: <CircleUser size={14} /> },
  { kind: 'labels', label: 'Labels', icon: <Tag size={14} /> },
  { kind: 'project', label: 'Project', icon: <Box size={14} /> },
  { kind: 'cycle', label: 'Cycle', icon: <CycleIcon progress={0.5} /> },
]

export function BulkBar({ ids, t, onClear }: { ids: ID[]; t: TrackerState; onClear: () => void }) {
  const [picker, setPicker] = useState<{ kind: BulkPicker; anchor: HTMLElement } | null>(null)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [deleting, setDeleting] = useState(false)
  if (ids.length === 0) return null

  const pickerProps = (kind: BulkPicker) => ({
    issueIds: ids,
    anchor: picker?.kind === kind ? picker.anchor : null,
    open: picker?.kind === kind,
    onOpenChange: (open: boolean) => {
      if (!open) setPicker(null)
    },
  })

  const runDelete = async () => {
    setDeleting(true)
    let failed = 0
    let lastError = ''
    for (const id of ids) {
      try {
        await t.deleteIssue(id)
      } catch (err) {
        failed += 1
        lastError = errorMessage(err)
      }
    }
    setDeleting(false)
    setConfirmDelete(false)
    if (failed > 0) toast.error(`${failed} of ${ids.length} issues could not be deleted: ${lastError}`)
    else toast(`${ids.length} ${ids.length === 1 ? 'issue' : 'issues'} deleted`)
    onClear()
  }

  return (
    <>
      <div className="lic-bulk anim-toast" role="toolbar" aria-label="Bulk actions" data-bulk-bar>
        <span className="lic-bulk-count">
          <span className="w-510 tabular" data-bulk-count>
            {ids.length} selected
          </span>
          <button type="button" className="lic-bulk-clear" aria-label="Clear selection" title="Clear selection" onClick={onClear}>
            <X size={13} />
          </button>
        </span>
        <span className="lic-bulk-sep" />
        {ACTIONS.map((action) => (
          <button
            key={action.kind}
            type="button"
            className="lic-bulk-btn"
            data-bulk-action={action.kind}
            onClick={(e) => setPicker({ kind: action.kind, anchor: e.currentTarget })}
          >
            {action.icon}
            <span className="lic-bulk-label">{action.label}</span>
          </button>
        ))}
        <span className="lic-bulk-sep" />
        <button type="button" className="lic-bulk-btn lic-bulk-btn--danger" data-bulk-action="delete" onClick={() => setConfirmDelete(true)}>
          <Trash2 size={14} />
          <span className="lic-bulk-label">Delete</span>
        </button>
      </div>
      <StatusPicker {...pickerProps('status')} />
      <PriorityPicker {...pickerProps('priority')} />
      <AssigneePicker {...pickerProps('assignee')} />
      <LabelPicker {...pickerProps('labels')} />
      <ProjectPicker {...pickerProps('project')} />
      <CyclePicker {...pickerProps('cycle')} />
      <Dialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Delete ${ids.length} ${ids.length === 1 ? 'issue' : 'issues'}?`}
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button variant="danger" disabled={deleting} onClick={() => void runDelete()}>
              Delete
            </Button>
          </>
        }
      >
        <p className="t-2" style={{ margin: 0 }}>
          This permanently deletes the selected issues. Issues linked to an agent lane cannot be deleted.
        </p>
      </Dialog>
    </>
  )
}
