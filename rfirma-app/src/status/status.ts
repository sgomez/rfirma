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

/** Candidata a firmar en sedes, para el desplegable de la señal `Firma en sedes`. */
export interface SiteSignatureCandidate {
  id: string;
  name: string;
  selected: boolean;
}

export interface SignalRow {
  signal: Signal;
  value: string;
  verdict: Verdict;
  action: StatusAction | null;
  detail: StoreDetail[] | null;
  candidates: SiteSignatureCandidate[] | null;
  restartFirefoxNotice: boolean;
}

export interface StatusPort {
  /** Lee el estado actual de las señales de la instalación. */
  readStatus(): Promise<SignalRow[]>;
  /** Vuelve a comprobar el estado remidiendo contra los orígenes. */
  recheck(): Promise<SignalRow[]>;
  /** Instala el certificado de rFirma donde falte y vuelve a medir su señal. */
  installLocalCaCertificate(): Promise<SignalRow>;
  /**
   * Elige quién abre las sedes; si es rFirma, instala también su certificado
   * (ID-366). Devuelve las dos filas que la elección vuelve a medir.
   */
  chooseSiteSignatureHandler(handlerId: string): Promise<SignalRow[]>;
}

/**
 * Si el triángulo de aviso del botón de menú debe encenderse: regla propia,
 * no «alguna fila en Atención» (docs/design/cabecera.md, sección «El aviso»).
 * Solo enciende el certificado de rFirma ausente o a medias, y `Sin
 * configurar` en Firma en sedes; que las sedes abran otro programa es una
 * elección legítima, no una avería.
 */
export function hasMenuAttention(rows: SignalRow[]): boolean {
  return rows.some((row) => {
    if (row.signal === "localCaCertificate") {
      return row.verdict === "attention" || row.verdict === "incorrect";
    }
    if (row.signal === "siteSignature") {
      return row.verdict === "attention" && row.value === "";
    }
    return false;
  });
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
      candidates: null,
      restartFirefoxNotice: false,
    },
  ],
  recheckRows?: SignalRow[],
  installedRow?: SignalRow,
  chosenRows?: SignalRow[],
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
    installLocalCaCertificate: async () => {
      const installed = installedRow ?? {
        signal: "localCaCertificate",
        value: "",
        verdict: "checking",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      };
      rows = rows.map((row) => (row.signal === installed.signal ? installed : row));
      return installed;
    },
    chooseSiteSignatureHandler: async () => {
      const chosen = chosenRows ?? [];
      rows = rows.map((row) => chosen.find((updated) => updated.signal === row.signal) ?? row);
      return chosen;
    },
  };
}
