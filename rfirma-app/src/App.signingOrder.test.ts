import { describe, expect, it } from "vitest";
import { signingOrderFor } from "./App.signingOrder";
import { aCertificate } from "./App.testSupport";
import { base64Of } from "./signing/rubric";
import { rubric } from "./signing/SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./signing/visibleSignature";
import type { Placement } from "./viewer/signatureBox";

// Grada A: qué se firma (Testing Decisions, costura 1) — la orden lleva el
// modelo elegido y la rúbrica solo viaja cuando «Con rúbrica» está encendido.
describe("signingOrderFor · el modelo elegido", () => {
  const placement: Placement = { rect: { x0: 100, y0: 100, x1: 300, y1: 180 }, pages: "all" };
  const geometry = { page: 1, view: [0, 0, 595, 842] as const, rotate: 0 };

  it("carries the chosen model and the rubric in base64 when «Con rúbrica» is on", () => {
    const order = signingOrderFor({
      documentId: "doc-1",
      certificate: aCertificate,
      box: { placement, geometry },
      pageCount: 3,
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        content: { model: "rubricOnly" },
        withRubric: true,
      },
      rubric,
      signedAt: "hoy, 11:04",
      language: "es",
    });

    expect(order.content).toEqual({ model: "rubricOnly" });
    expect(order.withRubric).toBe(true);
    expect(order.rubric).toBe(base64Of(rubric));
  });

  it("does not carry the rubric with «Con rúbrica» off in the complete model", () => {
    const order = signingOrderFor({
      documentId: "doc-1",
      certificate: aCertificate,
      box: { placement, geometry },
      pageCount: 3,
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        content: { model: "complete" },
        withRubric: false,
      },
      rubric,
      signedAt: "hoy, 11:04",
      language: "es",
    });

    expect(order.content).toEqual({ model: "complete" });
    expect(order.withRubric).toBe(false);
    expect(order.rubric).toBeNull();
  });
});
