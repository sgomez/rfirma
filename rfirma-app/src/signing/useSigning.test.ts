import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Certificate } from "./certificate";
import type { SignedDocument, SigningBackend, SigningOrder, StageResult } from "./flow";
import type { TokenFailure } from "./token";
import { acknowledgementFor, useSigning } from "./useSigning";

const signed: SignedDocument = {
  name: "contrato_firmado.pdf",
  folder: "Documentos",
  sizeBytes: 2_400_000,
};

const certificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid" },
  remembered: false,
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
    pages: { only: [3] },
    pageCount: 10,
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
    discard: async () => {},
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
    // El asa del documento de partida viaja con el estado: es lo que ata el
    // acuse de recibo a su documento (`acknowledgementFor`).
    expect(result.current.state).toEqual({
      kind: "signed",
      document: signed,
      origin: anOrder().document,
    });
  });

  /**
   * Sin esto el acuse de recibo no tendría salida: el estado «Firmado» se queda
   * montado y no hay forma de volver al panel para firmar otro documento.
   */
  it("goes back to the panel from the signed state without telling the backend", async () => {
    const discard = vi.fn(async () => {});
    const { result } = renderHook(() => useSigning(backendOf({ discard })));

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("1234"));
    act(() => result.current.signAnother());

    expect(result.current.state).toEqual({ kind: "idle" });
    // El ciclo terminó por su propio pie en la postfirma: no hay nada a medias
    // que el backend tenga que olvidar.
    expect(discard).not.toHaveBeenCalled();
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
    expect(result.current.state).toEqual({
      kind: "signed",
      document: signed,
      origin: anOrder().document,
    });
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

  /**
   * Volver al panel no basta: el ciclo a medias lo guarda el backend, así que
   * cancelar tiene que decírselo o el PDF, los atributos a firmar y el sello se
   * quedan vivos hasta que se cierre la ventana.
   */
  it("tells the backend to forget the half-open cycle when the PIN dialog is cancelled", async () => {
    const discard = vi.fn(async () => {});
    const { result } = renderHook(() => useSigning(backendOf({ discard })));

    await act(() => result.current.start(certificate, anOrder()));
    act(() => result.current.cancel());

    expect(discard).toHaveBeenCalledTimes(1);
  });

  /** Cerrar el aviso de un fallo va por el mismo `cancel`, y también limpia. */
  it("forgets the cycle when a failure notice is dismissed as well", async () => {
    const discard = vi.fn(async () => {});
    const { result } = renderHook(() =>
      useSigning(backendOf({ discard, sign: async () => failed(cardGone) })),
    );

    await act(() => result.current.start(certificate, anOrder()));
    await act(() => result.current.submitPin("1234"));
    expect(result.current.state).toEqual({ kind: "failed", failure: cardGone });

    act(() => result.current.cancel());

    expect(discard).toHaveBeenCalledTimes(1);
  });

  /**
   * Si el backend no puede olvidar, la ventana vuelve al panel igual: no hay
   * nada que contarle a nadie, y una promesa rechazada sin dueño tumbaría el
   * proceso.
   */
  it("goes back to the panel even if the backend cannot forget the cycle", async () => {
    const discard = vi.fn(() => Promise.reject(new Error("no hay isolate")));
    const { result } = renderHook(() => useSigning(backendOf({ discard })));

    await act(() => result.current.start(certificate, anOrder()));
    act(() => result.current.cancel());

    expect(result.current.state).toEqual({ kind: "idle" });
    expect(discard).toHaveBeenCalled();
  });
});

/**
 * La regla que ata el acuse de recibo a su documento. Vive fuera del gancho
 * porque la ventana la necesita **en la pintada**, antes de que ningún efecto
 * haya corrido: un solo fotograma con el nombre de un fichero y las páginas de
 * otro ya es el dato inventado que el ID-44 prohíbe.
 */
describe("acknowledgementFor", () => {
  const signedState = {
    kind: "signed",
    document: signed,
    origin: "/run/user/1000/doc/1e8b83b9/contrato.pdf",
  } as const;

  it("shows the acknowledgement while its own document is the active one", () => {
    expect(acknowledgementFor(signedState, signedState.origin)).toBe(signedState);
  });

  it("hides it when another document has been opened", () => {
    // Con otro delante el recuento de páginas sería el del otro documento.
    expect(acknowledgementFor(signedState, "/run/user/1000/doc/aa11bb22/otro.pdf")).toBeNull();
  });

  it("hides it when there is no active document left", () => {
    // Olvidar el activo o vaciar la lista: sin esto quedaba una tercera columna
    // al lado del visor en su estado vacio (ID-51).
    expect(acknowledgementFor(signedState, null)).toBeNull();
  });

  it("has nothing to show in the states that are not the signed one", () => {
    expect(acknowledgementFor({ kind: "idle" }, "/a.pdf")).toBeNull();
    expect(acknowledgementFor({ kind: "running", stage: "sign" }, "/a.pdf")).toBeNull();
    expect(acknowledgementFor({ kind: "pin", failure: null }, "/a.pdf")).toBeNull();
  });
});
