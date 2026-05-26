/// <reference types="vite/client" />

declare module '*.css'

interface File {
  /** Electron-specific: the file's absolute path on disk */
  readonly path: string
}

type UpdaterUnsubscribe = () => void

// Shared domain types live in src/shared/types.ts (included via tsconfig.web.json).
// window.api is typed via apps/desktop/src/preload/env.d.ts → ElectronAPI.
