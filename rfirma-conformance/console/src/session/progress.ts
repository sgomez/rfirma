import { useMemo, useRef } from "react";
import type { Snapshot } from "../contract/Snapshot";
import type { Progress } from "../report/SetSection";
import type { Activity } from "../ui/icons";

export interface Batch {
  set: string | null;
  current: string;
  done: number;
  total: number;
  startedAt: number;
  checkSince: number;
}

const idle: Progress = { activityOf: () => null, whyPending: () => null, runningSince: null };

/** Qué hace la sesión con cada comprobación, leído del estado solo si es `live`. */
export function useProgress(snapshot: Snapshot | null, live: boolean): Progress {
  return useMemo(() => {
    if (!snapshot || !live) return idle;
    const running = new Set(snapshot.running?.ids ?? []);
    const queued = new Set(snapshot.queued);
    const asking = snapshot.question?.check ?? null;
    const activityOf = (id: string): Activity | null => {
      if (asking === id) return "asking";
      if (running.has(id)) return "running";
      if (queued.has(id)) return "queued";
      return null;
    };
    return {
      activityOf,
      whyPending: (id: string) => snapshot.why_pending[id] ?? null,
      runningSince: snapshot.running ? performance.now() - snapshot.running.elapsed_ms : null,
    };
  }, [snapshot, live]);
}

/** La tanda en curso: cada comprobación vista en curso o en cola desde que la cola se vació. */
export function useBatch(snapshot: Snapshot | null): Batch | null {
  const seen = useRef<{ ids: Set<string>; startedAt: number } | null>(null);
  return useMemo(() => {
    const running = snapshot?.running;
    if (!snapshot || (!running && snapshot.queued.length === 0)) {
      seen.current = null;
      return null;
    }
    const now = performance.now();
    if (!seen.current) {
      seen.current = { ids: new Set(), startedAt: now - (running?.elapsed_ms ?? 0) };
    }
    const pending = [...(running?.ids ?? []), ...snapshot.queued];
    for (const id of pending) seen.current.ids.add(id);
    const current = running?.ids[0] ?? snapshot.queued[0] ?? "";
    const set =
      snapshot.report?.sets.find((each) => each.checks.some((check) => check.id === current))
        ?.name ?? null;
    const total = seen.current.ids.size;
    return {
      set,
      current,
      done: total - pending.length,
      total,
      startedAt: seen.current.startedAt,
      checkSince: now - (running?.elapsed_ms ?? 0),
    };
  }, [snapshot]);
}
