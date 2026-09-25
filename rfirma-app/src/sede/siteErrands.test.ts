import { describe, expect, it, vi } from "vitest";
import type { TokenFailure } from "../signing/token";
import type { DescribedDocument, SiteErrandView } from "./siteErrands";
import {
  ASKING_TO_CONFIRM,
  ASKING_TO_SIGN,
  certificate,
  described,
  watched,
} from "./siteErrandsFixtures";
import type { PortalResult } from "./siteErrandView";

/**
 * Grada A: el adaptador del puerto, **contra las órdenes dobladas** (TD-78).
 *
 * Lo que se comprueba es lo único que aquí se decide: que `watch` se suscribe
 * una vez y se desuscribe, y que cada consentimiento y desenlace pasa por las
 * órdenes correctas. La conversión pura del momento del backend tiene sus
 * propias pruebas en `errandConversion.test.ts`, y el lote en
 * `siteErrands.batch.test.ts`. La ventana ya está probada por su puerto en
 * `SedeWindow.test.tsx`, contra el doble, y esas pruebas no se tocan.
 */

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

describe("el diálogo del portal sale solo", () => {
  const SAVING: SiteErrandView = {
    origin: "sede.ejemplo.gob.es",
    stage: { kind: "saving", filename: "firma.pdf" },
  };
  const LOADING: SiteErrandView = {
    origin: "sede.ejemplo.gob.es",
    stage: { kind: "loading", multiple: true },
  };

  it("opens the save dialog as soon as the saving moment arrives", async () => {
    const { push, calls, last } = watched();

    push(SAVING);

    expect(last()?.stage).toEqual({ kind: "saving", filename: "firma.pdf" });
    await vi.waitFor(() => expect(calls.saveFile).toHaveBeenCalledTimes(1));
    expect(calls.loadFiles).not.toHaveBeenCalled();
  });

  it("opens the load dialog as soon as the loading moment arrives", async () => {
    const { push, calls, last } = watched();

    push(LOADING);

    expect(last()?.stage).toEqual({ kind: "loading", multiple: true });
    await vi.waitFor(() => expect(calls.loadFiles).toHaveBeenCalledTimes(1));
    expect(calls.saveFile).not.toHaveBeenCalled();
  });

  it("ends the errand when the portal order fails", async () => {
    const { push, last } = watched({
      saveFile: async () => ({
        ok: false,
        failure: { situation: "unknown", detail: "el portal no contesta", attemptsLeft: null },
      }),
    });

    push(SAVING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({
        kind: "outcome",
        outcome: { kind: "refused", situation: "unknown", detail: "el portal no contesta" },
      }),
    );
  });

  it("warns and opens the save dialog again when the chosen destination cannot be written", async () => {
    const saveFile = vi
      .fn<() => Promise<PortalResult<boolean>>>()
      .mockResolvedValueOnce({
        ok: false,
        failure: { situation: "saveDestinationUnwritable", detail: "permiso denegado" },
      })
      .mockResolvedValueOnce({ ok: true, value: true });
    const { push, seen, last } = watched({ saveFile });

    push(SAVING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({ kind: "outcome", outcome: { kind: "saved" } }),
    );
    expect(saveFile).toHaveBeenCalledTimes(2);
    expect(seen.map((errand) => errand?.stage)).toContainEqual({
      kind: "saving",
      filename: "firma.pdf",
      unwritable: true,
    });
  });

  it("shows the saved outcome once the file is written", async () => {
    const { push, last } = watched({ saveFile: async () => ({ ok: true, value: true }) });

    push(SAVING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({ kind: "outcome", outcome: { kind: "saved" } }),
    );
  });

  it("shows nothing locally when the save was the tail of a signandsave already shown as signed", async () => {
    const { push, last } = watched({ saveFile: async () => ({ ok: true, value: false }) });

    push(SAVING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({ kind: "saving", filename: "firma.pdf" }),
    );
  });

  it("shows the loaded outcome with how many files were delivered", async () => {
    const { push, last } = watched({ loadFiles: async () => ({ ok: true, value: 2 }) });

    push(LOADING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({ kind: "outcome", outcome: { kind: "loaded", fileCount: 2 } }),
    );
  });

  it("shows nothing locally when the load continues the errand with one more step", async () => {
    const { push, last } = watched({ loadFiles: async () => ({ ok: true, value: null }) });

    push(LOADING);

    await vi.waitFor(() => expect(last()?.stage).toEqual({ kind: "loading", multiple: true }));
  });

  it("classifies a cancelled save as its own refusal situation", async () => {
    const { push, last } = watched({
      saveFile: async () => ({
        ok: false,
        failure: {
          situation: "saveCancelled",
          detail: "el dialogo de guardado se cerro sin elegir nada",
          attemptsLeft: null,
        },
      }),
    });

    push(SAVING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: "saveCancelled",
          detail: "el dialogo de guardado se cerro sin elegir nada",
        },
      }),
    );
  });

  it("classifies a cannotLoadData failure as its own refusal situation", async () => {
    const { push, last } = watched({
      loadFiles: async () => ({
        ok: false,
        failure: { situation: "cannotLoadData", detail: "no such file", attemptsLeft: null },
      }),
    });

    push(LOADING);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({
        kind: "outcome",
        outcome: { kind: "refused", situation: "cannotLoadData", detail: "no such file" },
      }),
    );
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
    const slow = deferred<{ ok: true; value: { kind: "typedOnScreen" } }>();
    const { push, port, last, seen } = watched({ beginSigning: async () => slow.promise });

    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    const consenting = port.consent("handle-1");
    push({ origin: null, stage: { kind: "waiting" } });
    slow.release({ ok: true, value: { kind: "typedOnScreen" } });
    await consenting;

    expect(last()?.stage.kind).toBe("waiting");
    expect(seen.map((errand) => errand?.stage.kind)).toEqual(["consent", "signing", "waiting"]);
  });
});

