import { screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ClientKind } from "../contract/ClientKind";
import type { LabelView } from "../contract/LabelView";
import { A_BUG_LABEL, AN_ADR_LABEL, aReportView, aSnapshot, withLabels } from "../test/fixtures";
import { renderConsoleAt } from "../test/render";

const THE_CHECK = "empty_uri_rejected";
const A_FAILED_CHECK = "unsupported_protocol_uri_rejected";
const A_COMPLIANT_CHECK = "greeting_opens_the_channel";

function aReportOf(kind: ClientKind, id: string, labels: LabelView[]) {
  return aSnapshot({ report: { ...withLabels(aReportView(), id, labels), kind } });
}

function theItemOf(row: HTMLElement) {
  return row.closest("li") as HTMLElement;
}

async function theRowOf(id: string) {
  const name = await screen.findByText(id, { selector: ".check-id" });
  return name.closest("[data-check-row]") as HTMLElement;
}

describe("a check row", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("keeps the id of the check outside any button so that it can be selected", async () => {
    renderConsoleAt("/");

    const name = await screen.findByText(THE_CHECK, { selector: ".check-id" });

    expect(name.closest("button")).toBeNull();
  });

  it("unfolds the check on a click that selects nothing", async () => {
    const { user } = renderConsoleAt("/");
    const row = await theRowOf(THE_CHECK);

    await user.click(row);

    expect(row).toHaveAttribute("aria-expanded", "true");
  });

  it("does not unfold the check when the click ends a text selection", async () => {
    const { user } = renderConsoleAt("/");
    const row = await theRowOf(THE_CHECK);
    vi.spyOn(window, "getSelection").mockReturnValue({
      isCollapsed: false,
      toString: () => THE_CHECK,
    } as Selection);

    await user.click(row);

    expect(row).toHaveAttribute("aria-expanded", "false");
  });

  it("copies the id of the check to the clipboard from its own button", async () => {
    const { user } = renderConsoleAt("/");

    await user.click(await screen.findByRole("button", { name: `Copiar ${THE_CHECK}` }));

    expect(await navigator.clipboard.readText()).toBe(THE_CHECK);
  });

  it.each(["autofirma", "rfirma"] as const)(
    "shows every label of a check with its reason in a report of %s",
    async (kind) => {
      renderConsoleAt("/", aReportOf(kind, A_FAILED_CHECK, [A_BUG_LABEL, AN_ADR_LABEL]));
      const row = await theRowOf(A_FAILED_CHECK);

      expect(within(row).getByText("autofirma:bug:1.9.2")).toHaveAttribute(
        "title",
        A_BUG_LABEL.reason,
      );
      expect(within(row).getByText("rfirma:adr-0010")).toHaveAttribute(
        "title",
        AN_ADR_LABEL.reason,
      );
    },
  );

  it("leaves unlabelled a check without labels", async () => {
    renderConsoleAt("/", aReportOf("rfirma", A_FAILED_CHECK, [AN_ADR_LABEL]));

    expect(within(await theRowOf(THE_CHECK)).queryByText(/rfirma:adr/)).toBeNull();
  });

  it("shows each label with its reason in the detail of the check", async () => {
    const { user } = renderConsoleAt("/", aReportOf("rfirma", A_FAILED_CHECK, [A_BUG_LABEL]));

    await user.click(await theRowOf(A_FAILED_CHECK));

    const detail = screen.getByText("Etiquetas", { selector: "dt" }).closest(".field");
    expect(detail).toHaveTextContent(`autofirma:bug:1.9.2 ${A_BUG_LABEL.reason}`);
  });

  it.each(["autofirma", "rfirma"] as const)(
    "marks as explained a labelled noncompliance in a report of %s",
    async (kind) => {
      const { user } = renderConsoleAt("/", aReportOf(kind, A_FAILED_CHECK, [AN_ADR_LABEL]));
      const row = await theRowOf(A_FAILED_CHECK);

      await user.click(row);

      expect(theItemOf(row)).toHaveAttribute("data-explained");
      expect(screen.getByText(/explicado por su etiqueta/, { selector: ".muted" })).toBeVisible();
    },
  );

  it("does not mark as explained a noncompliance without labels", async () => {
    renderConsoleAt("/", aReportOf("rfirma", THE_CHECK, [AN_ADR_LABEL]));

    expect(theItemOf(await theRowOf(A_FAILED_CHECK))).not.toHaveAttribute("data-explained");
  });

  it("does not mark as explained a labelled check that complies", async () => {
    renderConsoleAt("/", aReportOf("rfirma", A_COMPLIANT_CHECK, [AN_ADR_LABEL]));

    expect(theItemOf(await theRowOf(A_COMPLIANT_CHECK))).not.toHaveAttribute("data-explained");
  });

  it("counts the unexplained noncompliances apart from the explained ones", async () => {
    renderConsoleAt("/", aReportOf("rfirma", A_FAILED_CHECK, [AN_ADR_LABEL]));
    await theRowOf(A_FAILED_CHECK);

    expect(
      screen.getAllByText("0 sin explicar · 1 explicados", { selector: ".counts .count" }).length,
    ).toBeGreaterThan(0);
  });

  it("counts as unexplained a noncompliance without labels", async () => {
    renderConsoleAt("/", aReportOf("autofirma", THE_CHECK, [A_BUG_LABEL]));
    await theRowOf(A_FAILED_CHECK);

    expect(
      screen.getAllByText("1 sin explicar · 0 explicados", { selector: ".counts .count" }).length,
    ).toBeGreaterThan(0);
  });
});
