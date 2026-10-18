import React, { forwardRef } from 'react'
import { Kbd } from './Kbd'
import { Tooltip } from './Tooltip'

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger'
export type ButtonSize = 'sm' | 'md'

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant
  size?: ButtonSize
  icon?: React.ReactNode
  kbd?: string[]
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = 'secondary', size = 'md', icon, kbd, className, children, type = 'button', ...rest },
  ref,
) {
  const iconOnly = icon !== undefined && (children === undefined || children === null || children === false)
  const classes = [
    'lui-btn',
    `lui-btn--${variant}`,
    size === 'sm' ? 'lui-btn--sm' : '',
    iconOnly ? 'lui-btn--icon-only' : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ')
  return (
    <button ref={ref} type={type} className={classes} {...rest}>
      {icon}
      {children}
      {kbd && kbd.length > 0 && <Kbd keys={kbd} />}
    </button>
  )
})

export interface IconButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  label: string
  icon: React.ReactNode
  size?: ButtonSize
  active?: boolean
  kbd?: string[]
  tooltipSide?: 'top' | 'bottom' | 'left' | 'right'
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { label, icon, size = 'md', active, kbd, tooltipSide = 'bottom', className, type = 'button', ...rest },
  ref,
) {
  const classes = ['lui-iconbtn', size === 'sm' ? 'lui-iconbtn--sm' : '', active ? 'lui-iconbtn--active' : '', className ?? '']
    .filter(Boolean)
    .join(' ')
  return (
    <Tooltip content={label} kbd={kbd} side={tooltipSide}>
      <button ref={ref} type={type} aria-label={label} aria-pressed={active} className={classes} {...rest}>
        {icon}
      </button>
    </Tooltip>
  )
})
