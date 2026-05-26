import { Skeleton } from './ui/skeleton'

export function DocumentSkeleton() {
  return (
    <div aria-hidden className="flex flex-col gap-6">
      <Skeleton className="h-8 w-2/3" />
      <div className="flex flex-col gap-2">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-4/5" />
      </div>
      <Skeleton className="h-6 w-1/2" />
      <div className="flex flex-col gap-2">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-11/12" />
        <Skeleton className="h-4 w-3/4" />
      </div>
      <Skeleton className="h-24 w-full rounded-lg" />
    </div>
  )
}
