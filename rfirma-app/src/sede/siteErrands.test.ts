import { describe, expect, it, vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import type { TokenFailure } from "../signing/token";
import type { Errand } from "./errand";
import type { DescribedDocument, SiteCommands, SiteErrandView } from "./siteErrands";
import { errandOf, siteErrands } from "./siteErrands";

/**
 * Grada A: el adaptador del puerto, **contra las órdenes dobladas** (TD-78).
 *
 * Lo que se comprueba es lo único que aquí se decide: que `watch` se suscribe
 * una vez y se desuscribe, y que cada momento que llega se convierte en el
 * `Errand` que `SedeWindow` espera. La ventana ya está probada por su puerto en
 * `SedeWindow.test.tsx`, contra el doble, y esas pruebas no se tocan.
 */

function certificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "handle-1",
    label: "FNMT",
    holderName: "ADA LOVELACE BYRON",
    idNumber: "99999999R",
    issuer: "FNMT-RCM",
    store: "installed",
    status: { kind: "valid", notAfter: 4_102_444_800 },
    remembered: false,
    ...overrides,
  };
}

const described: DescribedDocument = { title: "Solicitud", pages: 3, sizeBytes: 4096 };

/** Las once órdenes, dobladas, y el asa para empujar momentos por el evento. */
function doubled(overrides: Partial<SiteCommands> = {}) {
  const stop = vi.fn();
  let emit: ((view: SiteErrandView) => void) | null = null;
  const calls = {
    watch: vi.fn(),
    readErrand: vi.fn(),
    identify: vi.fn(),
    decline: vi.fn(),
    beginSigning: vi.fn(),
    signWithPin: vi.fn(),
    finishSigning: vi.fn(),
    installCertificate: vi.fn(),
    lookAgain: vi.fn(),
    installLocalCa: vi.fn(),
    closeWindow: vi.fn(),
    describeDocument: vi.fn(),
  };
  const commands: SiteCommands = {
    watch: (onView) => {
      calls.watch();
      emit = onView;
      return stop;
    },
    readErrand: async () => {
      calls.readErrand();
      return null;
    },
    identify: async (id) => {
      calls.identify(id);
      return { ok: true, value: undefined };
    },
    decline: async () => calls.decline(),
    beginSigning: async (id) => {
      calls.beginSigning(id);
      return { ok: true, value: { kind: "typedOnScreen", attemptsLeft: null } };
    },
    signWithPin: async (secret) => {
      calls.signWithPin(secret);
      return { ok: true, value: undefined };
    },
    finishSigning: async () => {
      calls.finishSigning();
      return { ok: true, value: undefined };
    },
    installCertificate: async () => {
      calls.installCertificate();
      return true;
    },
    lookAgain: async () => calls.lookAgain(),
    installLocalCa: async () => calls.installLocalCa(),
    closeWindow: async () => calls.closeWindow(),
    describeDocument: async (id) => {
      calls.describeDocument(id);
      return described;
    },
    ...overrides,
  };

  return { commands, calls, stop, push: (view: SiteErrandView) => emit?.(view) };
}

/** El puerto ya suscrito, con la lista de trámites que ha ido publicando. */
function watched(overrides: Partial<SiteCommands> = {}) {
  const world = doubled(overrides);
  const port = siteErrands(world.commands);
  const seen: (Errand | null)[] = [];
  const unwatch = port.watch((errand) => seen.push(errand));
  return { ...world, port, seen, unwatch, last: () => seen[seen.length - 1] };
}

const ASKING_TO_SIGN: SiteErrandView = {
  origin: "sede.ejemplo.gob.es",
  stage: {
    kind: "askingToSign",
    document: "asa-opaca-1",
    round: "cosign",
    certificates: [certificate()],
    unregisteredSignatures: true,
  },
};

describe("la suscripción al trámite", () => {
  it("subscribes once and unsubscribes on teardown", () => {
    const { calls, stop, unwatch } = watched();

    expect(calls.watch).toHaveBeenCalledOnce();
    expect(stop).not.toHaveBeenCalled();

    unwatch();

    expect(stop).toHaveBeenCalledOnce();
    expect(calls.watch).toHaveBeenCalledOnce();
  });

  it("stops publishing after teardown", async () => {
    const { push, seen, unwatch } = watched();

    unwatch();
    push({ origin: null, stage: { kind: "waiting" } });
    await vi.waitFor(() => expect(seen).toHaveLength(0));
  });

  /*
   * La regresión de la ventana en negro: el backend publica el primer momento
   * nada más abrir la ventana, y para entonces el frontal todavía no ha
   * registrado el `listen`. Ese momento no llega nunca por el evento, y sin
   * momento `SedeWindow` no pinta nada.
   */
  it("takes the errand that was published before anyone was listening", async () => {
    const { seen } = watched({
      // El evento no trae nada: quien lo emitió lo hizo antes de esto.
      watch: () => () => {},
      readErrand: async () => ({ origin: null, stage: { kind: "waiting" } }),
    });

    await vi.waitFor(() => expect(seen).toHaveLength(1));
    expect(seen[0]?.stage.kind).toBe("waiting");
  });

  it("asks for the errand only after the listener is in place", async () => {
    const order: string[] = [];
    watched({
      watch: () => {
        order.push("watch");
        return () => {};
      },
      readErrand: async () => {
        order.push("readErrand");
        return null;
      },
    });

    await vi.waitFor(() => expect(order).toEqual(["watch", "readErrand"]));
  });

  /*
   * El guardado es, por definición, el mismo momento o uno más viejo: si
   * mientras se leía ha entrado uno por el evento, repintar el guardado
   * encima retrocedería el trámite.
   */
  it("lets a moment that arrived by event win over the stored one", async () => {
    const { seen, push } = watched({
      watch: (onView) => {
        // Un momento entra por el evento antes de que la lectura resuelva.
        queueMicrotask(() => onView(ASKING_TO_SIGN));
        return () => {};
      },
      readErrand: async () => ({ origin: null, stage: { kind: "waiting" } }),
    });

    await vi.waitFor(() => expect(seen.length).toBeGreaterThan(0));
    expect(seen.map((errand) => errand?.stage.kind)).not.toContain("waiting");
    expect(push).toBeDefined();
  });
});

