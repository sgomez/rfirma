import { describe, expect, it } from "vitest";
import type { RecentDocument } from "./recents";
import { CAPACITY, forget, inMemoryRecents, record, shownBadge } from "./recents";

function document(path: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    path,
    name: path.slice(path.lastIndexOf("/") + 1),
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    ...overrides,
  };
}

// Grada A: reglas de la lista, sin disco ni backend.
describe("record", () => {
  it("puts the newest document first", () => {
    const recents = record([document("/a.pdf")], document("/b.pdf"));

    expect(recents.map((entry) => entry.path)).toEqual(["/b.pdf", "/a.pdf"]);
  });

  it("identifies a document by its canonical path, so reopening an old one rescues it", () => {
    const recents = record([document("/a.pdf"), document("/b.pdf")], document("/b.pdf"));

    expect(recents.map((entry) => entry.path)).toEqual(["/b.pdf", "/a.pdf"]);
  });

  it("keeps at most ten documents, evicting the least recently used", () => {
    const eleven = Array.from({ length: 11 }, (_, index) => document(`/${index}.pdf`));

    const recents = eleven.reduce((list, entry) => record(list, entry), [] as RecentDocument[]);

    expect(recents).toHaveLength(CAPACITY);
    expect(recents.at(0)?.path).toBe("/10.pdf");
    expect(recents.map((entry) => entry.path)).not.toContain("/0.pdf");
  });
});

describe("shownBadge", () => {
  it("paints the cached badge while the path answers", () => {
    expect(shownBadge(document("/a.pdf", { badge: "Signed" }))).toBe("Signed");
  });

  it("paints Unavailable when the path no longer answers", () => {
    expect(shownBadge(document("/a.pdf", { badge: "Signed", available: false }))).toBe(
      "Unavailable",
    );
  });
});

describe("forget", () => {
  it("takes one row out of the list", () => {
    const recents = forget([document("/a.pdf"), document("/b.pdf")], "/a.pdf");

    expect(recents.map((entry) => entry.path)).toEqual(["/b.pdf"]);
  });
});

describe("inMemoryRecents", () => {
  it("never hands out more than ten entries, not even the ones it was built with", async () => {
    const store = inMemoryRecents(
      Array.from({ length: 15 }, (_, index) => document(`/${index}.pdf`)),
    );

    expect(await store.list()).toHaveLength(CAPACITY);
  });

  it("empties the list", async () => {
    const store = inMemoryRecents([document("/a.pdf")]);

    await store.clear();

    expect(await store.list()).toEqual([]);
  });
});
