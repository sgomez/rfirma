import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { StoreDetail, WithdrawalReport } from "./status";
import { WithdrawCertificateDialog } from "./WithdrawCertificateDialog";

const STORES: StoreDetail[] = [
  { brand: "firefox", trusted: true },
  { brand: "chrome", trusted: true },
];

const SUCCESSFUL_REPORT: WithdrawalReport = {
  handler: { kind: "withdrawn" },
  stores: [
    { brand: "firefox", outcome: { kind: "withdrawn" } },
    { brand: "chrome", outcome: { kind: "wasNotThere" } },
  ],
};

function renderDialog(props: Partial<Parameters<typeof WithdrawCertificateDialog>[0]> = {}) {
  return renderWithCatalog(
    <WithdrawCertificateDialog
      stores={STORES}
      onWithdraw={vi.fn().mockResolvedValue(SUCCESSFUL_REPORT)}
      onClose={vi.fn()}
      {...props}
    />,
  );
}

// Grada A: el velo de docs/design/retirar-certificado.md, ID-367, ID-376.
describe("WithdrawCertificateDialog", () => {
  it("is an alertdialog that traps focus and starts with the question", () => {
    renderDialog();

    const dialog = screen.getByRole("alertdialog", { name: "Retirar el certificado de rFirma" });
    expect(dialog).toBeVisible();
    expect(dialog).toHaveFocus();
    expect(
      screen.getByText("El certificado de rFirma, de los navegadores donde esté"),
    ).toBeInTheDocument();
    expect(screen.getByText("Que las sedes abran rFirma")).toBeInTheDocument();
    expect(screen.getByText("Se puede volver a instalar desde este panel.")).toBeInTheDocument();
  });

  it("cancels without withdrawing", async () => {
    const user = userEvent.setup();
    const onWithdraw = vi.fn();
    const onClose = vi.fn();
    renderDialog({ onWithdraw, onClose });

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onWithdraw).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("withdraws with a disabled Cerrar while working, and shows the restart notice always on the result", async () => {
    const user = userEvent.setup();
    let resolveWithdraw!: (report: WithdrawalReport) => void;
    const withdrawPromise = new Promise<WithdrawalReport>((resolve) => {
      resolveWithdraw = resolve;
    });
    renderDialog({ onWithdraw: vi.fn().mockReturnValue(withdrawPromise) });

    await user.click(screen.getByRole("button", { name: "Retirar" }));

    expect(screen.getByRole("alertdialog", { name: "Retirando…" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeDisabled();

    resolveWithdraw(SUCCESSFUL_REPORT);

    await waitFor(() => {
      expect(screen.getByRole("alertdialog", { name: "Certificado retirado" })).toBeVisible();
    });
    expect(
      screen.getByText(
        "Reinicia el navegador para que deje de confiar en el certificado retirado.",
      ),
    ).toBeInTheDocument();
  });

  it("closes on Cerrar after a successful withdrawal", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderDialog({ onClose });

    await user.click(screen.getByRole("button", { name: "Retirar" }));
    await screen.findByRole("alertdialog", { name: "Certificado retirado" });
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("offers Reintentar next to Cerrar, and titles the partial outcome, when something failed", async () => {
    const user = userEvent.setup();
    const partialReport: WithdrawalReport = {
      handler: { kind: "withdrawn" },
      stores: [
        { brand: "firefox", outcome: { kind: "withdrawn" } },
        { brand: "chrome", outcome: { kind: "failed", reason: "perfil en uso" } },
      ],
    };
    const onWithdraw = vi.fn().mockResolvedValue(partialReport);
    renderDialog({ onWithdraw });

    await user.click(screen.getByRole("button", { name: "Retirar" }));

    const dialog = await screen.findByRole("alertdialog", { name: "Retirado a medias" });
    expect(screen.getByText("perfil en uso")).toBeInTheDocument();
    expect(dialog).toHaveTextContent("Cerrar");
    expect(screen.getByRole("button", { name: "Reintentar" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reintentar" }));

    expect(onWithdraw).toHaveBeenCalledTimes(2);
    expect(onWithdraw).toHaveBeenNthCalledWith(1, null);
    expect(onWithdraw).toHaveBeenNthCalledWith(2, partialReport);
  });

  it("moves to the full outcome once a retry fixes what had failed", async () => {
    const user = userEvent.setup();
    const partialReport: WithdrawalReport = {
      handler: { kind: "withdrawn" },
      stores: [
        { brand: "firefox", outcome: { kind: "withdrawn" } },
        { brand: "chrome", outcome: { kind: "failed", reason: "perfil en uso" } },
      ],
    };
    const onWithdraw = vi
      .fn()
      .mockResolvedValueOnce(partialReport)
      .mockResolvedValueOnce(SUCCESSFUL_REPORT);
    renderDialog({ onWithdraw });

    await user.click(screen.getByRole("button", { name: "Retirar" }));
    await screen.findByRole("alertdialog", { name: "Retirado a medias" });
    await user.click(screen.getByRole("button", { name: "Reintentar" }));

    await screen.findByRole("alertdialog", { name: "Certificado retirado" });
    expect(screen.queryByRole("button", { name: "Reintentar" })).not.toBeInTheDocument();
  });

  it("ignores Escape while working, but closes on Escape from the question", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const neverResolves = new Promise<WithdrawalReport>(() => {});
    renderDialog({ onWithdraw: vi.fn().mockReturnValue(neverResolves), onClose });

    await user.click(screen.getByRole("button", { name: "Retirar" }));
    await user.keyboard("{Escape}");
    expect(onClose).not.toHaveBeenCalled();
  });
});
