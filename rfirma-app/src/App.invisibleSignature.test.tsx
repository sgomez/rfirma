import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";
import type { SigningBackend, SigningOrder } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";

function recordingSigner(presigned: SigningOrder[]): SigningBackend {
  return {
    presign: vi.fn(async (order: SigningOrder) => {
      presigned.push(order);
      return { ok: true as const, value: { kind: "typedOnScreen" as const } };
    }),
    sign: async () => ({ ok: true, value: undefined }),
    postsign: async () => ({
      ok: true,
      value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
    }),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    unregisteredSignatures: async () => false,
    previousSignatures: async () => ({
      signatures: [],
      warningCount: 0,
      tone: "information",
      changedAfterLastSignature: false,
    }),
    discard: async () => {},
  };
}

async function signingPanel(presigned: SigningOrder[]) {
  const user = userEvent.setup();
  renderApp(
    inMemoryRecents(),
    [document("factura.pdf")],
    pdfsOf({ "factura.pdf": 3 }),
    {},
    { list: async () => [{ ...aCertificate, remembered: true }] },
    emptyRubricPicker(),
    recordingSigner(presigned),
  );
  await openPdf(user);
  const panel = await screen.findByRole("region", { name: "Panel de firma" });
  await within(panel).findByRole("button", { name: "Firmar como Ada Lovelace" });
  return { user, panel };
}

describe("App, firmando con la firma visible apagada", () => {
  it("sends an order without placement when the switch was never turned on", async () => {
    const presigned: SigningOrder[] = [];
    const { user, panel } = await signingPanel(presigned);

    await user.click(within(panel).getByRole("button", { name: "Firmar como Ada Lovelace" }));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.placement).toBeNull();
  });

  it("sends an order without placement when the switch was turned on and off again", async () => {
    const presigned: SigningOrder[] = [];
    const { user, panel } = await signingPanel(presigned);
    const toggle = within(panel).getByRole("switch", { name: "Firma visible" });
    await user.click(toggle);
    await within(panel).findByText("En la página 1");
    await user.click(toggle);

    await user.click(within(panel).getByRole("button", { name: "Firmar como Ada Lovelace" }));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.placement).toBeNull();
  });
});
