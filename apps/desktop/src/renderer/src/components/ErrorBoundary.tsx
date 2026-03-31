import { Component, type ErrorInfo, type ReactNode } from 'react'
import { Warning, ArrowCounterClockwise } from '@phosphor-icons/react'
import { Button } from './ui/button'

interface Props {
  children: ReactNode
  tabId: string
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('Render error in markdown view:', error, info.componentStack)
  }

  componentDidUpdate(prevProps: Props): void {
    if (prevProps.tabId !== this.props.tabId) {
      this.setState({ hasError: false, error: null })
    }
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-4 p-12 text-center">
          <Warning className="size-10 text-muted-foreground" />
          <div>
            <h2 className="text-lg font-medium">Something went wrong</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              An error occurred while rendering this document.
            </p>
            {this.state.error && (
              <p className="mt-2 max-w-md font-mono text-xs text-muted-foreground/70">
                {this.state.error.message}
              </p>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => this.setState({ hasError: false, error: null })}
          >
            <ArrowCounterClockwise className="mr-2 size-3.5" />
            Try again
          </Button>
        </div>
      )
    }
    return this.props.children
  }
}
