# Mermaid gallery

One diagram per Mermaid type, for comparing renderers side by side.

## Flowchart

```mermaid
flowchart TD
  start([Open file]) --> kind{Markdown?}
  kind -- yes --> parse[Parse with comark]
  kind -- no --> html[Sandboxed HTML viewer]
  parse --> shiki[Highlight code]
  parse --> mermaid[Render diagrams]
  shiki --> view[MarkdownView]
  mermaid --> view
  subgraph Renderer
    view --> paint((Paint))
  end
```

## Sequence

```mermaid
sequenceDiagram
  autonumber
  participant R as Renderer
  participant P as Preload
  participant M as Main
  R->>P: window.api.readFile(path)
  P->>M: ipc invoke file:read
  M-->>P: contents
  P-->>R: contents
  Note over R,M: chokidar watches for changes
  loop On change
    M->>R: file:changed
  end
  alt Too large
    R->>R: Show warning
  else Normal
    R->>R: Re-render
  end
```

## Class

```mermaid
classDiagram
  class Tab {
    +string path
    +string content
    +scrollTop number
    +reload() void
  }
  class Workspace {
    +Tab[] tabs
    +open(path) Tab
    +close(tab) void
  }
  class Watcher
  Workspace "1" *-- "many" Tab : holds
  Workspace --> Watcher : uses
  Tab <|-- HtmlTab
```

## State

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Loading: open
  Loading --> Ready: parsed
  Loading --> Error: failed
  Ready --> Loading: file changed
  Error --> Idle: dismiss
  Ready --> [*]
  state Ready {
    [*] --> Reading
    Reading --> Searching: Cmd+F
    Searching --> Reading: Esc
  }
```

## Entity relationship

```mermaid
erDiagram
  FOLDER ||--o{ FILE : contains
  FILE ||--o{ HEADING : has
  FILE {
    string path PK
    int size
    datetime modified
  }
  HEADING {
    string id PK
    int depth
    string text
  }
```

## Gantt

```mermaid
gantt
  title Release plan
  dateFormat YYYY-MM-DD
  section Desktop
    Pele prototype   :a1, 2026-10-17, 3d
    Visual review    :after a1, 2d
  section Native
    Worker spike     :2026-10-20, 4d
    Ship             :milestone, 2026-10-26, 0d
```

## Pie

```mermaid
pie title File types opened
  "Markdown" : 78
  "MDX" : 12
  "HTML" : 10
```

## User journey

```mermaid
journey
  title Reading a doc
  section Open
    Drop file on dock: 5: Reader
    Wait for render: 3: Reader
  section Read
    Scroll: 5: Reader
    Search: 4: Reader
```

## Git graph

```mermaid
gitGraph
  commit
  branch feature
  checkout feature
  commit
  commit
  checkout main
  merge feature
  commit
```

## Mindmap

```mermaid
mindmap
  root((mdow))
    Desktop
      Electron
      comark
    Native
      gpuix
      Bun
    Web
      TanStack Start
```

## Timeline

```mermaid
timeline
  title Mdow releases
  2026-08 : v1.10 : Folder sidebar
  2026-09 : v1.12 : HTML viewer
  2026-10 : v1.13 : Companion chat
```

## Quadrant

```mermaid
quadrantChart
  title Renderer trade-offs
  x-axis Slow --> Fast
  y-axis Heavy --> Light
  quadrant-1 Ideal
  quadrant-2 Light but slow
  quadrant-3 Avoid
  quadrant-4 Fast but heavy
  mermaid.js: [0.25, 0.2]
  Pele: [0.9, 0.8]
  beautiful-mermaid: [0.7, 0.6]
```

## XY chart

```mermaid
xychart-beta
  title "Render time (ms)"
  x-axis [flow, seq, class, state]
  y-axis "ms" 0 --> 90
  bar [24, 31, 82, 40]
  line [24, 31, 82, 40]
```

## Sankey

```mermaid
sankey-beta
Files,Markdown,78
Files,MDX,12
Files,HTML,10
Markdown,Diagrams,20
```

## Block

```mermaid
block-beta
  columns 3
  Main Preload Renderer
  space:3
  IPC:3
```

## Broken

```mermaid
flowchart TD
  A -->
  B -->> {oops
```
