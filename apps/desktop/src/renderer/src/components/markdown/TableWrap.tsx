import type { HTMLAttributes } from 'react'

export function TableWrap(props: HTMLAttributes<HTMLTableElement>) {
  return (
    <div className="table-wrap">
      <table {...props} />
    </div>
  )
}