describe("cada momento que llega se convierte en lo que la ventana espera", () => {
  it("turns the waiting moment into an errand with no origin yet", () => {
    expect(errandOf({ origin: null, stage: { kind: "waiting" } })).toEqual<Errand>({
      origin: null,
      operation: "sign",
      stage: { kind: "waiting" },
    });
  });

  it("turns an identity request into a consent with no document", () => {
    const view: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "askingForConsent", certificates: [certificate()] },
    };

    expect(errandOf(view)).toEqual<Errand>({
      origin: "sede.ejemplo.gob.es",
      operation: "selectcert",
      stage: {
        kind: "consent",
        document: null,
        certificates: [certificate()],
        narrowed: false,
      },
    });
  });

  it("turns a dead end into the repair moment", () => {
    const view: SiteErrandView = {
      origin: null,
      stage: { kind: "noChannel", reason: "localCaMissing" },
    };

    expect(errandOf(view).stage).toEqual({ kind: "noChannel", reason: "localCaMissing" });
  });

  it("names a refusal the catalogue does not know as unknown", () => {
    const view: SiteErrandView = {
      origin: null,
      stage: {
        kind: "outcome",
        outcome: { kind: "refused", situation: "siteErrandNotLive", detail: "CRUDO" },
      },
    };

    expect(errandOf(view).stage).toEqual({
      kind: "outcome",
      outcome: { kind: "refused", situation: "unknown", detail: "CRUDO" },
    });
  });

  it("keeps a refusal the catalogue does know", () => {
    const view: SiteErrandView = {
      origin: null,
      stage: {
        kind: "outcome",
        outcome: { kind: "refused", situation: "unsupportedFilter", detail: "CRUDO" },
      },
    };

    expect(errandOf(view).stage).toMatchObject({
      outcome: { situation: "unsupportedFilter" },
    });
  });

  it("turns no usable certificate into its own moment", () => {
    const view: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "noCertificate", reason: "excluded", owned: 2 },
    };

    expect(errandOf(view).stage).toEqual({ kind: "noCertificate", reason: "excluded", owned: 2 });
  });

  it("reads the document of a signature request by its opaque handle", async () => {
    const { push, calls, last } = watched();

    push(ASKING_TO_SIGN);

    await vi.waitFor(() =>
      expect(last()).toEqual<Errand>({
        origin: "sede.ejemplo.gob.es",
        operation: "sign",
        stage: {
          kind: "consent",
          document: {
            title: "Solicitud",
            pages: 3,
            sizeBytes: 4096,
            signatures: 1,
            hasUnregisteredSignatures: true,
          },
          certificates: [certificate()],
          narrowed: false,
        },
      }),
    );
    expect(calls.describeDocument).toHaveBeenCalledWith("asa-opaca-1");
  });

  it("consents without a card when the document cannot be read", async () => {
    const { push, last } = watched({ describeDocument: async () => null });

    push(ASKING_TO_SIGN);

    await vi.waitFor(() => expect(last()?.stage).toMatchObject({ document: null }));
  });
});

