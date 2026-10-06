import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  aCertificateSection,
  aRubricSection,
  aVisibleSignatureSection,
  certificate,
  renderPanel,
  rubric,
} from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

// Grada A: el modelo y la rúbrica (docs/design/panel-de-firma.md § El modelo, § La rúbrica).
describe("SigningPanel · Modelo y rúbrica", () => {
  it("chooses the complete model", async () => {
    const user = userEvent.setup();
    const change = vi.fn();
    renderPanel({
      signature: aVisibleSignatureSection(
        {
          ...DEFAULT_VISIBLE_SIGNATURE,
          enabled: true,
          withRubric: true,
          content: { model: "rubricOnly" },
        },
        { change },
      ),
      rubric: aRubricSection({ value: rubric }),
    });

    await user.click(screen.getByRole("radio", { name: "Completa" }));

    expect(change).toHaveBeenCalledWith(
      expect.objectContaining({ content: { model: "complete" } }),
    );
  });

  it("chooses the rubric-only model and turns «Con rúbrica» on with it", async () => {
    const user = userEvent.setup();
    const change = vi.fn();
    renderPanel({
      signature: aVisibleSignatureSection(
        { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true, withRubric: true },
        { change },
      ),
      rubric: aRubricSection({ value: rubric }),
    });

    await user.click(screen.getByRole("radio", { name: "Solo rúbrica" }));

    expect(change).toHaveBeenCalledWith(
      expect.objectContaining({ content: { model: "rubricOnly" }, withRubric: true }),
    );
  });

  it("chooses the custom model with a starting phrase of signer and date", async () => {
    const user = userEvent.setup();
    const change = vi.fn();
    renderPanel({
      signature: aVisibleSignatureSection(
        { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true },
        { change },
      ),
    });

    await user.click(screen.getByRole("radio", { name: "Personalizada" }));

    expect(change).toHaveBeenCalledWith(
      expect.objectContaining({
        content: {
          model: "custom",
          phrase: [
            { text: "Visto bueno de " },
            { datum: "signer" },
            { text: ", " },
            { datum: "signedAt" },
          ],
        },
      }),
    );
  });

  it("shows the signer with its identifier masked, exactly as it will be stamped", async () => {
    const user = userEvent.setup();
    const fnmtTest = {
      ...certificate,
      holderName: "EIDAS CERTIFICADO PRUEBAS - 99999999R",
      stampedSigner: "EIDAS CERTIFICADO PRUEBAS - ***9999**",
    };
    const { container } = renderPanel({
      certificate: aCertificateSection({
        kind: "chosen",
        certificate: fnmtTest,
        certificates: [fnmtTest],
      }),
      signature: aVisibleSignatureSection({
        ...DEFAULT_VISIBLE_SIGNATURE,
        enabled: true,
        content: { model: "custom", phrase: [{ datum: "signer" }] },
      }),
    });

    await user.click(screen.getByRole("button", { name: "Dato" }));

    const masked = "EIDAS CERTIFICADO PRUEBAS - ***9999**";
    const menu = screen.getByRole("menu", { name: "Dato" });
    expect(screen.getByRole("textbox", { name: /frase/i })).toHaveTextContent(masked);
    expect(within(menu).getAllByRole("menuitem")[0]).toHaveTextContent(`Firmante${masked}`);
    const thumbnails = Array.from(container.querySelectorAll(".panel__model-lines"));
    expect(thumbnails.map((lines) => lines.textContent).join(" ")).toContain(masked);
    expect(thumbnails.map((lines) => lines.textContent).join(" ")).not.toContain("99999999R");
  });

  it("turns «Con rúbrica» off with a click, when nothing locks it", async () => {
    const user = userEvent.setup();
    const change = vi.fn();
    renderPanel({
      signature: aVisibleSignatureSection(
        { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true, withRubric: true },
        { change },
      ),
      rubric: aRubricSection({ value: rubric }),
    });

    await user.click(screen.getByRole("switch", { name: "Con rúbrica" }));

    expect(change).toHaveBeenCalledWith(expect.objectContaining({ withRubric: false }));
  });
});
