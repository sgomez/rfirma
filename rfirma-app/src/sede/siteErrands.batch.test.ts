import { describe, expect, it, vi } from "vitest";
import type { SiteErrandView } from "./siteErrands";
import { certificate, watched } from "./siteErrandsFixtures";

/** Grada A: el lote, remoto y local, contra las órdenes dobladas (TD-78). */

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
