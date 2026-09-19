import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { memoryStatus, type SignalRow } from "../status/status";
import { renderWithCatalog } from "../testing/render";
import { SetupWizard } from "./SetupWizard";

const aVersionRow: SignalRow = {
  signal: "version",
  value: "0.4.1",
  verdict: "correct",
  action: null,
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

const certificateNotInstalled: SignalRow = {
  signal: "localCaCertificate",
  value: "",
  verdict: "attention",
  action: { kind: "repair", target: "" },
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

const handlerNotOurs: SignalRow = {
  signal: "siteSignature",
  value: "AutoFirma",
  verdict: "attention",
  action: { kind: "choice", target: "rfirma.desktop" },
  detail: null,
  candidates: [
    { id: "autofirma.desktop", name: "AutoFirma", selected: true },
    { id: "rfirma.desktop", name: "rFirma", selected: false },
  ],
  restartFirefoxNotice: false,
};

// Grada A: el asistente solo habla con `StatusPort`, sin puerto propio.
describe("SetupWizard", () => {
  it("does not mount at all once a previous run has seen it", () => {
    renderWithCatalog(<SetupWizard seen={true} onFinish={() => {}} />);

    expect(screen.queryByText("Configurar rFirma")).not.toBeInTheDocument();
  });

  it("welcomes with the independence disclaimer and moves to the two actions on Continuar", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <SetupWizard
        seen={false}
        statusPort={memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs])}
        onFinish={() => {}}
      />,
    );

    expect(screen.getByText("Configurar rFirma")).toBeInTheDocument();
    expect(screen.getByText(/aplicación compatible con AutoFirma 1\.9\.2/)).toBeInTheDocument();
    expect(screen.getByText("Proyecto independiente.")).toBeInTheDocument();
    expect(screen.getByText("Paso 1 de 2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(screen.getByText("El certificado de rFirma")).toBeInTheDocument();
    expect(screen.getByText("Usar rFirma por defecto")).toBeInTheDocument();
    expect(screen.getByText("Paso 2 de 2")).toBeInTheDocument();
  });

  it("installs the certificate through the same use case as the status panel", async () => {
    const user = userEvent.setup();
    const installedRow: SignalRow = {
      ...certificateNotInstalled,
      verdict: "correct",
      action: null,
      restartFirefoxNotice: true,
    };
    renderWithCatalog(
      <SetupWizard
        seen={false}
        statusPort={memoryStatus(
          [aVersionRow, certificateNotInstalled, handlerNotOurs],
          undefined,
          installedRow,
        )}
        onFinish={() => {}}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    await user.click(screen.getByRole("button", { name: "Instalar" }));

    await waitFor(() => {
      expect(screen.getByText("Instalado en tus navegadores.")).toBeInTheDocument();
    });
    expect(
      screen.getByText("Si tienes alguno abierto, reinícialo para que lo reconozca."),
    ).toBeInTheDocument();
  });

  it("offers no button while the certificate card is working (docs/design/primer-arranque.md)", async () => {
    const user = userEvent.setup();
    let resolveInstall: (row: SignalRow) => void = () => {};
    const base = memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs]);
    const port = {
      ...base,
      installLocalCaCertificate: () =>
        new Promise<SignalRow>((resolve) => {
          resolveInstall = resolve;
        }),
    };
    renderWithCatalog(<SetupWizard seen={false} statusPort={port} onFinish={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    await user.click(screen.getByRole("button", { name: "Instalar" }));

    expect(screen.getByText("Instalando…")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();

    resolveInstall({
      ...certificateNotInstalled,
      verdict: "correct",
      action: null,
      restartFirefoxNotice: false,
    });
    await waitFor(() => {
      expect(screen.getByText("Instalado en tus navegadores.")).toBeInTheDocument();
    });
  });

  it("shows the per-store failure list and retries with the same action", async () => {
    const user = userEvent.setup();
    const failedRow: SignalRow = {
      ...certificateNotInstalled,
      verdict: "incorrect",
      detail: [
        { brand: "firefox", trusted: true },
        { brand: "nssdb", trusted: false },
      ],
    };
    const installedRow: SignalRow = {
      ...certificateNotInstalled,
      verdict: "correct",
      action: null,
      restartFirefoxNotice: false,
    };
    let calls = 0;
    const base = memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs]);
    const port = {
      ...base,
      installLocalCaCertificate: async () => {
        calls += 1;
        return calls === 1 ? failedRow : installedRow;
      },
    };
    renderWithCatalog(<SetupWizard seen={false} statusPort={port} onFinish={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    await user.click(screen.getByRole("button", { name: "Instalar" }));

    await waitFor(() => {
      expect(screen.getByText("No se ha podido instalar en todas partes.")).toBeInTheDocument();
    });
    expect(screen.getByText("Firefox")).toBeInTheDocument();
    expect(screen.getByText("Almacén del sistema")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reintentar" }));

    await waitFor(() => {
      expect(screen.getByText("Instalado en tus navegadores.")).toBeInTheDocument();
    });
  });

  it("chooses rFirma as the site signature handler through the same use case as the status panel", async () => {
    const user = userEvent.setup();
    const chosenHandler: SignalRow = {
      ...handlerNotOurs,
      value: "rFirma",
      verdict: "correct",
      action: null,
      candidates: [
        { id: "autofirma.desktop", name: "AutoFirma", selected: false },
        { id: "rfirma.desktop", name: "rFirma", selected: true },
      ],
    };
    const chosenCertificate: SignalRow = {
      ...certificateNotInstalled,
      verdict: "correct",
      action: null,
    };
    renderWithCatalog(
      <SetupWizard
        seen={false}
        statusPort={memoryStatus(
          [aVersionRow, certificateNotInstalled, handlerNotOurs],
          undefined,
          undefined,
          [chosenCertificate, chosenHandler],
        )}
        onFinish={() => {}}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    await user.click(screen.getByRole("button", { name: "Que abran rFirma" }));

    await waitFor(() => {
      expect(screen.getByText("Ahora abren rFirma.")).toBeInTheDocument();
    });
    expect(screen.getByText("Instalado en tus navegadores.")).toBeInTheDocument();
  });

  it("declines an action without touching the status port", async () => {
    const user = userEvent.setup();
    const port = memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs]);
    const install = vi.spyOn(port, "installLocalCaCertificate");
    renderWithCatalog(<SetupWizard seen={false} statusPort={port} onFinish={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    const firstNotNow = screen.getAllByRole("button", { name: "Ahora no" }).at(0);
    if (firstNotNow === undefined) throw new Error("esperaba una tarjeta con «Ahora no»");
    await user.click(firstNotNow);

    expect(install).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();
  });

  it("uses the alternate copy when AutoFirma does not appear among the candidates", async () => {
    const user = userEvent.setup();
    const handlerAlone: SignalRow = {
      ...handlerNotOurs,
      value: "",
      action: { kind: "choice", target: "rfirma.desktop" },
      candidates: [{ id: "rfirma.desktop", name: "rFirma", selected: false }],
    };
    renderWithCatalog(
      <SetupWizard
        seen={false}
        statusPort={memoryStatus([aVersionRow, certificateNotInstalled, handlerAlone])}
        onFinish={() => {}}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(
      screen.getByText("Ahora mismo las sedes electrónicas no tienen ningún programa asignado."),
    ).toBeInTheDocument();
  });

  it("marks setupWizardSeen on Terminar regardless of what the two cards did", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    renderWithCatalog(
      <SetupWizard
        seen={false}
        statusPort={memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs])}
        onFinish={onFinish}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    await user.click(screen.getByRole("button", { name: "Terminar" }));

    expect(onFinish).toHaveBeenCalledOnce();
  });

  // La cabecera es la del ADR-0007, con su menú completo (docs/design/primer-arranque.md).
  it("carries the app name and the ADR-0007 menu, reusing the shared Header", async () => {
    const user = userEvent.setup();
    const onOpenStatus = vi.fn();
    const onOpenPreferences = vi.fn();
    const onOpenHelp = vi.fn();
    const onOpenAbout = vi.fn();
    renderWithCatalog(
      <SetupWizard
        seen={false}
        onFinish={() => {}}
        onOpenStatus={onOpenStatus}
        onOpenPreferences={onOpenPreferences}
        onOpenHelp={onOpenHelp}
        onOpenAbout={onOpenAbout}
      />,
    );

    expect(screen.getByText("rFirma")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    expect(screen.getByRole("menuitem", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Preferencias…" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: /Comentarios y ayuda/ })).toBeInTheDocument();

    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));

    expect(onOpenAbout).toHaveBeenCalledOnce();
    expect(onOpenStatus).not.toHaveBeenCalled();
    expect(onOpenPreferences).not.toHaveBeenCalled();
    expect(onOpenHelp).not.toHaveBeenCalled();
  });
});
