import type { ErrorType } from '../../../shared/types'

export function getReadErrorType(error: unknown): ErrorType {
  if (error instanceof Error) {
    if (error.message === 'not-found') return 'not-found'
    if (error.message === 'permission-denied') return 'permission-denied'
    if (error.message === 'read-error') return 'read-error'
  }
  return 'read-error'
}
