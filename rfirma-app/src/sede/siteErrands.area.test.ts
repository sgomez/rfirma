import { describe, expect, it, vi } from "vitest";
import type { MarkedArea } from "./errand";
import { MARKING_THE_AREA, opened, watched } from "./siteErrandsFixtures";

/** Grada A: el área de la firma visible que pide `visibleSignature`, contra las órdenes dobladas. */

const AREA: MarkedArea = {
  page: 1,
  pages: { only: [1] },
  pageCount: 3,
  mediaBox: [0, 0, 595, 842],
  rotation: 0,
  rect: [50, 60, 250, 140],
};

describe("el área de la firma visible", () => {
  it("opens the document the backend names and shows it to mark the area", async () => {
    const { push, calls, last } = watched();
    push(MARKING_THE_AREA);

    await vi.waitFor(() => expect(last()?.stage.kind).toBe("marking"));
    expect(calls.openDocument).toHaveBeenCalledWith("asa-opaca-1");
    expect(last()?.stage).toEqual({ kind: "marking", pdf: opened });
  });

  it("hands the marked area on and waits for the backend to publish the consent", async () => {
    const { push, port, calls, last } = watched();
    push(MARKING_THE_AREA);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("marking"));

    await port.markArea(AREA);

    expect(calls.markArea).toHaveBeenCalledWith(AREA);
    expect(last()?.stage.kind).toBe("marking");
  });

  it("shows the cancelled outcome when cancelling ended the errand", async () => {
    const { push, port, calls, last } = watched({
      markArea: async () => ({ ok: true, value: false }),
    });
    push(MARKING_THE_AREA);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("marking"));

    await port.markArea(null);

    expect(calls.decline).not.toHaveBeenCalled();
    expect(last()?.stage).toEqual({
      kind: "outcome",
      outcome: { kind: "cancelled", document: null },
    });
  });

  it("ends the errand when the area cannot be handed on", async () => {
    const { push, port, last } = watched({
      markArea: async () => ({
        ok: false,
        failure: {
          situation: "unknown",
          detail: "el recuadro se sale de la pagina",
          attemptsLeft: null,
        },
      }),
    });
    push(MARKING_THE_AREA);
    await vi.waitFor(() => expect(last()?.stage.kind).toBe("marking"));

    await port.markArea(AREA);

    expect(last()?.stage.kind).toBe("outcome");
  });

  it("marks nothing outside the area moment", async () => {
    const { port, calls } = watched();

    await port.markArea(AREA);

    expect(calls.markArea).not.toHaveBeenCalled();
  });
});
