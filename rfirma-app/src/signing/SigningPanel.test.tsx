import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { placementStateOf } from "../placement/testing/fixtures";
import {
  aCertificateSection,
  aSigningSection,
  aVisibleSignatureSection,
  certificate,
  previousSignatureOf,
  renderPanel,
  reportOf,
} from "./testing/harness";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

// Lo que se ve en cada estado, en `SigningPanel.presentation.test.tsx`.
describe("SigningPanel", () => {
  it("opens the «Ver firmas» dialog from «Ver firmas →» and closes it", async () => {
    const user = userEvent.setup();
    renderPanel({ previousSignatures: reportOf([previousSignatureOf()]) });

    await user.click(screen.getByRole("button", { name: "Ver firmas →" }));

    const dialog = screen.getByRole("dialog", { name: "Firmas del documento" });
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("asks to look for certificates again when none turned up", async () => {
    const user = userEvent.setup();
    const lookAgain = vi.fn();
    renderPanel({ certificate: aCertificateSection({ kind: "empty" }, { lookAgain }) });

    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(lookAgain).toHaveBeenCalled();
  });

  it("calls onBack from the error's «Volver»", async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();
    renderPanel({
      failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)" },
      signing: aSigningSection({ back: onBack }),
    });

    await user.click(screen.getByRole("button", { name: "Volver" }));

    expect(onBack).toHaveBeenCalled();
  });

  // Uno recordado puede caducar entre sesiones (ADR-0010).
  it.each([
    ["expired", { kind: "expired", notAfter: 1_767_225_600 }],
    ["revoked", { kind: "revoked", reason: "keyCompromise" }],
  ] as const)("refuses to sign with a %s chosen certificate", (_kind, status) => {
    const unusable = { ...certificate, status };
    renderPanel({
      certificate: aCertificateSection({
        kind: "chosen",
        certificate: unusable,
        certificates: [unusable],
      }),
    });

    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  it("chooses the certificate from the selector, not from the footer", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const grace = { ...certificate, id: "otra", holderName: "Grace Hopper" };
    renderPanel({
      certificate: aCertificateSection(
        { kind: "unchosen", certificates: [certificate, grace] },
        { choose },
      ),
    });

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    await user.click(screen.getByRole("option", { name: /Grace Hopper/ }));

    expect(choose).toHaveBeenCalledWith(grace);
  });
});

describe("la firma visible, al cambiar el certificado", () => {
  const visible = aVisibleSignatureSection({ ...DEFAULT_VISIBLE_SIGNATURE, enabled: true });
  const hidden = aVisibleSignatureSection({ ...DEFAULT_VISIBLE_SIGNATURE, enabled: false });

  it("brings the placement back when a certificate that went away comes back", () => {
    const { show } = renderPanel({ signature: visible });
    expect(screen.getByText("En la página 3")).toBeInTheDocument();

    show({ certificate: aCertificateSection({ kind: "empty" }), signature: visible });
    expect(screen.queryByText("En la página 3")).not.toBeInTheDocument();
    show({ signature: visible });

    expect(screen.getByText("En la página 3")).toBeInTheDocument();
  });

  it("shows no placement block with the visible signature off", () => {
    renderPanel({ signature: hidden });

    expect(screen.queryByRole("radiogroup", { name: "En qué páginas" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
  });

  it("turns the sign button off while the typed pages make no sense", () => {
    renderPanel({
      signature: visible,
      placementState: placementStateOf({
        rect: { x0: 100, y0: 100, x1: 300, y1: 180 },
        sets: { single: 3, these: { only: [10, 40] } },
        mode: "these",
        pageCount: 6,
      }),
    });

    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });
});
