import { Component, type ErrorInfo, type ReactNode } from "react";
import { reportComponentError } from "./lib/errorReporting";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * React has no hook equivalent for `componentDidCatch`, so this is a class
 * component -- the one exception to the rest of the app. Without it, a
 * render-time exception anywhere below blanks the window with nothing
 * recorded; this reports the failure (see `src/lib/errorReporting.ts`) and
 * shows a fallback instead of a blank screen.
 *
 * The fallback follows AGENTS.md rule 5 ("errors go where the thing is, as
 * text"): plain text at the ordinary size, no boxed alert, no icon -- just
 * the sentence, the error on its own line, and a way back in.
 */
export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    reportComponentError(error, info.componentStack ?? null);
  }

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div style={{ padding: 24, fontSize: "var(--text-base, 14px)" }}>
        <p>FrameWork hit an error it could not recover from.</p>
        <code
          style={{
            display: "block",
            marginTop: 8,
            whiteSpace: "pre-wrap",
          }}
        >
          {error.message}
        </code>
        <button
          type="button"
          style={{ marginTop: 16 }}
          onClick={() => window.location.reload()}
        >
          Reload
        </button>
      </div>
    );
  }
}
