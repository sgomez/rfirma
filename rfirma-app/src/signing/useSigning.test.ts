import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Certificate } from "./certificate";
import type { SignedDocument, SigningBackend, SigningOrder, StageResult } from "./flow";
import type { TokenFailure } from "./token";
import { useSigning } from "./useSigning";

const signed: SignedDocument = { name: "contrato_firmado.pdf", folder: "Documentos" };

const certificate: Certificate = {
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  status: { kind: "valid" },
};

const wrongPin: TokenFailure = {
  situation: "incorrectPin",
  detail: "CKR_PIN_INCORRECT (C_Login)",
  attemptsLeft: 2,
};

const cardGone: TokenFailure = {
  situation: "tokenAbsent",
  detail: "CKR_DEVICE_REMOVED (C_Sign)",
  attemptsLeft: null,
};

/**
 * Una orden cualquiera. Lo que se prueba aquí es **el orden de las etapas**, no
 * su contenido: la orden se pasa entera y este bucle no la mira.
 */
const anOrder = (): SigningOrder => ({
  document: "/run/user/1000/doc/1e8b83b9/contrato.pdf",
  certificate: "Firma",
  placement: {
    page: 3,
    mediaBox: [0, 0, 595, 842],
    rotation: 0,
    rect: [72, 500, 272, 600],
  },
  fields: { signerName: true, idNumber: true, signedAt: true, reason: false },
  reason: "",
  signedAt: "31/08/26, 12:00:00",
  rubric: null,
  language: "es",
});

const ok = <T>(value: T): StageResult<T> => ({ ok: true, value });
const failed = (failure: TokenFailure): StageResult<never> => ({ ok: false, failure });

/** Un backend de mentira: cada etapa devuelve lo que se le diga, en orden. */
function backendOf(overrides: Partial<SigningBackend> = {}): SigningBackend {
  return {
    presign: async () => ok(undefined),
    sign: async () => ok(undefined),
    postsign: async () => ok(signed),
    ...overrides,
  };
}

// Grada A: las tres etapas son un puerto, y aquí se conducen con un doble.
describe("useSigning", () => {
  it("asks for the PIN after the presignature, never before", async () => {
    const presign = vi.fn(async () => ok(undefined));
    const { result } = renderHook(() => useSigning(backendOf({ presign })));

    const started = act(() => result.current.start(certificate, anOrder()));
    expect(presign).toHaveBeenCalled();
    await started;

    expect(result.current.state).toEqual({ kind: "pin", failure: null });
  });

  it("runs the three stages in order and ends with the signed document", async () => {
    const calls: string[] = [];
    const { result } = renderHook(() =>
      useSigning(
        backendOf({
          presign: async () => {
            calls.push("presign");
            return ok(undefined);
          },
          sign: async () => {
            calls.push("sign");
            return ok(undefined);
          },
          postsign: async () => {
            calls.push("postsign");
            return ok(signed);
          },
        }),
      ),
    );

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("1234"));

    expect(calls).toEqual(["presign", "sign", "postsign"]);
    expect(result.current.state).toEqual({ kind: "signed", document: signed });
  });

  it("retries a wrong PIN without repeating the presignature", async () => {
    const presign = vi.fn(async () => ok(undefined));
    const sign = vi
      .fn<SigningBackend["sign"]>()
      .mockResolvedValueOnce(failed(wrongPin))
      .mockResolvedValueOnce(ok(undefined));
    const { result } = renderHook(() => useSigning(backendOf({ presign, sign })));

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("0000"));

    expect(result.current.state).toEqual({ kind: "pin", failure: wrongPin });

    await act(() => result.current.submitPin("1234"));

    expect(presign).toHaveBeenCalledTimes(1);
    expect(result.current.state).toEqual({ kind: "signed", document: signed });
  });

  it("takes a token failure that is not about the PIN out of the dialog", async () => {
    const { result } = renderHook(() =>
      useSigning(backendOf({ sign: async () => failed(cardGone) })),
    );

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("1234"));

    expect(result.current.state).toEqual({ kind: "failed", failure: cardGone });
  });

  it("keeps the postsignature failure with its own stage in the raw detail", async () => {
    const assembling: TokenFailure = {
      situation: "unknown",
      detail: "postfirma: el PDF no se ha podido ensamblar",
      attemptsLeft: null,
    };
    const { result } = renderHook(() =>
      useSigning(backendOf({ postsign: async () => failed(assembling) })),
    );

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("1234"));

    expect(result.current.state).toEqual({ kind: "failed", failure: assembling });
  });

  it("warns about an expired certificate before ever asking for the PIN", async () => {
    const presign = vi.fn(async () => ok(undefined));
    const { result } = renderHook(() => useSigning(backendOf({ presign })));

    await act(() =>
      result.current.start(
        {
          ...certificate,
          status: { kind: "expired", notAfter: 1_767_225_600 },
        },
        anOrder(),
      ),
    );

    expect(presign).not.toHaveBeenCalled();
    expect(result.current.state).toEqual({
      kind: "failed",
      failure: { situation: "certificateExpired", detail: "notAfter=1767225600" },
    });
  });

  it("warns about a revoked certificate before ever asking for the PIN", async () => {
    const presign = vi.fn(async () => ok(undefined));
    const { result } = renderHook(() => useSigning(backendOf({ presign })));

    await act(() =>
      result.current.start(
        {
          ...certificate,
          status: { kind: "revoked", reason: "keyCompromise" },
        },
        anOrder(),
      ),
    );

    expect(presign).not.toHaveBeenCalled();
    expect(result.current.state).toEqual({
      kind: "failed",
      failure: { situation: "certificateRevoked", detail: "revocado: keyCompromise" },
    });
  });

  it("goes back to the panel when the PIN dialog is cancelled", async () => {
    const { result } = renderHook(() => useSigning(backendOf()));

    await act(() => result.current.start(certificate, anOrder()));
    act(() => result.current.cancel());

    expect(result.current.state).toEqual({ kind: "idle" });
  });
});
