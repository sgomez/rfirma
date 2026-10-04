import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProgressBar } from "./ProgressBar";

describe("ProgressBar", () => {
  it("is a progressbar named by its caller, with the value and the default 0 to 100 range", () => {
    render(<ProgressBar value={40} aria-label="Firmando" />);

    const bar = screen.getByRole("progressbar", { name: "Firmando" });
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
    expect(bar).toHaveAttribute("aria-valuenow", "40");
  });

  it("fills the share of the range the value covers, with a custom range", () => {
    render(<ProgressBar value={2} min={1} max={5} aria-label="Etapa" />);

    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuemin", "1");
    expect(bar).toHaveAttribute("aria-valuemax", "5");
    expect(bar.firstElementChild).toHaveStyle({ width: "25%" });
  });

  it("can be named by a labelling element", () => {
    render(
      <>
        <h2 id="title">Firmando</h2>
        <ProgressBar value={1} aria-labelledby="title" />
      </>,
    );

    expect(screen.getByRole("progressbar", { name: "Firmando" })).toBeInTheDocument();
  });
});
