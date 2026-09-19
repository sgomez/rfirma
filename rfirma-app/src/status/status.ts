/**
 * Puerto de consulta y medición de señales del panel de estado.
 */

export type Signal = "version" | "siteSignature" | "localCaCertificate" | "userCertificates";

export type Verdict = "correct" | "attention" | "incorrect" | "notApplicable" | "checking";

export type ActionKind = "repair" | "choice" | "link";

export type StoreBrand = "firefox" | "chrome" | "nssdb";

export interface StoreDetail {
  brand: StoreBrand;
  trusted: boolean;
}

export interface StatusAction {
  kind: ActionKind;
  target: string;
}

export interface SignalRow {
  signal: Signal;
  value: string;
  verdict: Verdict;
  action: StatusAction | null;
  detail: StoreDetail[] | null;
}

export interface StatusPort {
  /** Lee el estado actual de las señales de la instalación. */
  readStatus(): Promise<SignalRow[]>;
  /** Vuelve a comprobar el estado remidiendo contra los orígenes. */
  recheck(): Promise<SignalRow[]>;
}

/** Doble en memoria para pruebas de la interfaz. */
export function memoryStatus(
  initialRows: SignalRow[] = [
    {
      signal: "version",
      value: "0.4.1",
      verdict: "correct",
      action: null,
      detail: null,
    },
  ],
  recheckRows?: SignalRow[],
): StatusPort {
  let rows = [...initialRows];
  return {
    readStatus: async () => rows,
    recheck: async () => {
      if (recheckRows) {
        rows = [...recheckRows];
      }
      return rows;
    },
  };
}
