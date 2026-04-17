import { defineConfig } from 'vite';

// Minimal explicit Vite config. Enables source maps in production builds so
// errors surfaced by AppErrorBoundary point back to real lines instead of
// minified positions. Dev server otherwise unchanged.
export default defineConfig({
  build: {
    sourcemap: true,
    // Don't tree-shake away legitimate `console.warn` / `.error`; our
    // instrumentation uses them for real diagnostics. `console.debug` stays.
    minify: 'esbuild',
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
  },
});
