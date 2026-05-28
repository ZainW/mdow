# Dev diff — WelcomeView polish

Sample PR body with a React change and markdown copy edits for UI dev.

## Summary

- Center the empty state hero
- Add shadcn `Card` for the drop hint
- Wire a dev-only sample loader on the welcome screen

## React

```diff
--- a/apps/desktop/src/renderer/src/components/WelcomeView.tsx
+++ b/apps/desktop/src/renderer/src/components/WelcomeView.tsx
@@ -1,6 +1,8 @@
 import { Button } from './ui/button'
+import { Card, CardContent } from './ui/card'
 import { Logo } from './Logo'
+import { openDevWorkspace } from '../dev/open-dev-workspace'

 export function WelcomeView() {
   const openTab = useAppStore((s) => s.openTab)
@@ -118,14 +120,21 @@
             <Button variant="outline" onClick={() => void handleOpenFolder()}>
               <FolderOpen data-icon="inline-start" />
               Open Folder
             </Button>
+            {import.meta.env.DEV && (
+              <Button variant="secondary" onClick={openDevWorkspace}>
+                Dev samples
+              </Button>
+            )}
           </div>
-          <div className="mt-4 rounded-lg border border-dashed px-4 py-3 text-xs">
-            <strong>Anywhere in this window</strong> — drop `.md` files or a folder.
-          </div>
+          <Card size="sm" className="mt-4 ring-dashed ring-border/80">
+            <CardContent className="py-3 text-xs text-muted-foreground">
+              <strong className="font-medium text-foreground">Anywhere in this window</strong>
+              {' — drop '}
+              <code className="rounded bg-muted px-1 font-mono">.md</code> files or a folder.
+            </CardContent>
+          </Card>
         </div>
```

## Markdown copy

```diff
--- a/README.md
+++ b/README.md
@@ -1,5 +1,5 @@
 # Mdow

-A quiet markdown viewer.
+A quiet markdown viewer built with shadcn Base UI.

 Drop a file anywhere, or open a folder from the welcome screen.
```
