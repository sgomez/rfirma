import { waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { inMemoryRecents } from "./documents/recents";
import { document, openPdf, pdfsOf, renderApp } from "./testing/harness";

describe("App, con un recuadro recordado que no cabe en su única página", () => {
  it("puts it inside the page, keeping its size, and remembers it there", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    const outside = { rect: { x0: 600, y0: 40, x1: 800, y1: 120 }, pages: { only: [1] } };
    renderApp({
      recents,
      documents: [document("recordado.pdf", { placement: outside })],
      pdfs: pdfsOf({ "recordado.pdf": 2 }),
    });

    await openPdf(user);

    await waitFor(async () =>
      expect((await recents.list())[0]?.placement).toEqual({
        rect: { x0: 395, y0: 40, x1: 595, y1: 120 },
        pages: { only: [1] },
      }),
    );
  });
});
