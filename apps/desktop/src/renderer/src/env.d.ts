/// <reference types="vite/client" />

declare module '*.css'

interface File {
  /** Electron-specific: the file's absolute path on disk */
  readonly path: string
}
