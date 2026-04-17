// c0-auto-2 task 52 (CLAUDE2_500_TASKS.md): barrel for shared components.
//
// Pages can now write a single import when they need several shared
// primitives:
//   import { Label, StatCard, ErrorAlert, SkeletonLoader } from './components';
//
// This file is the source of truth for what's public from the components
// directory. Anything not re-exported here is implementation-private.
// Extending: when a new component lands in ./components, add one line
// here in alphabetical order.

export { BarChart } from './BarChart';
export { ErrorAlert } from './ErrorAlert';
export { Label } from './Label';
export { SkeletonLoader } from './SkeletonLoader';
export { StatCard } from './StatCard';
export { TabBar } from './TabBar';
export type { TabDef } from './TabBar';
// DataTable  -- task 26, pending
