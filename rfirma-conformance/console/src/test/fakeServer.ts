import type { Comparison } from "../contract/Comparison";
import type { LiveLine } from "../contract/LiveLine";
import type { ReportView } from "../contract/ReportView";
import type { Snapshot } from "../contract/Snapshot";
import type { SuiteFailure } from "../contract/SuiteFailure";
import type { EventStream, Wire } from "../suite/suite";

export interface Received {
  method: string;
  path: string;
  params: Record<string, string>;
  body: unknown;
}

class FakeStream implements EventStream {
  private readonly listeners = new Map<string, Set<(event: Event) => void>>();
  closed = false;

  addEventListener(type: string, listener: (event: Event) => void) {
    const set = this.listeners.get(type) ?? new Set();
    set.add(listener);
    this.listeners.set(type, set);
  }

  close() {
    this.closed = true;
  }

  emit(type: string, data: unknown) {
    if (this.closed) return;
    const event = new MessageEvent(type, { data: JSON.stringify(data) });
    for (const listener of this.listeners.get(type) ?? []) listener(event);
  }
}

/** El segundo adaptador del seam: responde las rutas de la suite desde memoria y apunta cada llamada. */
export class FakeServer implements Wire {
  readonly received: Received[] = [];
  readonly streams: FakeStream[] = [];
  snapshot: Snapshot;
  reportViews = new Map<string, ReportView>();
  logs = new Map<string, string>();
  comparison: Comparison | null = null;

  constructor(snapshot: Snapshot) {
    this.snapshot = snapshot;
  }

  fetch = async (url: string, init?: RequestInit): Promise<Response> => {
    const parsed = new URL(url, "http://127.0.0.1:47117");
    const params = Object.fromEntries(parsed.searchParams);
    if (params.token !== "t0k") return reply(403, { error: "falta el token de la consola" });
    const method = init?.method ?? "GET";
    const body = typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
    this.received.push({ method, path: parsed.pathname, params, body });
    return this.route(method, parsed.pathname, params);
  };

  events = (url: string): EventStream => {
    const stream = new FakeStream();
    this.streams.push(stream);
    if (new URL(url, "http://127.0.0.1:47117").searchParams.get("token") === "t0k") {
      queueMicrotask(() => stream.emit("state", this.snapshot));
    }
    return stream;
  };

  publish(snapshot: Snapshot) {
    this.snapshot = snapshot;
    for (const stream of this.streams) stream.emit("state", snapshot);
  }

  say(line: LiveLine) {
    for (const stream of this.streams) stream.emit("log", line);
  }

  fail(failure: SuiteFailure) {
    for (const stream of this.streams) stream.emit("suite_failure", failure);
  }

  posted(path: string): unknown[] {
    return this.received
      .filter((call) => call.method === "POST" && call.path === path)
      .map((call) => call.body);
  }

  private route(method: string, path: string, params: Record<string, string>): Response {
    if (method === "POST") return reply(200, { ok: null });
    switch (path) {
      case "/api/report-view": {
        const view = this.reportViews.get(params.report ?? "");
        return view
          ? reply(200, { ok: view })
          : reply(409, { error: `no existe el informe «${params.report}»` });
      }
      case "/api/log":
        return new Response(this.logs.get(params.check ?? "") ?? "", { status: 200 });
      case "/api/transcript":
        return new Response("", { status: 200 });
      case "/api/references":
        return reply(200, { ok: ["autofirma-1.9.2"] });
      case "/api/compare":
        return reply(200, { ok: this.comparison });
      default:
        return reply(404, { error: "no hay tal ruta" });
    }
  }
}

function reply(status: number, payload: unknown): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
