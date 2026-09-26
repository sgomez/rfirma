import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderPanel, rubric } from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

// Grada A: el modelo y la rúbrica (docs/design/panel-de-firma.md § El modelo, § La rúbrica).
describe("SigningPanel · Modelo y rúbrica", () => {
  it("chooses the complete model", async () => {
    const user = userEvent.setup();
    const onChangeSignature = vi.fn();
    renderPanel({
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        enabled: true,
        withRubric: true,
        content: { model: "rubricOnly" },
      },
      rubric,
      onChangeSignature,
    });

    await user.click(screen.getByRole("radio", { name: "Completa" }));

    expect(onChangeSignature).toHaveBeenCalledWith(
      expect.objectContaining({ content: { model: "complete" } }),
    );
  });

  it("chooses the rubric-only model and turns «Con rúbrica» on with it", async () => {
    const user = userEvent.setup();
    const onChangeSignature = vi.fn();
    renderPanel({
      signature: { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true, withRubric: true },
      rubric,
      onChangeSignature,
    });

    await user.click(screen.getByRole("radio", { name: "Solo rúbrica" }));

    expect(onChangeSignature).toHaveBeenCalledWith(
      expect.objectContaining({ content: { model: "rubricOnly" }, withRubric: true }),
    );
  });

  it("cannot choose the custom card yet", () => {
    renderPanel();

    expect(screen.getByRole("radio", { name: "Personalizada" })).toBeDisabled();
  });

  it("turns «Con rúbrica» off with a click, when nothing locks it", async () => {
    const user = userEvent.setup();
    const onChangeSignature = vi.fn();
    renderPanel({
      signature: { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true, withRubric: true },
      rubric,
      onChangeSignature,
    });

    await user.click(screen.getByRole("switch", { name: "Con rúbrica" }));

    expect(onChangeSignature).toHaveBeenCalledWith(expect.objectContaining({ withRubric: false }));
  });

  it("shows the rubric already normalized, over white, before signing", () => {
    renderPanel({ rubric });

    const thumbnail = screen.getByAltText("Tu rúbrica, tal como se estampará");
    expect(thumbnail).toHaveAttribute("src", rubric.dataUrl);
    expect(screen.getByText(/Se estampa sobre blanco/)).toBeInTheDocument();
  });

  it("disables the rubric-only card while «Con rúbrica» is off", () => {
    renderPanel({ rubric: null });

    expect(screen.getByRole("radio", { name: "Solo rúbrica" })).toBeDisabled();
  });

  it("locks «Con rúbrica» on when the rubric-only card is chosen", () => {
    renderPanel({
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        enabled: true,
        withRubric: true,
        content: { model: "rubricOnly" },
      },
      rubric,
    });

    const rubricSwitch = screen.getByRole("switch", { name: "Con rúbrica" });
    expect(rubricSwitch).toBeDisabled();
    expect(rubricSwitch).toHaveAttribute("aria-checked", "true");
  });

  it("shows a dashed hole in place of the thumbnail once switched on without an image", () => {
    renderPanel({
      signature: { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true, withRubric: true },
      rubric: null,
    });

    // El hueco aparece en la fila y en la tarjeta *Completa*, que también
    // lleva la rúbrica encendida (docs/design/panel-de-firma.md § La rúbrica).
    expect(screen.getAllByTitle("Sin rúbrica cargada").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Cargar…" })).toBeInTheDocument();
  });

  it("counts the rubric failure as it is chosen, with the raw detail apart", () => {
    renderPanel({
      rubricFailure: { situation: "notAnAcceptedImage", detail: "image/gif" },
    });

    expect(screen.getByText("Esa imagen no vale como rúbrica")).toBeInTheDocument();
    expect(screen.getByText("image/gif")).toBeInTheDocument();
  });
});
