# Small Baseline

This fixture measures the fastest normal path for opening and rendering a markdown file.

It includes a short list so the renderer still creates a few block nodes:

- Open the file
- Parse markdown
- Paint content

```ts
export const baseline = 'small'
```
