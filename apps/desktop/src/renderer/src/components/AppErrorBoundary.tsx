import { Component, type ErrorInfo, type ReactNode } from 'react'
import { RotateCcw, TriangleAlert } from 'lucide-react'
import { Button } from './ui/button'
import { iconStroke } from '../lib/icons'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class AppErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('Application render error:', error, info.componentStack)
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen w-screen flex-col items-center justify-center gap-4 bg-background p-12 text-center">
          <TriangleAlert
            className="size-12 text-destructive"
            strokeWidth={iconStroke.default}
          />
          <div>
            <h1 className="text-xl font-semibold">Something went wrong</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              The application encountered an unexpected error.
            </p>
            {this.state.error && (
              <p className="mt-3 max-w-lg font-mono text-xs text-muted-foreground/70">
                {this.state.error.message}
              </p>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => window.location.reload()}
          >
            <RotateCcw className="mr-2 size-3.5" strokeWidth={iconStroke.default} />
            Reload application
          </Button>
        </div>
      )
    }
    return this.props.children
  }
}