describe("los momentos que pone el adaptador", () => {
  it("walks from consent to the two signing legs and the outcome", async () => {
    const { push, port, seen, calls, last } = watched();
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(calls.beginSigning).toHaveBeenCalledWith("handle-1");
    expect(calls.signWithPin).toHaveBeenCalledWith("");
    expect(calls.finishSigning).toHaveBeenCalledOnce();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: {
        kind: "signed",
        document: {
          title: "Solicitud",
          pages: 3,
          sizeBytes: 4096,
          round: { kind: "cosign" },
          hasUnregisteredSignatures: true,
        },
      },
    });
    expect(seen.map((errand) => errand?.stage.kind)).toEqual([
      "consent",
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

  it("ends the errand when signing fails", async () => {
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

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "refused", situation: "incorrectPin", detail: "CKR_PIN_INCORRECT" },
    });
  });

  it.each([
    "triphaseServerUrlMissing",
    "triphaseServerException",
    "triphaseServerUnreachable",
    "triphaseServerUnexpectedAnswer",
  ] as const)("names the triphase server failure %s as its own refusal", async (situation) => {
    const { push, port, last } = watched({
      signWithPin: async () => ({
        ok: false,
        failure: { situation, detail: "CRUDO", attemptsLeft: null },
      }),
    });
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));
    await port.consent("handle-1");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "refused", situation, detail: "CRUDO" },
    });
  });

  it("shows the cancelled outcome when the person gives no password for the PDF", async () => {
    const { push, port, calls, last } = watched({
      beginSigning: async () => ({
        ok: false,
        failure: {
          situation: "userCancelled" as TokenFailure["situation"],
          detail: "la persona no ha dado la contraseña del PDF",
          attemptsLeft: null,
        },
      }),
    });
    push(ASKING_TO_SIGN);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));
    await port.consent("handle-1");

    expect(calls.signWithPin).not.toHaveBeenCalled();
    expect(last()?.stage).toMatchObject({ outcome: { kind: "cancelled" } });
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

  it("hands the confirmation on and waits for the backend to publish what follows", async () => {
    const { push, port, calls, last } = watched();
    push(ASKING_TO_CONFIRM);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("confirming"));

    await port.confirmSignatures();

    expect(calls.confirmSignatures).toHaveBeenCalledOnce();
    expect(last()?.stage.kind).toBe("confirming");
  });

  it("drops a second confirmation that lost the race to the moment the backend published", async () => {
    let attempt = 0;
    let publish: (view: SiteErrandView) => void = () => {};
    const world = watched({
      confirmSignatures: async () => {
        attempt += 1;
        if (attempt === 1) {
          await Promise.resolve();
          publish(ASKING_TO_SIGN);
          return { ok: true, value: undefined };
        }
        return {
          ok: false,
          failure: { situation: "unknown", detail: "no hay tramite vivo", attemptsLeft: null },
        };
      },
    });
    publish = world.push;
    world.push(ASKING_TO_CONFIRM);
    await vi.waitFor(() => expect(world.last()?.stage.kind).toBe("confirming"));

    await Promise.all([world.port.confirmSignatures(), world.port.confirmSignatures()]);

    await vi.waitFor(() => expect(world.last()?.stage.kind).toBe("consent"));
  });

  it("refuses the errand when the repeated validation cannot be asked for", async () => {
    const { push, port, last } = watched({
      confirmSignatures: async () => ({
        ok: false,
        failure: { situation: "unknown", detail: "no hay tramite vivo", attemptsLeft: null },
      }),
    });
    push(ASKING_TO_CONFIRM);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("confirming"));

    await port.confirmSignatures();

    expect(last()?.stage).toMatchObject({ kind: "outcome", outcome: { kind: "refused" } });
  });

  it("declines and shows the cancelled outcome when the confirmation is refused", async () => {
    const { push, port, calls, last } = watched();
    push(ASKING_TO_CONFIRM);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("confirming"));

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

describe("la selección automática que pide la sede", () => {
  const withTheOnlyOne = (withoutAsking: boolean): SiteErrandView => ({
    origin: "sede.ejemplo.gob.es",
    stage: {
      kind: "askingToSign",
      document: "asa-opaca-1",
      signing: "pdf",
      round: { kind: "sign" },
      certificates: [certificate()],
      unregisteredSignatures: false,
      alreadyChosen: "handle-1",
      withoutAsking,
    },
  });

  it("consents alone with the only candidate when the backend says so", async () => {
    const { push, calls, last } = watched();
    push(withTheOnlyOne(true));

    await vi.waitFor(() => expect(last()?.stage).toMatchObject({ outcome: { kind: "signed" } }));
    expect(calls.beginSigning).toHaveBeenCalledWith("handle-1");
  });

  it("waits for the person when the only candidate is merely preselected", async () => {
    const { push, calls, last } = watched();
    push(withTheOnlyOne(false));

    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));
    expect(calls.beginSigning).not.toHaveBeenCalled();
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
    await port.dismissWarning();

    expect(calls.lookAgain).toHaveBeenCalledOnce();
    expect(calls.installLocalCa).toHaveBeenCalledOnce();
    expect(calls.closeWindow).toHaveBeenCalledOnce();
    expect(calls.dismissWarning).toHaveBeenCalledOnce();
  });
});
