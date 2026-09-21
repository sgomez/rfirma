import type { Answer } from "../contract/Answer";
import type { ClientChoice } from "../contract/ClientChoice";
import type { Comparison } from "../contract/Comparison";
import type { DeducedCoordinates } from "../contract/DeducedCoordinates";
import type { NewReport } from "../contract/NewReport";
import type { ReportChoice } from "../contract/ReportChoice";
import type { ReportView } from "../contract/ReportView";
import type { Request } from "../contract/Request";
import type { Validation } from "../contract/Validation";

export interface EventStream {
  addEventListener(type: string, listener: (event: Event) => void): void;
  close(): void;
}

/** Lo que la consola necesita de un servidor: peticiones sueltas y un flujo de eventos. */
export interface Wire {
  fetch(url: string, init?: RequestInit): Promise<Response>;
  events(url: string): EventStream;
}

export const browserWire: Wire = {
  fetch: (url, init) => window.fetch(url, init),
  events: (url) => new EventSource(url),
};

type Params = Record<string, string | undefined>;

async function unwrap<T>(response: Response): Promise<T> {
  const payload = (await response.json()) as { ok?: T; error?: string };
  if (!response.ok || payload.error !== undefined) {
    throw new Error(payload.error ?? `la suite respondió ${response.status}`);
  }
  return payload.ok as T;
}

/** El contrato HTTP de la suite, tipado con las formas que genera Rust. */
export function suiteOver(wire: Wire, token: string) {
  const url = (path: string, params: Params = {}) => {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) query.set(key, value);
    }
    query.set("token", token);
    return `${path}?${query}`;
  };
  const get = async <T>(path: string, params?: Params) =>
    unwrap<T>(await wire.fetch(url(path, params)));
  const text = async (path: string, params: Params) => {
    const response = await wire.fetch(url(path, params));
    if (!response.ok) return unwrap<string>(response);
    return response.text();
  };
  const post = async <T = null>(path: string, body: unknown) =>
    unwrap<T>(
      await wire.fetch(url(path), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    );

  return {
    page: (path: string, params?: Params) => url(path, params),
    events: () => wire.events(url("/api/events")),
    defaults: () => get<DeducedCoordinates>("/api/defaults"),
    log: (check: string, report?: string) => text("/api/log", { check, report }),
    transcript: (check: string, report?: string) => text("/api/transcript", { check, report }),
    reportView: (report: string) => get<ReportView>("/api/report-view", { report }),
    references: () => get<string[]>("/api/references"),
    compare: (a: string, b: string) => get<Comparison>("/api/compare", { a, b }),
    validate: (report: string, reference: string) =>
      get<Validation>("/api/validate", { report, reference }),
    chooseClient: (choice: ClientChoice) => post("/api/client", choice),
    createReport: (report: NewReport) => post("/api/reports", report),
    openReport: (name: string) => post("/api/report", { name } satisfies ReportChoice),
    run: (request: Request) => post("/api/run", request),
    stop: () => post("/api/stop", {}),
    skip: () => post("/api/skip", {}),
    answer: (answer: string | null) => post("/api/answer", { answer } satisfies Answer),
  };
}

export type Suite = ReturnType<typeof suiteOver>;
