import {
  type KeyboardEvent,
  memo,
  type PointerEvent,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useLive } from "../suite/live";
import { useShortcuts } from "../ui/shortcuts";

export type Provenance = "sede" | "cliente" | "suite";

const PROVENANCES: readonly Provenance[] = ["sede", "cliente", "suite"];
const FORMER: Record<string, Provenance> = { conductor: "sede", sujeto: "cliente", arnés: "suite" };
const CHUNK = 200;
const COLLAPSED = 38;

interface Line {
  clock: string;
  provenance: Provenance | null;
  text: string;
}

export function parseLine(raw: string): Line {
  const match = /^(\d\d:\d\d\.\d\d) (\S+)\s?(.*)$/.exec(raw);
  if (!match) return { clock: "", provenance: null, text: raw };
  const tag = match[2] ?? "";
  const provenance = (FORMER[tag] ?? tag) as Provenance;
  return {
    clock: match[1] ?? "",
    provenance: PROVENANCES.includes(provenance) ? provenance : null,
    text: (match[3] ?? "").trimStart(),
  };
}

interface LogDockProps {
  check: string | null;
  report?: string;
  runningIds: readonly string[];
  follow?: { on: boolean; set: (on: boolean) => void };
}

export function LogDock({ check, report, runningIds, follow }: LogDockProps) {
  const { suite, onLine } = useLive();
  const lines = useRef<Line[]>([]);
  const held = useRef<Line[]>([]);
  const [, setVersion] = useState(0);
  const [paused, setPaused] = useState(false);
  const [shown, setShown] = useState<ReadonlySet<Provenance>>(new Set(PROVENANCES));
  const [complaint, setComplaint] = useState<string | null>(null);
  const [height, setHeight] = useState(() => Math.round(Math.min(260, window.innerHeight * 0.28)));
  const [collapsed, setCollapsed] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const stuck = useRef(true);
  const frame = useRef(0);

  const redraw = useCallback(() => {
    if (frame.current) return;
    frame.current = window.requestAnimationFrame(() => {
      frame.current = 0;
      setVersion((version) => version + 1);
    });
  }, []);

  const runsThisOne = check !== null && runningIds.includes(check);
  const head = runsThisOne ? (runningIds[0] ?? null) : null;

  // biome-ignore lint/correctness/useExhaustiveDependencies: una pasada nueva de la comprobación reescribe su registro
  useEffect(() => {
    lines.current = [];
    held.current = [];
    setComplaint(null);
    stuck.current = true;
    redraw();
    if (!check) return;
    let current = true;
    suite
      .log(check, report)
      .then((text) => {
        if (!current) return;
        const earlier = text.split("\n").filter(Boolean).map(parseLine);
        lines.current = [...earlier, ...lines.current];
        redraw();
      })
      .catch((error: unknown) => {
        if (current) setComplaint(error instanceof Error ? error.message : String(error));
      });
    return () => {
      current = false;
    };
  }, [suite, check, report, redraw, head]);

  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  useEffect(() => {
    if (!head) return;
    return onLine((line) => {
      if (line.check !== head) return;
      const parsed = parseLine(line.line);
      if (pausedRef.current) held.current.push(parsed);
      else lines.current.push(parsed);
      redraw();
    });
  }, [onLine, head, redraw]);

  const togglePause = useCallback(() => {
    setPaused((was) => {
      if (was) {
        lines.current.push(...held.current);
        held.current = [];
        stuck.current = true;
        redraw();
      }
      return !was;
    });
  }, [redraw]);

  useShortcuts({
    l: togglePause,
    f: follow ? () => follow.set(!follow.on) : undefined,
  });

  useLayoutEffect(() => {
    const element = scroller.current;
    if (element && stuck.current) element.scrollTop = element.scrollHeight;
  });

  useEffect(() => {
    const reserved = collapsed ? COLLAPSED : height;
    document.documentElement.style.setProperty("--dock-height", `${reserved}px`);
  }, [collapsed, height]);

  useEffect(() => () => document.documentElement.style.setProperty("--dock-height", "0px"), []);

  const onScroll = () => {
    const element = scroller.current;
    if (!element) return;
    stuck.current = element.scrollHeight - element.scrollTop - element.clientHeight < 24;
  };

  const all = lines.current;
  const filterKey = PROVENANCES.filter((each) => shown.has(each)).join(",");
  const chunks = Math.ceil(all.length / CHUNK);

  return (
    <section
      className={`log-dock${collapsed ? " is-collapsed" : ""}`}
      style={{ height: collapsed ? COLLAPSED : height }}
      aria-label="Registro"
    >
      {!collapsed && <ResizeHandle height={height} onResize={setHeight} />}
      <header className="log-bar">
        <button
          type="button"
          className="log-title"
          aria-expanded={!collapsed}
          onClick={() => setCollapsed((was) => !was)}
          title={collapsed ? "Mostrar el registro" : "Ocultar el registro"}
        >
          Registro
        </button>
        <span className="log-check" title={check ?? undefined}>
          {check ?? "ninguna comprobación elegida"}
        </span>
        {head && <span className="live-dot" role="img" aria-label="en directo" />}
        <span className="grow" />
        <fieldset className="filter" aria-label="Procedencia">
          {PROVENANCES.map((provenance) => (
            <button
              key={provenance}
              type="button"
              className={`chip provenance-${provenance}`}
              aria-pressed={shown.has(provenance)}
              onClick={() =>
                setShown((was) => {
                  const next = new Set(was);
                  if (next.has(provenance)) next.delete(provenance);
                  else next.add(provenance);
                  return next;
                })
              }
            >
              <span className="swatch" aria-hidden="true" />
              {provenance}
            </button>
          ))}
        </fieldset>
        {follow && (
          <label className="switch" title="Al empezar cada comprobación, muestra su registro (f)">
            <input
              type="checkbox"
              checked={follow.on}
              onChange={(event) => follow.set(event.target.checked)}
            />
            <span>Ir a la comprobación en curso</span>
          </label>
        )}
        <button
          type="button"
          className="button ghost small"
          aria-pressed={paused}
          onClick={togglePause}
          aria-keyshortcuts="l"
        >
          {paused ? `Reanudar${held.current.length ? ` (${held.current.length})` : ""}` : "Pausar"}
          <kbd>l</kbd>
        </button>
      </header>
      {!collapsed && (
        <div
          className="log-lines"
          ref={scroller}
          onScroll={onScroll}
          role="log"
          aria-live={paused ? "off" : "polite"}
          aria-relevant="additions"
          // biome-ignore lint/a11y/noNoninteractiveTabindex: un registro que se desplaza tiene que alcanzarse con el teclado
          tabIndex={0}
        >
          {complaint && <p className="log-empty">El registro no se pudo leer: {complaint}</p>}
          {!complaint && all.length === 0 && (
            <p className="log-empty">
              {check
                ? head
                  ? "Esperando la primera línea…"
                  : "Sin registro: la comprobación no se ha ejecutado en este informe."
                : "Elige una comprobación para ver su registro."}
            </p>
          )}
          {Array.from({ length: chunks }, (_, index) => (
            <LogChunk
              // biome-ignore lint/suspicious/noArrayIndexKey: un tramo es su posición en el registro
              key={index}
              lines={all}
              from={index * CHUNK}
              count={Math.min(CHUNK, all.length - index * CHUNK)}
              filterKey={filterKey}
              shown={shown}
            />
          ))}
        </div>
      )}
    </section>
  );
}

