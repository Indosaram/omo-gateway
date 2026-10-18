import React from 'react'
import { Check, Minus } from 'lucide-react'

export interface CheckboxProps {
  checked: boolean
  indeterminate?: boolean
  onChange: (checked: boolean, event: React.MouseEvent | React.KeyboardEvent) => void
  disabled?: boolean
  label?: string
  className?: string
}

export function Checkbox({ checked, indeterminate, onChange, disabled, label, className }: CheckboxProps) {
  const state = indeterminate ? 'indeterminate' : checked ? 'checked' : 'unchecked'
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={indeterminate ? 'mixed' : checked}
      aria-label={label}
      disabled={disabled}
      data-state={state}
      className={className ? `lui-checkbox ${className}` : 'lui-checkbox'}
      onClick={(event) => {
        event.stopPropagation()
        onChange(indeterminate ? true : !checked, event)
      }}
      onKeyDown={(event) => {
        if (event.key === ' ' || event.key === 'Enter') {
          event.preventDefault()
          event.stopPropagation()
          onChange(indeterminate ? true : !checked, event)
        }
      }}
    >
      {state === 'checked' && <Check size={10} strokeWidth={3} />}
      {state === 'indeterminate' && <Minus size={10} strokeWidth={3} />}
    </button>
  )
}
