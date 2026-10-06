import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { sitesHandledByRfirma, versionUpToDate } from "./testing/fixtures";

const noop = () => {};

function renderView(onClose: () => void = noop) {
  return renderWithCatalog(
    <StatusView
      rows={[versionUpToDate, sitesHandledByRfirma]}
      onClose={onClose}
      onRecheck={noop}
      onAction={noop}
      onChooseSiteSignatureHandler={noop}
      onWithdraw={noop}
    />,
  );
}

describe("StatusView", () => {
  it("asks to close from the footer button", async () => {
    const onClose = vi.fn();
    renderView(onClose);

    await userEvent.setup().click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("keeps the close button in a footer that is sibling to the scrollable body (WCAG 2.4.11)", () => {
    const { container } = renderView();

    const body = container.querySelector(".status-view__body");
    const footer = container.querySelector(".status-view__footer");

    expect(body).not.toBeNull();
    expect(footer).not.toBeNull();
    expect(body?.nextElementSibling).toBe(footer);
    expect(footer?.parentElement).toBe(body?.parentElement);
  });
});
