import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { inMemoryDocumentPicker } from "./picker";
import { inMemoryRecents, type RecentDocument } from "./recents";
import { useDocuments } from "./useDocuments";

function document(name: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    // El identificador lo acuña el backend y es opaco: aquí se finge con un
    // prefijo que ninguna ruta tendría, para que nada pueda leerlo como tal.
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    ...overrides,
  };
}

// Grada A: los dos puertos son dobles en memoria, así que no hay ni portal ni
// fichero de estado.
describe("useDocuments", () => {
  it("lists what the store remembers, without opening anything", async () => {
    const store = inMemoryRecents([document("a.pdf")]);

    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker()));

    await waitFor(() => expect(result.current.recents).toHaveLength(1));
    expect(result.current.active).toBeNull();
  });

  it("makes the document opened through the portal the active one and remembers it", async () => {
    const store = inMemoryRecents();
    const factura = document("factura.pdf");
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([factura])));

    await act(() => result.current.open());

    expect(result.current.active).toEqual(factura);
    expect(result.current.recents).toEqual([factura]);
  });

  it("does not remember the document opened through the portal when remembering is off", async () => {
    const store = inMemoryRecents();
    const factura = document("factura.pdf");
    const { result } = renderHook(() =>
      useDocuments(store, inMemoryDocumentPicker([factura]), false),
    );

    await act(() => result.current.open());

    expect(result.current.active).toEqual(factura);
    expect(result.current.recents).toEqual([]);
    expect(await store.list()).toEqual([]);
  });

  it("leaves everything as it was when the portal is cancelled", async () => {
    const store = inMemoryRecents([document("a.pdf")]);
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker()));
    await waitFor(() => expect(result.current.recents).toHaveLength(1));

    await act(() => result.current.open());

    expect(result.current.active).toBeNull();
    expect(result.current.recents).toHaveLength(1);
  });

  it("drops the active document when its row is removed from the list", async () => {
    const store = inMemoryRecents();
    const informe = document("informe.pdf");
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([informe])));
    await act(() => result.current.open());

    await act(() => result.current.forget("id-informe.pdf"));

    expect(result.current.recents).toEqual([]);
    expect(result.current.active).toBeNull();
  });
});
