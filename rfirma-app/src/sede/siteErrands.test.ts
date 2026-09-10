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

/** Las órdenes, dobladas, y el asa para empujar momentos por el evento. */
function doubled(overrides: Partial<SiteCommands> = {}) {
  const stop = vi.fn();
  let emit: ((view: SiteErrandView) => void) | null = null;
  const calls = {
    watch: vi.fn(),
    readErrand: vi.fn(),
    identify: vi.fn(),
    confirmSignatures: vi.fn(),
    decline: vi.fn(),
    beginSigning: vi.fn(),
    signWithPin: vi.fn(),
    finishSigning: vi.fn(),
    saveFile: vi.fn(),
    loadFiles: vi.fn(),
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
    confirmSignatures: async () => {
      calls.confirmSignatures();
      return { ok: true, value: undefined };
    },
    decline: async () => calls.decline(),
    beginSigning: async (id) => {
      calls.beginSigning(id);
      return { ok: true, value: { kind: "typedOnScreen" } };
    },
    signWithPin: async (secret) => {
      calls.signWithPin(secret);
      return { ok: true, value: undefined };
    },
    finishSigning: async () => {
      calls.finishSigning();
      return { ok: true, value: undefined };
    },
    saveFile: async () => {
      calls.saveFile();
      return { ok: true, value: true };
    },
    loadFiles: async () => {
      calls.loadFiles();
      return { ok: true, value: 1 };
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
    signing: "pdf",
    round: { kind: "cosign" },
    certificates: [certificate()],
    unregisteredSignatures: true,
    alreadyChosen: null,
  },
};

const ASKING_TO_CONFIRM: SiteErrandView = {
  origin: "sede.ejemplo.gob.es",
  stage: { kind: "askingToConfirm", messageCode: "pdfShadowAttackSuspect" },
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
        signs: null,
        signing: null,
        items: null,
        certificates: [certificate()],
        narrowed: false,
      },
    });
  });

  it("turns a batch request into a consent with no document and its count of signatures", () => {
    const view: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: {
        kind: "askingToSignTheBatch",
        signs: 3,
        certificates: [certificate()],
        alreadyChosen: null,
      },
    };

    expect(errandOf(view)).toEqual<Errand>({
      origin: "sede.ejemplo.gob.es",
      operation: "sign",
      stage: {
        kind: "consent",
        document: null,
        signs: 3,
        signing: null,
        items: null,
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

  it("turns unreachable into the unreachable moment", () => {
    const view: SiteErrandView = {
      origin: null,
      stage: { kind: "unreachable" },
    };

    expect(errandOf(view).stage).toEqual({ kind: "unreachable" });
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

  it("turns a save request into the name the site proposed and never a path", () => {
    const view: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "saving", filename: "firma.pdf" },
    };

    expect(errandOf(view).stage).toEqual({ kind: "saving", filename: "firma.pdf" });
  });

  it("turns a save request without a proposed name into one with no name", () => {
    const view: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "saving", filename: null },
    };

    expect(errandOf(view).stage).toEqual({ kind: "saving", filename: null });
  });

  it("turns a load request into one file or several", () => {
    const one: SiteErrandView = {
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "loading", multiple: false },
    };
    const many: SiteErrandView = { ...one, stage: { kind: "loading", multiple: true } };

    expect(errandOf(one).stage).toEqual({ kind: "loading", multiple: false });
    expect(errandOf(many).stage).toEqual({ kind: "loading", multiple: true });
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
            round: { kind: "cosign" },
            hasUnregisteredSignatures: true,
          },
          signs: null,
          signing: "pdf",
          items: null,
          certificates: [certificate()],
          narrowed: false,
        },
      }),
    );
    expect(calls.describeDocument).toHaveBeenCalledWith("asa-opaca-1");
  });

  it("turns the confirmation the original asks for into the confirming stage", async () => {
    const view: SiteErrandView = {
      origin: "https://sede.example",
      stage: { kind: "askingToConfirm", messageCode: "ProtocolLauncher.65" },
    };
    const { push, last } = watched();

    push(view);

    await vi.waitFor(() =>
      expect(last()?.stage).toEqual({
        kind: "confirming",
        messageCode: "ProtocolLauncher.65",
      }),
    );
  });

  it.each(["pdf", "challenge", "xml", "invoice"] as const)(
    "carries the %s signing kind through to the consent stage",
    async (signing) => {
      const view: SiteErrandView = {
        ...ASKING_TO_SIGN,
        stage: {
          kind: "askingToSign",
          document: "asa-opaca-1",
          signing,
          round: { kind: "cosign" },
          certificates: [certificate()],
          unregisteredSignatures: true,
          alreadyChosen: null,
        },
      };
      const { push, last } = watched();

      push(view);

      await vi.waitFor(() => expect(last()?.stage).toMatchObject({ signing }));
    },
  );

  it.each(["tree", "leafs"] as const)(
    "carries the %s target of a countersignature through to the consent document",
    async (target) => {
      const view: SiteErrandView = {
        ...ASKING_TO_SIGN,
        stage: {
          kind: "askingToSign",
          document: "asa-opaca-1",
          signing: "pdf",
          round: { kind: "counter", target },
          certificates: [certificate()],
          unregisteredSignatures: false,
          alreadyChosen: null,
        },
      };
      const { push, last } = watched();

      push(view);

      await vi.waitFor(() =>
        expect(last()?.stage).toMatchObject({
          document: { round: { kind: "counter", target } },
        }),
      );
    },
  );

  it("consents without a card when the document cannot be read", async () => {
    const { push, last } = watched({ describeDocument: async () => null });

    push(ASKING_TO_SIGN);

    await vi.waitFor(() => expect(last()?.stage).toMatchObject({ document: null }));
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
      outcome: { kind: "refused", situation: "unknown", detail: "CKR_PIN_INCORRECT" },
    });
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