const LogChunk = memo(
  function LogChunk({
    lines,
    from,
    count,
    shown,
  }: {
    lines: Line[];
    from: number;
    count: number;
    filterKey: string;
    shown: ReadonlySet<Provenance>;
  }) {
    const rows = [];
    for (let at = from; at < from + count; at++) {
      const line = lines[at];
      if (!line || (line.provenance && !shown.has(line.provenance))) continue;
      rows.push(
        <div key={at} className={`line provenance-${line.provenance ?? "none"}`}>
          <span className="line-clock">{line.clock}</span>
          <span className="line-tag">{line.provenance ?? ""}</span>
          <span className="line-text">{line.text}</span>
        </div>,
      );
    }
    return <div className="log-chunk">{rows}</div>;
  },
  (before, after) =>
    before.lines === after.lines &&
    before.from === after.from &&
    before.count === after.count &&
    before.filterKey === after.filterKey,
);

function ResizeHandle({
  height,
  onResize,
}: {
  height: number;
  onResize: (height: number) => void;
}) {
  const clamp = (value: number) =>
    Math.round(Math.min(window.innerHeight * 0.75, Math.max(120, value)));
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    const startY = event.clientY;
    const startHeight = height;
    event.currentTarget.setPointerCapture(event.pointerId);
    const onMove = (move: globalThis.PointerEvent) =>
      onResize(clamp(startHeight + startY - move.clientY));
    const target = event.currentTarget;
    const onUp = () => {
      target.removeEventListener("pointermove", onMove);
      target.removeEventListener("pointerup", onUp);
    };
    target.addEventListener("pointermove", onMove);
    target.addEventListener("pointerup", onUp);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "ArrowUp") onResize(clamp(height + 24));
    else if (event.key === "ArrowDown") onResize(clamp(height - 24));
    else return;
    event.preventDefault();
  };
  return (
    // biome-ignore lint/a11y/useSemanticElements: un separador que se arrastra no tiene elemento nativo
    <div
      className="resize-handle"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Altura del registro"
      aria-valuenow={height}
      aria-valuemin={120}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
    />
  );
}
