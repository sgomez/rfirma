import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { destinationOfferingSingleChoice } from "./App.testSupport";
import { useDestination } from "./App.usePreferencesState";
import type { SigningState } from "./signing/useSigning";

describe("useDestination", () => {
  /**
   * Regresión: antes de este arreglo, el asa de una sola firma no se olvidaba
   * al terminar de firmar, así que «Volver a firmar» del mismo documento
   * reutilizaba el destino de la firma anterior en vez del de la preferencia.
   */
  it("forgets the single-signature destination once the signature reaches signed", async () => {
    const destinations = destinationOfferingSingleChoice(
      { folder: "Documentos", name: "factura-firmado.pdf", writable: true },
      { id: "single-42", folder: "Escritorio", name: "factura-firmado-2.pdf", writable: true },
    );
    const { result, rerender } = renderHook<
      ReturnType<typeof useDestination>,
      { signingStateKind: SigningState["kind"] }
    >(
      ({ signingStateKind }) =>
        useDestination(destinations, "factura.pdf", "Documentos", signingStateKind),
      {
        initialProps: { signingStateKind: "idle" },
      },
    );

    await act(async () => {
      await result.current.chooseSingleDestination();
    });
    expect(result.current.singleDestinationId).toBe("single-42");
    await waitFor(() => expect(result.current.destination?.folder).toBe("Escritorio"));

    rerender({ signingStateKind: "signed" });

    expect(result.current.singleDestinationId).toBeNull();
    await waitFor(() => expect(result.current.destination?.folder).toBe("Documentos"));
  });
});
