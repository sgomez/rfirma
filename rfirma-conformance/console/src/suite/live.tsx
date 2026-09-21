import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { LiveLine } from "../contract/LiveLine";
import type { Snapshot } from "../contract/Snapshot";
import type { Suite } from "./suite";

export type Connection = "connecting" | "live" | "lost";

type LineListener = (line: LiveLine) => void;

export interface Toast {
  id: number;
  text: string;
}

interface Live {
  suite: Suite;
  snapshot: Snapshot | null;
  connection: Connection;
  onLine: (listener: LineListener) => () => void;
  toasts: Toast[];
  complain: (action: string) => (error: unknown) => void;
  dismiss: (id: number) => void;
}

const LiveContext = createContext<Live | null>(null);

export function SuiteProvider({ suite, children }: { suite: Suite; children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connection, setConnection] = useState<Connection>("connecting");
  const [toasts, setToasts] = useState<Toast[]>([]);
  const listeners = useRef(new Set<LineListener>());
  const nextToast = useRef(0);

  useEffect(() => {
    const stream = suite.events();
    stream.addEventListener("state", (event) => {
      setSnapshot(JSON.parse((event as MessageEvent<string>).data) as Snapshot);
      setConnection("live");
    });
    stream.addEventListener("log", (event) => {
      const line = JSON.parse((event as MessageEvent<string>).data) as LiveLine;
      for (const listener of listeners.current) listener(line);
    });
    stream.addEventListener("open", () => setConnection("live"));
    stream.addEventListener("error", () => setConnection("lost"));
    return () => stream.close();
  }, [suite]);

  const onLine = useCallback((listener: LineListener) => {
    listeners.current.add(listener);
    return () => {
      listeners.current.delete(listener);
    };
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((shown) => shown.filter((toast) => toast.id !== id));
  }, []);

  const complain = useCallback(
    (action: string) => (error: unknown) => {
      const id = nextToast.current++;
      const reason = error instanceof Error ? error.message : String(error);
      setToasts((shown) => [...shown.slice(-3), { id, text: `${action}: ${reason}` }]);
      window.setTimeout(() => dismiss(id), 8000);
    },
    [dismiss],
  );

  const live = useMemo(
    () => ({ suite, snapshot, connection, onLine, toasts, complain, dismiss }),
    [suite, snapshot, connection, onLine, toasts, complain, dismiss],
  );
  return <LiveContext.Provider value={live}>{children}</LiveContext.Provider>;
}

export function useLive(): Live {
  const live = useContext(LiveContext);
  if (!live) throw new Error("useLive fuera de SuiteProvider");
  return live;
}