describe("un momento del backend gana a lo que estuviera en vuelo", () => {
  /** Una promesa que se resuelve cuando la prueba quiera. */
  function deferred<T>() {
    let release: (value: T) => void = () => {};
    const promise = new Promise<T>((resolve) => {
      release = resolve;
    });
    return { promise, release: (value: T) => release(value) };
  }

  it("drops a slow document description overtaken by a later moment", async () => {
    const slow = deferred<DescribedDocument>();
    const { push, last, seen } = watched({ describeDocument: async () => slow.promise });

    push(ASKING_TO_SIGN);
    // El trámite termina mientras el documento se está leyendo: lo que la
    // ventana tiene delante ya no es el consentimiento.
    push({ origin: null, stage: { kind: "waiting" } });
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("waiting"));

    slow.release(described);
    await Promise.resolve();

    expect(seen.map((errand) => errand?.stage.kind)).toEqual(["waiting"]);
  });

  it("drops the local moment of a consent overtaken while the backend answered", async () => {
    const slow = deferred<{ ok: true; value: { kind: "typedOnScreen"; attemptsLeft: null } }>();
    const { push, port, last, seen } = watched({ beginSigning: async () => slow.promise });

    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    const consenting = port.consent("handle-1");
    push({ origin: null, stage: { kind: "waiting" } });
    slow.release({ ok: true, value: { kind: "typedOnScreen", attemptsLeft: null } });
    await consenting;

    expect(last()?.stage.kind).toBe("waiting");
    expect(seen.map((errand) => errand?.stage.kind)).toEqual(["consent", "signing", "waiting"]);
  });
});

describe("los momentos que pone el adaptador", () => {
  it("walks from consent to the secret, the two signing legs and the outcome", async () => {
    const { push, port, seen, calls, last } = watched();
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(calls.beginSigning).toHaveBeenCalledWith("handle-1");
    expect(last()?.stage).toEqual({ kind: "secret", certificate: certificate(), failure: null });

    await port.submitSecret("1234");

    expect(calls.signWithPin).toHaveBeenCalledWith("1234");
    expect(calls.finishSigning).toHaveBeenCalledOnce();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: {
        kind: "signed",
        document: {
          title: "Solicitud",
          pages: 3,
          sizeBytes: 4096,
          signatures: 1,
          hasUnregisteredSignatures: true,
        },
      },
    });
    expect(seen.map((errand) => errand?.stage.kind)).toEqual([
      "consent",
      "signing",
      "secret",
      "signing",
      "signing",
      "outcome",
    ]);
  });

  it("signs with an empty secret when the store asks for none", async () => {
    const { push, port, calls, last } = watched({
      beginSigning: async () => ({ ok: true, value: { kind: "notNeeded" } }),
    });
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(calls.signWithPin).toHaveBeenCalledWith("");
    expect(last()?.stage).toMatchObject({ outcome: { kind: "signed" } });
  });

  it("keeps an incorrect pin inside the dialog", async () => {
    const failure: TokenFailure = {
      situation: "incorrectPin",
      detail: "CKR_PIN_INCORRECT",
      attemptsLeft: null,
    };
    const { push, port, last } = watched({
      signWithPin: async () => ({ ok: false, failure }),
    });
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));
    await port.consent("handle-1");

    await port.submitSecret("0000");

    expect(last()?.stage).toEqual({ kind: "secret", certificate: certificate(), failure });
  });

  it("ends the errand when a signing stage fails for anything else", async () => {
    const { push, port, last } = watched({
      finishSigning: async () => ({
        ok: false,
        failure: { situation: "unknown", detail: "el puente no contesta", attemptsLeft: null },
      }),
    });
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));
    await port.consent("handle-1");

    await port.submitSecret("1234");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "refused", situation: "unknown", detail: "el puente no contesta" },
    });
  });

  it("hands the identity over without ever signing", async () => {
    const { push, port, calls, last } = watched();
    push({
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "askingForConsent", certificates: [certificate()] },
    });

    await port.consent("handle-1");

    expect(calls.identify).toHaveBeenCalledWith("handle-1");
    expect(calls.beginSigning).not.toHaveBeenCalled();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "signed", document: null },
    });
  });

  it("declines and shows the cancelled outcome when there was something to answer", async () => {
    const { push, port, calls, last } = watched();
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.cancel();

    expect(calls.decline).toHaveBeenCalledOnce();
    expect(calls.closeWindow).not.toHaveBeenCalled();
    expect(last()?.stage).toMatchObject({ outcome: { kind: "cancelled" } });
  });

  it("declines and leaves when there was nothing to answer", async () => {
    const { push, port, calls } = watched();
    push({ origin: null, stage: { kind: "noChannel", reason: "channelNotOpened" } });

    await port.cancel();

    expect(calls.decline).toHaveBeenCalledOnce();
    expect(calls.closeWindow).toHaveBeenCalledOnce();
  });
});

describe("las salidas de la pantalla sin certificado", () => {
  it("looks again after installing one", async () => {
    const { port, calls } = watched();

    await port.installCertificate();

    expect(calls.installCertificate).toHaveBeenCalledOnce();
    expect(calls.lookAgain).toHaveBeenCalledOnce();
  });

  it("leaves the screen as it was when the dialog is dismissed", async () => {
    const { port, calls } = watched({ installCertificate: async () => false });

    await port.installCertificate();

    expect(calls.lookAgain).not.toHaveBeenCalled();
  });

  it("passes the remaining orders straight through", async () => {
    const { port, calls } = watched();

    await port.lookAgain();
    await port.installLocalCa();
    await port.close();

    expect(calls.lookAgain).toHaveBeenCalledOnce();
    expect(calls.installLocalCa).toHaveBeenCalledOnce();
    expect(calls.closeWindow).toHaveBeenCalledOnce();
  });
});
