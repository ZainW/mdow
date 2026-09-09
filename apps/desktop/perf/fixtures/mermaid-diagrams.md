# Mermaid diagram QA

Sample document for screenshotting the in-process SVG renderer. Diagrams should read as ink on paper: paper-warm nodes, line strokes, muted edges, IBM Plex Mono labels. Teal appears only on hover or selection — never as a rainbow fill.

## Flow

```mermaid
flowchart TD
  open[Open a note] --> parse[Parse markdown]
  parse --> draw[Draw SVG in the reader]
  draw --> read[Read on paper]
```

## Sequence

```mermaid
sequenceDiagram
  participant Reader
  participant Comark
  participant Mermaid
  Reader->>Comark: fence
  Comark->>Mermaid: render(id, source)
  Mermaid-->>Reader: inline SVG
```

## Across the page

```mermaid
flowchart LR
  md[Markdown] --> parse[comark]
  parse --> svg[SVG]
  svg --> view[MarkdownView]
```

## States

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Rendering: open document
  Rendering --> Idle: done
```
