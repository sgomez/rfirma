import { describe, expect, it } from "vitest";
import type { RecentDocument } from "./recents";
import { CAPACITY, forget, inMemoryRecents, record, shownBadge } from "./recents";

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
    placement: null,
    ...overrides,
  };
}

// Grada A: reglas de la lista, sin disco ni backend.
describe("record", () => {
  it("puts the newest document first", () => {
    const recents = record([document("a.pdf")], document("b.pdf"));

    expect(recents.map((entry) => entry.name)).toEqual(["b.pdf", "a.pdf"]);
  });

  // Reelegir la misma fila la rescata al frente. Reabrir el mismo fichero por
  // el diálogo no pasa por aquí con el mismo id: el backend acuña uno nuevo
  // por concesión (ID-62), y eso son dos filas.
  it("identifies a document by its opaque id, so re-selecting a row rescues it", () => {
    const recents = record([document("a.pdf"), document("b.pdf")], document("b.pdf"));

    expect(recents.map((entry) => entry.name)).toEqual(["b.pdf", "a.pdf"]);
  });

  it("keeps at most ten documents, evicting the least recently used", () => {
    const eleven = Array.from({ length: 11 }, (_, index) => document(`${index}.pdf`));

    const recents = eleven.reduce((list, entry) => record(list, entry), [] as RecentDocument[]);

    expect(recents).toHaveLength(CAPACITY);
    expect(recents.at(0)?.name).toBe("10.pdf");
    expect(recents.map((entry) => entry.id)).not.toContain("id-0.pdf");
  });
});

describe("shownBadge", () => {
  it("paints the cached badge while the document answers", () => {
    expect(shownBadge(document("a.pdf", { badge: "Signed" }))).toBe("Signed");
  });

  it("paints Unavailable when the document no longer answers", () => {
    expect(shownBadge(document("a.pdf", { badge: "Signed", available: false }))).toBe(
      "Unavailable",
    );
  });
});

describe("forget", () => {
  it("takes one row out of the list", () => {
    const recents = forget([document("a.pdf"), document("b.pdf")], "id-a.pdf");

    expect(recents.map((entry) => entry.name)).toEqual(["b.pdf"]);
  });
});

describe("inMemoryRecents", () => {
  it("never hands out more than ten entries, not even the ones it was built with", async () => {
    const store = inMemoryRecents(
      Array.from({ length: 15 }, (_, index) => document(`${index}.pdf`)),
    );

    expect(await store.list()).toHaveLength(CAPACITY);
  });

  it("empties the list", async () => {
    const store = inMemoryRecents([document("a.pdf")]);

    await store.clear();

    expect(await store.list()).toEqual([]);
  });
});
