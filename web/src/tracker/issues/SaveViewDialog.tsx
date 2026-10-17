import { useState } from 'react'
import { trackerApi } from '../api'
import { navigate } from '../router'
import type { TrackerState } from '../store'
import type { ID, ViewDisplay, ViewFilters } from '../types'
import { Button, Dialog, toast } from '../ui'
import { errorMessage } from './model'

export function SaveViewDialog({
  open,
  onOpenChange,
  filters,
  display,
  teamId,
  t,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  filters: ViewFilters
  display: ViewDisplay
  teamId: ID | null
  t: TrackerState
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [saving, setSaving] = useState(false)

  const save = async () => {
    const trimmed = name.trim()
    if (!trimmed || saving) return
    setSaving(true)
    try {
      const view = await trackerApi.createView({
        name: trimmed,
        description: description.trim() || null,
        filters,
        display,
        team_id: teamId,
      })
      t.upsertView(view)
      onOpenChange(false)
      setName('')
      setDescription('')
      toast.success(`View "${view.name}" saved`)
      navigate({ name: 'view', viewId: view.id })
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Save as view"
      width={480}
      footer={
        <>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" disabled={!name.trim() || saving} onClick={() => void save()}>
            Save view
          </Button>
        </>
      }
    >
      <form
        className="lic-form"
        onSubmit={(e) => {
          e.preventDefault()
          void save()
        }}
      >
        <input
          data-autofocus
          autoFocus
          className="lic-input lic-input--title"
          placeholder="View name"
          aria-label="View name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <input
          className="lic-input"
          placeholder="Description (optional)"
          aria-label="Description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <button type="submit" hidden />
      </form>
    </Dialog>
  )
}
