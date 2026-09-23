import { describe, expect, it, vi } from "vitest";
import type { Errand } from "./errand";
import { errandOf } from "./errandConversion";
import { ASKING_TO_SIGN, certificate, watched } from "./siteErrandsFixtures";
import type { SiteErrandView } from "./siteErrandView";

/**
 * Grada A: la conversión pura del momento del backend al `Errand` que la
 * ventana espera (TD-78). El puerto que la envuelve tiene sus propias
 * pruebas en `siteErrands.test.ts` y `siteErrands.batch.test.ts`.
 */

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

  it("turns the old web client warning into its own moment", () => {
    const view: SiteErrandView = {
      origin: null,
      stage: { kind: "oldWebClient" },
    };

    expect(errandOf(view).stage).toEqual({ kind: "oldWebClient" });
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

describe("el lote local: el resumen de cada elemento", () => {
  it("turns the local batch moment into a consent with the summary of each element", () => {
    const view: SiteErrandView = {
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

    expect(errandOf(view)).toEqual<Errand>({
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
});
