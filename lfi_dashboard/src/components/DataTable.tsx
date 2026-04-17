import React, { useMemo, useState } from 'react';
import { T } from '../tokens';

// c0-auto-2 task 26 / BIG #180 (CLAUDE2_500_TASKS.md): generic sortable
// filterable table with sticky headers + zebra stripes. 10+ pages in the
// app hand-rolled their own <table> blocks; this consolidates the shape
// without over-designing the column API.
//
// Design choices:
//  - Column is generic over T so consumers keep their row types. accessor
//    returns ReactNode so columns can render badges / links / formatted
//    numbers without stringifying at the call site.
//  - sortKey is optional; when omitted we fall back to the accessor's
//    string form. Set a sortKey that returns a number to get numeric sort.
//  - Sort state is internal by default but can be lifted via `sort` +
//    `onSortChange` props when the caller wants to persist or URL-encode
//    the current sort.
//  - Filter is opt-in via filterQuery + filterFn; defaults skip filtering
//    entirely. Keeps the zero-config call site minimal.
//  - Sticky header uses `position: sticky` + `top: 0` inside a scroll
//    container; caller decides the container height (or lets the table
//    grow naturally).
//
// AVP-PASS-27 (a11y):
//  - <th> with aria-sort={ascending|descending|none}
//  - sortable headers are role=button + tabIndex=0 + Enter/Space activation
//  - empty state renders a full-row message inside <tbody> so the table
//    shape stays framed

export interface Column<T> {
  id: string;
  /** Header text. Pass JSX via header node for richer headers. */
  header: string;
  accessor: (row: T) => React.ReactNode;
  /** Default true. Set false to lock a column out of the sort cycle. */
  sortable?: boolean;
  /** Value to sort by. Defaults to the string of accessor. Return a number
   *  to get a numeric comparison. */
  sortKey?: (row: T) => number | string;
  align?: 'left' | 'right' | 'center';
  /** CSS width -- px, %, or 'auto'. Left blank = auto-shrink. */
  width?: string;
  /** Optional cell style overrides merged onto every <td> of this column. */
  cellStyle?: React.CSSProperties;
}

export type SortDir = 'asc' | 'desc';

export interface DataTableProps<T> {
  C: any;
  rows: T[];
  columns: ReadonlyArray<Column<T>>;
  /** Row identity -- must be stable across renders for keys + sort. */
  rowKey: (row: T) => string | number;
  /** Lifted sort state. When absent the table manages its own. */
  sort?: { col: string; dir: SortDir };
  onSortChange?: (next: { col: string; dir: SortDir }) => void;
  /** Optional filter text applied before sorting. */
  filterQuery?: string;
  /** Match predicate. When omitted, falls back to a case-insensitive
   *  substring match against the string form of every accessor. */
  filterFn?: (row: T, query: string) => boolean;
  /** Default true. 1.5% alpha zebra rows; disable for simpler layouts. */
  stripe?: boolean;
  /** Default true. Set false if the parent wraps the table in its own
   *  layout that doesn't benefit from sticky headers. */
  stickyHeader?: boolean;
  /** Message shown when rows is empty or the filter returns zero matches. */
  emptyText?: string;
  /** Font size for cells -- sizeSm default; set sizeMd for larger layouts. */
  cellFontSize?: string;
}

