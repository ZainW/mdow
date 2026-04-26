/// <reference types="vite/client" />

declare module '*.css'

declare module '@vscode/vscode-languagedetection/model/model.json?url' {
  const src: string
  export default src
}
declare module '@vscode/vscode-languagedetection/model/group1-shard1of1.bin?url' {
  const src: string
  export default src
}

interface File {
  /** Electron-specific: the file's absolute path on disk */
  readonly path: string
}
