import { defineConfig } from 'vite';

// Minimal explicit Vite config. Enables source maps in production builds so
// errors surfaced by AppErrorBoundary point back to real lines instead of
// minified positions. Dev server otherwise unchanged.
//
// manualChunks: split heavy vendor deps into their own chunks so they cache
// across rebuilds — only the changed chunk re-downloads, not the whole 500 KB
// main bundle. react-vendor + virtuoso are stable; the app chunk is what
// changes on every commit.
export default defineConfig({
  build: {
    sourcemap: true,
    minify: 'esbuild',
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('react-dom') || id.includes('/react/') || id.includes('scheduler')) {
              return 'vendor-react';
            }
            if (id.includes('react-virtuoso')) return 'vendor-virtuoso';
            if (id.includes('@xterm') || id.includes('xterm')) return 'vendor-xterm';
            if (id.includes('highlight.js')) return 'vendor-hljs';
            if (id.includes('eruda')) return 'vendor-eruda';
            return 'vendor-misc';
          }
          return undefined;
        },
      },
    },
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
  },
});
