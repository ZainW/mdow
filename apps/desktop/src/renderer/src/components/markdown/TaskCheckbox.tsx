import type { InputHTMLAttributes } from 'react'

export function TaskCheckbox({
  type,
  class: className,
  checked,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & { class?: string }) {
  if (type === 'checkbox' && className?.includes('task-list-item-checkbox')) {
    return (
      <input
        type="checkbox"
        className={className}
        checked={Boolean(checked)}
        disabled
        readOnly
        aria-disabled
        {...props}
      />
    )
  }
  return <input type={type} className={className} checked={checked} {...props} />
}