describe("el lote remoto", () => {
  const ASKING_TO_SIGN_THE_BATCH: SiteErrandView = {
    origin: "sede.ejemplo.gob.es",
    stage: {
      kind: "askingToSignTheBatch",
      signs: 3,
      certificates: [certificate()],
      alreadyChosen: null,
    },
  };

  it("closes the batch with the secret alone, without a postsign of its own", async () => {
    const { push, port, seen, calls, last } = watched();
    push(ASKING_TO_SIGN_THE_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(calls.signWithPin).toHaveBeenCalledWith("");
    expect(calls.finishSigning).not.toHaveBeenCalled();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "batchSigned", signs: 3 },
    });
    expect(seen.map((errand) => errand?.stage.kind)).toEqual(["consent", "signing", "outcome"]);
  });

  it("names the batch's own refusals as the catalogue knows them", async () => {
    const { push, port, last } = watched({
      signWithPin: async () => ({
        ok: false,
        failure: {
          situation: "presignerUnreachable",
          detail: "no se ha podido contactar con el presigner",
          attemptsLeft: null,
        },
      }),
    });
    push(ASKING_TO_SIGN_THE_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "batchPresignerUnreachable",
        detail: "no se ha podido contactar con el presigner",
      },
    });
  });

  it("ends the errand on a wrong pin instead of asking for it again", async () => {
    const { push, port, last } = watched({
      signWithPin: async () => ({
        ok: false,
        failure: { situation: "incorrectPin", detail: "el PIN no es correcto", attemptsLeft: 2 },
      }),
    });
    push(ASKING_TO_SIGN_THE_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "batchSigningFailed",
        detail: "el PIN no es correcto",
      },
    });
  });

  it("names a failed batch signature as such when the secret cannot even be asked for", async () => {
    const { push, port, last } = watched({
      beginSigning: async () => ({
        ok: false,
        failure: { situation: "tokenAbsent", detail: "no hay token", attemptsLeft: null },
      }),
    });
    push(ASKING_TO_SIGN_THE_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "refused", situation: "batchSigningFailed", detail: "no hay token" },
    });
  });
});

describe("el lote local", () => {
  const ASKING_TO_SIGN_THE_LOCAL_BATCH: SiteErrandView = {
    origin: "sede.ejemplo.gob.es",
    stage: {
      kind: "askingToSignTheLocalBatch",
      items: [
        { id: "001", signing: "pdf", round: { kind: "sign" } },
        { id: "002", signing: "challenge", round: { kind: "cosign" } },
        { id: "003", signing: "xml", round: { kind: "sign" } },
      ],
      certificates: [certificate()],
      alreadyChosen: null,
    },
  };

  it("turns the local batch moment into a consent with the summary of each element", () => {
    expect(errandOf(ASKING_TO_SIGN_THE_LOCAL_BATCH)).toEqual<Errand>({
      origin: "sede.ejemplo.gob.es",
      operation: "sign",
      stage: {
        kind: "consent",
        document: null,
        signs: 3,
        signing: null,
        items: [
          { id: "001", signing: "pdf", round: { kind: "sign" } },
          { id: "002", signing: "challenge", round: { kind: "cosign" } },
          { id: "003", signing: "xml", round: { kind: "sign" } },
        ],
        certificates: [certificate()],
        narrowed: false,
      },
    });
  });

  it("closes the local batch with the secret alone, without a postsign of its own", async () => {
    const { push, port, calls, last } = watched();
    push(ASKING_TO_SIGN_THE_LOCAL_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(calls.signWithPin).toHaveBeenCalledWith("");
    expect(calls.finishSigning).not.toHaveBeenCalled();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "batchSigned", signs: 3 },
    });
  });

  it("names the local batch's own refusals as the catalogue knows them", async () => {
    const { push, port, last } = watched({
      signWithPin: async () => ({
        ok: false,
        failure: { situation: "signingFailed", detail: "fallo al firmar", attemptsLeft: null },
      }),
    });
    push(ASKING_TO_SIGN_THE_LOCAL_BATCH);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("consent"));

    await port.consent("handle-1");

    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: {
        kind: "refused",
        situation: "batchSigningFailed",
        detail: "fallo al firmar",
      },
    });
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
