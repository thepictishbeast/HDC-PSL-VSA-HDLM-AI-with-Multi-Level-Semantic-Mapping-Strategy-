import React from 'react';

// Class component because React 18 still requires class for error boundaries.
// Shows a helpful recovery surface instead of a blank page when any child throws
// during render. Resets on button click; offers reload as escape hatch.
// Theme fallback defaults are dark-palette safe since the boundary renders
// above theme context.
export class AppErrorBoundary extends React.Component<
  { children: React.ReactNode; themeBg?: string; themeText?: string; themeAccent?: string },
  { error: Error | null; componentStack: string | null }
> {
  state = { error: null as Error | null, componentStack: null as string | null };
  static getDerivedStateFromError(error: Error) { return { error, componentStack: null }; }
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('[AppErrorBoundary]', error, info?.componentStack);
    this.setState({ componentStack: info?.componentStack ?? null });
  }
  reset = () => { this.setState({ error: null, componentStack: null }); };
  render() {
    if (!this.state.error) return this.props.children;
    const bg = this.props.themeBg || '#0b0d14';
    const fg = this.props.themeText || '#e8e6f0';
    const accent = this.props.themeAccent || '#8b7bf7';
    const err = this.state.error;
    return (
      <div role="alert" style={{
        minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: bg, color: fg, padding: '40px', fontFamily: "'DM Sans', -apple-system, sans-serif",
      }}>
        <div style={{ maxWidth: '560px', width: '100%' }}>
          <div style={{ fontSize: '13px', color: accent, fontWeight: 700, letterSpacing: '0.14em', textTransform: 'uppercase', marginBottom: '8px' }}>
            UI Error
          </div>
          <h2 style={{ fontSize: '22px', fontWeight: 700, margin: '0 0 10px', letterSpacing: '-0.01em' }}>
            Something broke — but your work is safe
          </h2>
          <p style={{ fontSize: '14px', lineHeight: 1.6, opacity: 0.8, margin: '0 0 18px' }}>
            The dashboard hit a rendering error. Conversations and settings live in localStorage and
            are untouched. Try again to re-mount the UI; if that fails, reload.
          </p>
          <pre style={{
            background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.08)',
            borderRadius: '8px', padding: '12px 14px', fontSize: '12px', lineHeight: 1.5,
            color: fg, overflow: 'auto', maxHeight: '200px', margin: '0 0 18px',
            fontFamily: "'JetBrains Mono', monospace",
          }}>{String(err?.message || err)}</pre>
          <div style={{ display: 'flex', gap: '10px' }}>
            <button onClick={this.reset} style={{
              padding: '10px 18px', fontSize: '13px', fontWeight: 700,
              color: '#fff', background: accent, border: 'none', borderRadius: '8px',
              cursor: 'pointer', fontFamily: 'inherit',
            }}>Try again</button>
            <button onClick={() => window.location.reload()} style={{
              padding: '10px 18px', fontSize: '13px', fontWeight: 700,
              color: fg, background: 'transparent',
              border: '1px solid rgba(255,255,255,0.12)', borderRadius: '8px',
              cursor: 'pointer', fontFamily: 'inherit',
            }}>Reload page</button>
          </div>
          {this.state.componentStack && (
            <details style={{ marginTop: '18px', fontSize: '11px', opacity: 0.6 }}>
              <summary style={{ cursor: 'pointer' }}>Component stack</summary>
              <pre style={{
                background: 'rgba(255,255,255,0.03)', padding: '10px',
                borderRadius: '6px', overflow: 'auto', maxHeight: '160px',
                fontFamily: "'JetBrains Mono', monospace", fontSize: '10px',
              }}>{this.state.componentStack}</pre>
            </details>
          )}
        </div>
      </div>
    );
  }
}
