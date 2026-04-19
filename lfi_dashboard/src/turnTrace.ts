/**
 * turnTrace — per-turn latency instrumentation.
 *
 * Tags every chat send with a turn_id so diag exports show the full
 * round-trip timeline:
 *
 *   t0  send       — payload pushed onto the WS
 *   t1  first_frame — first onmessage with this turn (any type)
 *   t2  response    — chat_response (or chat_error) terminal frame
 *   t3  rendered    — React commit reflecting the new assistant message
 *
 * Each phase emits a `diag.info('turn-trace', ...)` entry. Negative or
 * stale turns are no-ops, so unrelated WS frames (telemetry, fleet
 * updates) don't pollute the trace.
 */

import { diag } from './diag';

export interface TurnTrace {
  turnId: string;
  sentAt: number;
  firstFrameAt?: number;
  responseAt?: number;
  renderedAt?: number;
  contentLen?: number;
  error?: string;
}

let counter = 0;

export function newTurnId(): string {
  counter = (counter + 1) & 0xffff;
  return `turn-${Date.now().toString(36)}-${counter.toString(36)}`;
}

export function markSend(payloadLen: number): TurnTrace {
  const t: TurnTrace = { turnId: newTurnId(), sentAt: performance.now(), contentLen: payloadLen };
  diag.info('turn-trace', `send ${t.turnId}`, { phase: 'send', turnId: t.turnId, payloadLen });
  return t;
}

export function markFirstFrame(t: TurnTrace | null, frameType: string): void {
  if (!t || t.firstFrameAt != null) return;
  t.firstFrameAt = performance.now();
  diag.info('turn-trace', `first-frame ${t.turnId} ${frameType}`, {
    phase: 'first_frame',
    turnId: t.turnId,
    frameType,
    elapsedMs: Math.round(t.firstFrameAt - t.sentAt),
  });
}

export function markResponse(t: TurnTrace | null, opts?: { error?: string; contentLen?: number }): void {
  if (!t || t.responseAt != null) return;
  t.responseAt = performance.now();
  if (opts?.error) t.error = opts.error;
  diag.info('turn-trace', `response ${t.turnId}${opts?.error ? ' ERROR' : ''}`, {
    phase: 'response',
    turnId: t.turnId,
    sendToFirstFrameMs: t.firstFrameAt ? Math.round(t.firstFrameAt - t.sentAt) : null,
    sendToResponseMs: Math.round(t.responseAt - t.sentAt),
    error: opts?.error || null,
    contentLen: opts?.contentLen ?? null,
  });
}

export function markRendered(t: TurnTrace | null): void {
  if (!t || t.renderedAt != null || t.responseAt == null) return;
  t.renderedAt = performance.now();
  diag.info('turn-trace', `rendered ${t.turnId}`, {
    phase: 'rendered',
    turnId: t.turnId,
    sendToRenderedMs: Math.round(t.renderedAt - t.sentAt),
    responseToRenderedMs: Math.round(t.renderedAt - t.responseAt),
  });
}
