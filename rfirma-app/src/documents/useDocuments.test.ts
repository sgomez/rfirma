import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { DocumentInHand } from "./document";
import { inMemoryDocumentPicker } from "./picker";
import { inMemoryRecents, type RecentDocument } from "./recents";
import { useDocuments } from "./useDocuments";

/** Una fila de la bandeja: lo que se guarda. */
function row(name: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    // El identificador lo acuña el backend y es opaco: aquí se finge con un
    // prefijo que ninguna ruta tendría, para que nada pueda leerlo como tal.
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    placement: null,
    ...overrides,
  };
}

/** Un documento en la mano: lo que se pinta y se firma (ID-287). */
function document(name: string, overrides: Partial<DocumentInHand> = {}): DocumentInHand {
  return {
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    placement: null,
    remembered: true,
    ...overrides,
  };
}

// Grada A: los dos puertos son dobles en memoria, así que no hay ni portal ni
// fichero de estado.
describe("useDocuments", () => {
  it("lists what the store remembers, without opening anything", async () => {
    const store = inMemoryRecents([row("a.pdf")]);

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
    // Y la fila que queda es la del mismo documento, con lo que solo la lista
    // sabe: cuándo se usó y que la ruta responde.
    expect(result.current.recents).toEqual([row("factura.pdf", { lastUsed: expect.any(Number) })]);
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

  /**
   * ID-306: los PDF que entran de más al soltar varios a la vez se anotan en
   * la bandeja, pero el documento activo no cambia — es lo que la persona
   * tiene delante y `enter` no lo toca.
   */
  it("notes a document in the tray without making it active", async () => {
    const store = inMemoryRecents();
    const factura = document("factura.pdf");
    const contrato = document("contrato.pdf");
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([factura])));
    await act(() => result.current.open());

    await act(() => result.current.enter(contrato));

    expect(result.current.active).toEqual(factura);
    expect(result.current.recents.map((row) => row.name)).toContain("contrato.pdf");
  });

  it("does not note anything when remembering is off", async () => {
    const store = inMemoryRecents();
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker(), false));

    await act(() => result.current.enter(document("contrato.pdf")));

    expect(result.current.recents).toEqual([]);
    expect(await store.list()).toEqual([]);
  });

  it("leaves everything as it was when the portal is cancelled", async () => {
    const store = inMemoryRecents([row("a.pdf")]);
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker()));
    await waitFor(() => expect(result.current.recents).toHaveLength(1));

    await act(() => result.current.open());

    expect(result.current.active).toBeNull();
    expect(result.current.recents).toHaveLength(1);
  });

  it("gives a document that was open before its page and its position back", async () => {
    // La fila la guarda el backend por su ruta canónica, así que reabrir el
    // mismo contrato —con otro identificador, ID-62— vuelve con su recuadro.
    const contrato = document("contrato.pdf");
    const box = { rect: { x0: 72, y0: 500, x1: 272, y1: 600 }, pages: { only: [3] } };
    const store = inMemoryRecents([row("contrato.pdf", { placement: box })]);
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([contrato])));

    await act(() => result.current.open());

    expect(result.current.active?.placement).toEqual(box);
  });

  it("does not let a brand new document inherit the position of another one", async () => {
    const box = { rect: { x0: 72, y0: 500, x1: 272, y1: 600 }, pages: { only: [3] } };
    const nomina = document("nomina.pdf");
    const store = inMemoryRecents([row("contrato.pdf", { placement: box })]);
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([nomina])));

    await act(() => result.current.open());

    expect(result.current.active?.placement).toBeNull();
  });

  it("writes where the box fell on the row of the document in front", async () => {
    const contrato = document("contrato.pdf");
    const store = inMemoryRecents();
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([contrato])));
    await act(() => result.current.open());

    const box = { rect: { x0: 10, y0: 20, x1: 210, y1: 120 }, pages: { only: [2] } };
    await act(() => result.current.place(box));

    expect(result.current.recents[0]?.placement).toEqual(box);
    await expect(store.list()).resolves.toEqual([
      row("contrato.pdf", { placement: box, lastUsed: expect.any(Number) }),
    ]);
  });

  it("has no row to write the box on when remembering is off", async () => {
    const contrato = document("contrato.pdf");
    const store = inMemoryRecents();
    const { result } = renderHook(() =>
      useDocuments(store, inMemoryDocumentPicker([contrato]), false),
    );
    await act(() => result.current.open());

    await act(() =>
      result.current.place({ rect: { x0: 10, y0: 20, x1: 210, y1: 120 }, pages: { only: [2] } }),
    );

    await expect(store.list()).resolves.toEqual([]);
  });

  /**
   * «Volver a firmar» relee el original del disco (ID-80): el documento activo
   * se repone, y quien lo mira —el efecto que abre el PDF— lo vuelve a leer
   * porque es otra referencia.
   */
  it("hands out a fresh active document when it is reopened", async () => {
    const contrato = document("contrato.pdf");
    const store = inMemoryRecents();
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([contrato])));
    await act(() => result.current.open());
    const before = result.current.active;

    act(() => result.current.reopen());

    expect(result.current.active).toEqual(before);
    expect(result.current.active).not.toBe(before);
  });

  it("reopens with the box where it was last dragged, not where it was opened", async () => {
    // El arrastre escribe en la fila y a propósito no toca el documento activo:
    // reponer la copia devolvería el recuadro a donde estaba al abrirlo.
    const contrato = document("contrato.pdf");
    const store = inMemoryRecents();
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([contrato])));
    await act(() => result.current.open());
    const box = { rect: { x0: 10, y0: 20, x1: 210, y1: 120 }, pages: { only: [2] } };
    await act(() => result.current.place(box));

    act(() => result.current.reopen());

    expect(result.current.active?.placement).toEqual(box);
  });

  /**
   * **TD-64**: un documento que no se recuerda se pone delante igual —se pinta
   * y se firma— pero no deja fila en la bandeja. Es el camino que necesita el
   * documento que manda una sede (ID-286).
   */
  it("puts a document that is not remembered in front without writing a row", async () => {
    const store = inMemoryRecents();
    const fromTheSede = document("de-la-sede.pdf", { remembered: false });
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([fromTheSede])));

    await act(() => result.current.open());

    expect(result.current.active).toEqual(fromTheSede);
    expect(result.current.recents).toEqual([]);
    await expect(store.list()).resolves.toEqual([]);
  });

  it("has no row to write the box on for a document that is not remembered", async () => {
    const store = inMemoryRecents();
    const fromTheSede = document("de-la-sede.pdf", { remembered: false });
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([fromTheSede])));
    await act(() => result.current.open());

    await act(() =>
      result.current.place({ rect: { x0: 10, y0: 20, x1: 210, y1: 120 }, pages: { only: [2] } }),
    );

    expect(result.current.recents).toEqual([]);
    await expect(store.list()).resolves.toEqual([]);
  });

  it("keeps a document that is not remembered in front when it is reopened", async () => {
    // No tiene fila de donde reponerse, así que se repone de sí mismo: lo que
    // «Volver a firmar» necesita es otra referencia, no otra fuente.
    const store = inMemoryRecents();
    const fromTheSede = document("de-la-sede.pdf", { remembered: false });
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker([fromTheSede])));
    await act(() => result.current.open());
    const before = result.current.active;

    act(() => result.current.reopen());

    expect(result.current.active).toEqual(fromTheSede);
    expect(result.current.active).not.toBe(before);
  });

  it("takes the row chosen in the tray in hand, and it is remembered", async () => {
    const store = inMemoryRecents([row("a.pdf")]);
    const { result } = renderHook(() => useDocuments(store, inMemoryDocumentPicker()));
    await waitFor(() => expect(result.current.recents).toHaveLength(1));

    act(() => {
      const chosen = result.current.recents[0];
      if (chosen) result.current.select(chosen);
    });

    expect(result.current.active).toEqual(document("a.pdf"));
  });

  it("has nothing to reopen without a document in front", () => {
    const { result } = renderHook(() => useDocuments(inMemoryRecents(), inMemoryDocumentPicker()));

    act(() => result.current.reopen());

    expect(result.current.active).toBeNull();
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
