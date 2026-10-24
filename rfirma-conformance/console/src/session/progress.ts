import { useMemo } from "react";
import type { Snapshot } from "../contract/Snapshot";
import type { Progress } from "../report/SetSection";
import type { Activity } from "../ui/icons";

const idle: Progress = { activityOf: () => null, whyPending: () => null, runningSince: null };

/** Qué hace la sesión con cada comprobación, leído del estado solo si es `live`. */
export function useProgress(snapshot: Snapshot | null, live: boolean): Progress {
  return useMemo(() => {
    if (!snapshot || !live) return idle;
    const running = new Set(snapshot.running?.ids ?? []);
    const queued = new Set(snapshot.queued);
    const activityOf = (id: string): Activity | null => {
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
