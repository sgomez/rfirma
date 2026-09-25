import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Summary } from "../contract/Summary";
import { Counts } from "./SummaryBar";

function aSummaryWith(noncompliant: number, explained: number): Summary {
  return {
    total: 10,
    compliant: 10 - noncompliant,
    noncompliant,
    explained,
    not_observable: 0,
    pending: 0,
  };
}

describe("the counts", () => {
  it("agrees in number with a single explained noncompliance", () => {
    render(<Counts summary={aSummaryWith(2, 1)} />);

    expect(screen.getByTitle(/^1 sin explicar · 1 explicado\./)).toBeVisible();
  });

  it("agrees in number with several explained noncompliances", () => {
    render(<Counts summary={aSummaryWith(3, 2)} />);

    expect(screen.getByTitle(/^1 sin explicar · 2 explicados\./)).toBeVisible();
  });

  it("shows the explained noncompliances in a short badge", () => {
    render(<Counts summary={aSummaryWith(3, 2)} />);

    expect(screen.getByText("2 expl.", { selector: ".explained-count" })).toBeVisible();
  });

  it("leaves out the badge when nothing fails", () => {
    render(<Counts summary={aSummaryWith(0, 0)} />);

    expect(screen.queryByText(/expl\./)).toBeNull();
  });
});
