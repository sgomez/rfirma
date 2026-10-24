import type { CatalogueSet } from "../contract/CatalogueSet";
import type { Comparison } from "../contract/Comparison";
import type { DeducedCoordinates } from "../contract/DeducedCoordinates";
import type { LiveLine } from "../contract/LiveLine";
import type { LogLine } from "../contract/LogLine";
import type { ReportView } from "../contract/ReportView";
import type { SelectionResolution } from "../contract/SelectionResolution";
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
  logs = new Map<string, LogLine[]>();
  comparison: Comparison | null = null;
  sharingAnErrand: string[] = [];
  deduced: DeducedCoordinates = {
    name: "autofirma-1.9.2-2026-10-24",
    title: "autofirma 1.9.2",
    os: "Linux",
    os_version: "6.8.0",
    client_version: "1.9.2",
  };

  catalogue: CatalogueSet[] = [
    { name: "saludo", checks: ["greeting_opens_the_channel", "greeting_echoes"] },
    {
      name: "errores",
      checks: [
        "unsupported_protocol_uri_rejected",
        "unknown_operation_rejected",
        "empty_uri_rejected",
      ],
    },
  ];

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

  private resolve(text: string): SelectionResolution {
    const resolution: SelectionResolution = {
      ids: [],
      greetings_added: [],
      unknown: [],
      out_of_scope: [],
      sharing_an_errand: [...this.sharingAnErrand],
    };
    for (const name of text.split(/[\s,]+/).filter(Boolean)) {
      const hit = this.catalogue.flatMap((set) =>
        set.name === name || set.name.startsWith(`${name}.`)
          ? set.checks
          : set.checks.filter((id) => id === name),
      );
      if (hit.length === 0) resolution.unknown.push(name);
      for (const id of hit) if (!resolution.ids.includes(id)) resolution.ids.push(id);
    }
    if (
      resolution.ids.some((id) => id === "greeting_echoes") &&
      !resolution.ids.includes("greeting_opens_the_channel")
    ) {
      resolution.ids.unshift("greeting_opens_the_channel");
      resolution.greetings_added.push("greeting_opens_the_channel");
    }
    return resolution;
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
      case "/api/catalogue":
        return reply(200, { ok: this.catalogue });
      case "/api/selection":
        return reply(200, { ok: this.resolve(params.text ?? "") });
      case "/api/defaults":
        return reply(200, { ok: this.deduced });
      case "/api/log":
        return reply(200, { ok: this.logs.get(params.check ?? "") ?? [] });
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
