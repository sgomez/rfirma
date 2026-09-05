import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { UrlHandlerBanner } from "./UrlHandlerBanner";

const noop = () => {};

function renderBanner(props: Partial<Parameters<typeof UrlHandlerBanner>[0]> = {}) {
  return renderWithCatalog(
    <UrlHandlerBanner onAccept={noop} onLater={noop} onNever={noop} {...props} />,
  );
}

// Grada A: el banner no habla con nadie; las tres respuestas son suyas.
describe("UrlHandlerBanner", () => {
  it("offers the three answers and nothing else", () => {
    renderBanner();

    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      "Sí",
      "Ahora no",
      "No volver a preguntar",
    ]);
  });

  it("takes the shortcut of the setting when the answer is yes", async () => {
    const user = userEvent.setup();
    const onAccept = vi.fn();
    renderBanner({ onAccept });

    await user.click(screen.getByRole("button", { name: "Sí" }));

    expect(onAccept).toHaveBeenCalledOnce();
  });

  it("keeps «not now» apart from «do not ask again»", async () => {
    const user = userEvent.setup();
    const onLater = vi.fn();
    const onNever = vi.fn();
    renderBanner({ onLater, onNever });

    await user.click(screen.getByRole("button", { name: "Ahora no" }));

    expect(onLater).toHaveBeenCalledOnce();
    expect(onNever).not.toHaveBeenCalled();
  });

  it("turns the question off for good when asked to", async () => {
    const user = userEvent.setup();
    const onNever = vi.fn();
    renderBanner({ onNever });

    await user.click(screen.getByRole("button", { name: "No volver a preguntar" }));

    expect(onNever).toHaveBeenCalledOnce();
  });
});
