import { screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ClientKind } from "../contract/ClientKind";
import {
  aKnownBug,
  aReportView,
  aSnapshot,
  withABug,
  withADeprecatedFormat,
} from "../test/fixtures";
import { renderConsoleAt } from "../test/render";

const THE_CHECK = "empty_uri_rejected";
const A_FAILED_CHECK = "unsupported_protocol_uri_rejected";

function aReportWhereABugFails(kind: ClientKind, master: "present" | "fixed" | "partial") {
  const report = { ...withABug(aReportView(), A_FAILED_CHECK, aKnownBug(master)), kind };
  return aSnapshot({ report });
}

function aReportOfRfirmaWhereADeprecatedFormatFails() {
  const report = {
    ...withADeprecatedFormat(aReportView(), A_FAILED_CHECK),
    kind: "rfirma" as const,
  };
  return aSnapshot({ report });
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

  it("labels a check that a known bug of AutoFirma fails with the state of the bug in master", async () => {
    renderConsoleAt("/", aReportWhereABugFails("rfirma", "present"));
    const row = await theRowOf(A_FAILED_CHECK);

    const label = within(row).getByText("Bug AutoFirma 1.9.2 · sigue en master");

    expect(label).toHaveAttribute("title", `BUG-15: ${aKnownBug().title}`);
  });

  it.each([
    ["fixed", "Bug AutoFirma 1.9.2 · corregido en master"],
    ["partial", "Bug AutoFirma 1.9.2 · corregido a medias en master"],
  ] as const)("names a bug that is %s in master", async (master, label) => {
    renderConsoleAt("/", aReportWhereABugFails("autofirma", master));

    expect(within(await theRowOf(A_FAILED_CHECK)).getByText(label)).toBeInTheDocument();
  });

  it("leaves unlabelled a check that no known bug fails", async () => {
    renderConsoleAt("/", aReportWhereABugFails("autofirma", "present"));

    expect(within(await theRowOf(THE_CHECK)).queryByText(/Bug AutoFirma/)).toBeNull();
  });

  it("shows the id and the title of the bug in the detail of the check", async () => {
    const { user } = renderConsoleAt("/", aReportWhereABugFails("autofirma", "present"));

    await user.click(await theRowOf(A_FAILED_CHECK));

    const detail = screen.getByText("Bug conocido").closest(".field") as HTMLElement;
    expect(detail).toHaveTextContent(`BUG-15 ${aKnownBug().title}`);
  });

  it("marks as expected a noncompliance of AutoFirma that its known bug explains", async () => {
    const { user } = renderConsoleAt("/", aReportWhereABugFails("autofirma", "present"));
    const row = await theRowOf(A_FAILED_CHECK);

    await user.click(row);

    expect(theItemOf(row)).toHaveAttribute("data-expected");
    expect(screen.getByText(/esperado por el bug/)).toBeInTheDocument();
  });

  it("does not mark as expected the same noncompliance in a report of rFirma", async () => {
    renderConsoleAt("/", aReportWhereABugFails("rfirma", "present"));

    expect(theItemOf(await theRowOf(A_FAILED_CHECK))).not.toHaveAttribute("data-expected");
  });

  it("labels a check of a deprecated format and says why in its detail", async () => {
    const { user } = renderConsoleAt("/", aReportOfRfirmaWhereADeprecatedFormatFails());
    const row = await theRowOf(A_FAILED_CHECK);

    await user.click(row);

    expect(within(row).getByText("Formato deprecado")).toBeInTheDocument();
    const detail = screen.getByText("Formato deprecado", { selector: "dt" }).closest(".field");
    expect(detail).toHaveTextContent(/AutoFirma lo soporta y rFirma no/);
  });

  it("does not count as a failure the noncompliance of rFirma in a deprecated format", async () => {
    const { user } = renderConsoleAt("/", aReportOfRfirmaWhereADeprecatedFormatFails());
    const row = await theRowOf(A_FAILED_CHECK);

    await user.click(row);

    expect(theItemOf(row)).toHaveAttribute("data-expected");
    expect(screen.getByText(/no cuenta como fallo/, { selector: ".muted" })).toBeInTheDocument();
    expect(
      screen.getAllByText(/1 deprecados/, { selector: ".counts .count" }).length,
    ).toBeGreaterThan(0);
  });

  it("leaves unlabelled a check of a supported format", async () => {
    renderConsoleAt("/", aReportOfRfirmaWhereADeprecatedFormatFails());

    expect(within(await theRowOf(THE_CHECK)).queryByText("Formato deprecado")).toBeNull();
  });
});
