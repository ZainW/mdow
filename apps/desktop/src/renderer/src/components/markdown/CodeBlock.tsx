import type { HTMLAttributes } from 'react'
import { Check, Copy } from 'lucide-react'
import { iconSize, iconStroke } from '../../lib/icons'

export function CodeBlock({
  children,
  language,
  ...props
}: HTMLAttributes<HTMLPreElement> & { language?: string; class?: string }) {
  const className = props.class
  const { class: _class, ...preProps } = props
  return (
    <div className="code-block-wrapper relative">
      {language ? <span className="code-lang-badge">{language}</span> : null}
      <pre className={className} {...preProps}>
        {children}
      </pre>
      <button
        className="copy-code-btn"
        type="button"
        data-copy-code
        aria-label="Copy code"
        title="Copy code"
      >
        <Copy
          className="copy-icon copy-icon-default"
          size={iconSize.md}
          strokeWidth={iconStroke.default}
          aria-hidden
        />
        <Check
          className="copy-icon copy-icon-done"
          size={iconSize.md}
          strokeWidth={iconStroke.emphasis}
          aria-hidden
        />
      </button>
    </div>
  )
}
