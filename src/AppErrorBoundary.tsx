import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AppButton } from './design/AppButton';

interface AppErrorBoundaryProps {
  children: ReactNode;
}

interface AppErrorBoundaryState {
  error: Error | null;
  componentStack: string;
}

/** Keep a renderer failure visible and recoverable instead of leaving a blank native window. */
export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { error: null, componentStack: '' };

  static getDerivedStateFromError(error: unknown): Partial<AppErrorBoundaryState> {
    return { error: error instanceof Error ? error : new Error(String(error)) };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    this.setState({ componentStack: info.componentStack ?? '' });
    console.error('Code Atlas could not render the current view.', error, info.componentStack);
  }

  render() {
    const { error, componentStack } = this.state;
    if (!error) return this.props.children;

    return (
      <main className="app-error-fallback" role="alert" aria-labelledby="app-error-title">
        <div className="app-error-card">
          <span className="app-error-eyebrow">CODE ATLAS · VIEW ERROR</span>
          <h1 id="app-error-title">This view could not be displayed.</h1>
          <p>The rest of Code Atlas is still running. Reload the workspace to return to the repository overview.</p>
          <AppButton intent="primary" type="button" className="" onClick={() => window.location.reload()}>
            Reload workspace
          </AppButton>
          <details className="app-error-details">
            <summary>Technical details</summary>
            <pre>{error.message}{componentStack ? `\n${componentStack}` : ''}</pre>
          </details>
        </div>
      </main>
    );
  }
}
