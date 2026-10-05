import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";
import type { SigningBackend, SigningOrder } from "./signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";

const TODAY = new Intl.DateTimeFormat("es", { dateStyle: "short" }).format(new Date());

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
    previousSignatures: async () => ({
      signatures: [],
      warningCount: 0,
      tone: "information",
      changedAfterLastSignature: false,
      findings: [],
    }),
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
  };
}

/** El panel con la firma visible encendida y *Personalizada* elegida. */
async function withCustomModel(presigned: SigningOrder[] = []) {
  const user = userEvent.setup();
  renderApp({
    recents: inMemoryRecents(),
    documents: [document("factura.pdf")],
    pdfs: pdfsOf({ "factura.pdf": 3 }),
    settings: {},
    certificates: { list: async () => [{ ...aCertificate, remembered: true }] },
    rubrics: emptyRubricPicker(),
    signer: recordingSigner(presigned),
  });
  await openPdf(user);
  const panel = await screen.findByRole("region", { name: "Panel de firma" });
  await within(panel).findByRole("button", { name: "Firmar" });
  await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
  await user.click(await within(panel).findByRole("radio", { name: "Personalizada" }));
  const phrase = within(panel).getByRole("textbox", { name: "Frase de la firma" });
  return { user, panel, phrase };
}

/** Deja el cursor al final de la frase, como un clic tras su último carácter. */
function caretAtEnd(phrase: HTMLElement) {
  phrase.focus();
  const range = window.document.createRange();
  range.selectNodeContents(phrase);
  range.collapse(false);
  window.getSelection()?.removeAllRanges();
  window.getSelection()?.addRange(range);
}

/** Deja el cursor al principio de la frase, antes de su primer carácter. */
function caretAtStart(phrase: HTMLElement) {
  phrase.focus();
  const range = window.document.createRange();
  range.setStart(phrase, 0);
  range.collapse(true);
  window.getSelection()?.removeAllRanges();
  window.getSelection()?.addRange(range);
}

function pillsIn(phrase: HTMLElement) {
  return Array.from(phrase.querySelectorAll<HTMLElement>("[data-datum]")).map(
    (pill) => pill.dataset.datum,
  );
}

// Grada A: la frase de *Personalizada* (docs/design/panel-de-firma.md § El modelo).
describe("App, con el modelo Personalizada", () => {
  it("starts from a phrase that shows the signer and the date already resolved, as pills", async () => {
    const { phrase } = await withCustomModel();

    expect(phrase).toHaveTextContent(`Visto bueno de Ada Lovelace Byron, ${TODAY}`);
    expect(pillsIn(phrase)).toEqual(["signer", "signedAt"]);
  });

  it("edits the phrase as text around the pills", async () => {
    const { user, phrase } = await withCustomModel();

    caretAtEnd(phrase);
    await user.keyboard(" en Sevilla");

    expect(phrase).toHaveTextContent(`Visto bueno de Ada Lovelace Byron, ${TODAY} en Sevilla`);
    expect(pillsIn(phrase)).toEqual(["signer", "signedAt"]);
  });

  it("offers the signer, the issuer and the date in «+ Dato», each with its sample value", async () => {
    const { user, panel } = await withCustomModel();

    await user.click(within(panel).getByRole("button", { name: "Dato" }));

    const menu = within(panel).getByRole("menu", { name: "Dato" });
    const items = within(menu).getAllByRole("menuitem");
    expect(items.map((item) => item.textContent)).toEqual([
      "FirmanteAda Lovelace Byron",
      "EmisorAC FNMT Usuarios",
      `Fecha${TODAY}`,
    ]);
  });

  it("inserts the datum where the caret was left", async () => {
    const { user, panel, phrase } = await withCustomModel();
    caretAtEnd(phrase);
    await user.keyboard(" con ");

    await user.click(within(panel).getByRole("button", { name: "Dato" }));
    await user.click(within(panel).getByRole("menuitem", { name: /^Emisor/ }));

    expect(phrase).toHaveTextContent(
      `Visto bueno de Ada Lovelace Byron, ${TODAY} con AC FNMT Usuarios`,
    );
    expect(pillsIn(phrase)).toEqual(["signer", "signedAt", "issuer"]);
    expect(within(panel).queryByRole("menu")).not.toBeInTheDocument();
  });

  it("keeps the caret where it was when «+ Dato» closes without a choice", async () => {
    const { user, panel, phrase } = await withCustomModel();
    caretAtStart(phrase);
    const add = within(panel).getByRole("button", { name: "Dato" });

    await user.click(add);
    await user.keyboard("{Escape}");
    await user.click(add);
    await user.click(within(panel).getByRole("menuitem", { name: /^Emisor/ }));

    expect(pillsIn(phrase)).toEqual(["issuer", "signer", "signedAt"]);
  });

  it("deletes a pill whole, like a word, with one Backspace", async () => {
    const { user, panel, phrase } = await withCustomModel();
    await user.click(within(panel).getByRole("button", { name: "Dato" }));
    await user.click(within(panel).getByRole("menuitem", { name: /^Emisor/ }));

    await user.keyboard("{Backspace}");

    expect(pillsIn(phrase)).toEqual(["signer", "signedAt"]);
    expect(phrase).toHaveTextContent(`Visto bueno de Ada Lovelace Byron, ${TODAY}`);
  });

  it("works «+ Dato» from the keyboard, and Escape gives the focus back to the button", async () => {
    const { user, panel, phrase } = await withCustomModel();
    const add = within(panel).getByRole("button", { name: "Dato" });

    add.focus();
    await user.keyboard("{Enter}");
    expect(within(panel).getByRole("menuitem", { name: /^Firmante/ })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(within(panel).queryByRole("menu")).not.toBeInTheDocument();
    expect(add).toHaveFocus();

    await user.keyboard("{Enter}");
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");

    expect(pillsIn(phrase)).toEqual(["signer", "signedAt", "signedAt"]);
    expect(phrase).toHaveFocus();
  });

  it("keeps a single line: Enter does not break the phrase", async () => {
    const { user, phrase } = await withCustomModel();

    caretAtEnd(phrase);
    await user.keyboard("{Enter}");

    expect(phrase.querySelector("br, div")).toBeNull();
  });

  it("brings the phrase back after choosing another model and returning", async () => {
    const { user, panel, phrase } = await withCustomModel();
    caretAtEnd(phrase);
    await user.keyboard(" en Sevilla");

    await user.click(within(panel).getByRole("radio", { name: "Completa" }));
    await user.click(within(panel).getByRole("radio", { name: "Personalizada" }));

    expect(within(panel).getByRole("textbox", { name: "Frase de la firma" })).toHaveTextContent(
      `Visto bueno de Ada Lovelace Byron, ${TODAY} en Sevilla`,
    );
  });

  it("shows no reason, neither in the panel nor in any section of Preferences", async () => {
    const { user, panel } = await withCustomModel();
    expect(within(panel).queryByText(/motivo/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    const preferences = await screen.findByRole("region", { name: "Preferencias" });
    const index = within(preferences).getByRole("navigation");
    for (const section of within(index).getAllByRole("tab")) {
      await user.click(section);
      expect(within(preferences).queryByText(/motivo/i)).not.toBeInTheDocument();
    }
  });
});