export function DataTable<T>({
  C, rows, columns, rowKey,
  sort, onSortChange, filterQuery, filterFn,
  stripe = true, stickyHeader = true,
  emptyText, cellFontSize,
}: DataTableProps<T>): JSX.Element {
  const [internalSort, setInternalSort] = useState<{ col: string; dir: SortDir } | null>(null);
  const activeSort = sort ?? internalSort;

  const toggleSort = (col: Column<T>) => {
    if (col.sortable === false) return;
    const next: { col: string; dir: SortDir } = activeSort?.col === col.id
      ? { col: col.id, dir: activeSort.dir === 'asc' ? 'desc' : 'asc' }
      : { col: col.id, dir: (col.align === 'right' ? 'desc' : 'asc') };
    if (onSortChange) onSortChange(next);
    else setInternalSort(next);
  };

  const ariaSort = (col: Column<T>): 'ascending' | 'descending' | 'none' =>
    activeSort?.col === col.id ? (activeSort.dir === 'asc' ? 'ascending' : 'descending') : 'none';

  const arrow = (col: Column<T>): string =>
    activeSort?.col !== col.id ? '' : activeSort.dir === 'asc' ? ' \u25B2' : ' \u25BC';

  // Filter first, then sort, so sort doesn't do unnecessary work on rows
  // that won't render.
  const processed = useMemo(() => {
    const q = (filterQuery ?? '').trim();
    const match = q
      ? (filterFn ?? ((row: T, qq: string) => {
          const needle = qq.toLowerCase();
          return columns.some(col => {
            const v = col.accessor(row);
            return typeof v === 'string' && v.toLowerCase().includes(needle);
          });
        }))
      : null;
    const filtered = match ? rows.filter(r => match(r, q)) : rows;
    if (!activeSort) return filtered;
    const col = columns.find(c => c.id === activeSort.col);
    if (!col) return filtered;
    const keyOf = col.sortKey ?? ((row: T) => {
      const v = col.accessor(row);
      return typeof v === 'string' ? v : String(v);
    });
    const sign = activeSort.dir === 'asc' ? 1 : -1;
    return [...filtered].sort((a, b) => {
      const ka = keyOf(a);
      const kb = keyOf(b);
      if (typeof ka === 'number' && typeof kb === 'number') return sign * (ka - kb);
      return sign * String(ka).localeCompare(String(kb));
    });
  }, [rows, columns, filterQuery, filterFn, activeSort]);

  const fz = cellFontSize ?? T.typography.sizeSm;
  const headerBase: React.CSSProperties = {
    padding: '10px 14px', fontWeight: T.typography.weightBold,
    color: C.textSecondary, background: C.bgCard,
    borderBottom: `1px solid ${C.borderSubtle}`,
    whiteSpace: 'nowrap',
    ...(stickyHeader ? { position: 'sticky' as const, top: 0, zIndex: 1 } : null),
  };

  return (
    <div style={{ border: `1px solid ${C.borderSubtle}`, borderRadius: T.radii.md, overflow: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: fz, color: C.text }}>
        <thead>
          <tr>
            {columns.map(col => {
              const sortable = col.sortable !== false;
              const th: React.CSSProperties = {
                ...headerBase,
                textAlign: col.align ?? 'left',
                width: col.width,
                cursor: sortable ? 'pointer' : 'default',
                userSelect: 'none',
              };
              return (
                <th key={col.id}
                  onClick={sortable ? () => toggleSort(col) : undefined}
                  onKeyDown={sortable ? (e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault(); toggleSort(col);
                    }
                  } : undefined}
                  tabIndex={sortable ? 0 : undefined}
                  role={sortable ? 'button' : undefined}
                  aria-sort={sortable ? ariaSort(col) : undefined}
                  style={th}>{col.header}{arrow(col)}</th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {processed.length === 0 ? (
            <tr>
              <td colSpan={columns.length} style={{
                padding: '20px', textAlign: 'center',
                color: C.textMuted, fontSize: T.typography.sizeSm,
                fontStyle: 'italic',
              }}>{emptyText ?? (filterQuery ? `No rows match "${filterQuery}"` : 'No rows')}</td>
            </tr>
          ) : processed.map((row, i) => (
            <tr key={rowKey(row)}
              style={stripe && i % 2 === 1 ? { background: 'rgba(255,255,255,0.015)' } : undefined}>
              {columns.map(col => (
                <td key={col.id} style={{
                  padding: '10px 14px',
                  textAlign: col.align ?? 'left',
                  borderBottom: `1px solid ${C.borderSubtle}`,
                  ...col.cellStyle,
                }}>{col.accessor(row)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
