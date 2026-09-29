import { useSyncExternalStore } from 'react'

/** Imperative reader commands from keys, menus, the outline and links. */
export type ReaderCommand =
  | { type: 'scroll'; to: 'top' | 'bottom' }
  | { type: 'page'; direction: 1 | -1 }
  | { type: 'line'; direction: 1 | -1 }
  | { type: 'block'; index: number }
  | { type: 'slug'; slug: string }
  | { type: 'find'; direction: 1 | -1 }

type Listener = (command: ReaderCommand) => void

const listeners = new Set<Listener>()

export function sendReader(command: ReaderCommand) {
  for (const listener of listeners) listener(command)
}

export function onReaderCommand(listener: Listener) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** The first row on screen in the active reader, for the outline's current-section mark. */
let topRow = 0
const rowListeners = new Set<() => void>()

export function setTopRow(row: number) {
  if (row === topRow) return
  topRow = row
  for (const listener of rowListeners) listener()
}

export function useTopRow() {
  return useSyncExternalStore(
    (listener) => {
      rowListeners.add(listener)
      return () => {
        rowListeners.delete(listener)
      }
    },
    () => topRow,
  )
}
