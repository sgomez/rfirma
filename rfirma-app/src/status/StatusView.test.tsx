import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { memoryStatus, type SignalRow, type StatusPort } from "./status";

const stillChecking: SignalRow = {
  signal: "localCaCertificate",
  value: "",
  verdict: "checking",
  action: null,
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

describe("StatusView", () => {
  it("renders the title and close button", () => {
    renderWithCatalog(<StatusView onClose={() => {}} />);

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  it("calls onClose when clicking Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose when pressing Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not call onClose when Escape was default-prevented", () => {
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("keeps the close button in a footer that is sibling to the scrollable body (WCAG 2.4.11)", () => {
    const { container } = renderWithCatalog(<StatusView onClose={() => {}} />);

    const body = container.querySelector(".status-view__body");
    const footer = container.querySelector(".status-view__footer");

    expect(body).not.toBeNull();
    expect(footer).not.toBeNull();
    expect(body?.nextElementSibling).toBe(footer);
    expect(footer?.parentElement).toBe(body?.parentElement);
  });

  it("renders the status table columns: Señal, Valor, Veredicto, Acción", () => {
    renderWithCatalog(<StatusView onClose={() => {}} />);

    expect(screen.getByText("Señal")).toBeInTheDocument();
    expect(screen.getByText("Valor")).toBeInTheDocument();
    expect(screen.getByText("Veredicto")).toBeInTheDocument();
    expect(screen.getByText("Acción")).toBeInTheDocument();
  });

  it("renders a row with role status showing Correcto when up to date", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Versión")).toBeInTheDocument();
    expect(within(row).getByText("0.4.1")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders Atención and an update link action when an update is available", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1 → 0.5.0",
        verdict: "attention",
        action: {
          kind: "link",
          target: "releases",
        },
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Versión")).toBeInTheDocument();
    expect(within(row).getByText("0.4.1 → 0.5.0")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Actualizar" })).toBeInTheDocument();
  });

  it("renders rFirma and Correcto when rFirma signs at sites", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Firma en sedes")).toBeInTheDocument();
    expect(within(row).getByText("rFirma")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders Sin configurar and Atención when no program is configured", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "attention",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Sin configurar")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders No se puede consultar and No aplica without action inside the sandbox", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "notApplicable",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("No se puede consultar")).toBeInTheDocument();
    expect(within(row).getByText("No aplica")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders plain text with no dropdown when there is only one candidate", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("rFirma")).toBeInTheDocument();
    expect(within(row).queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("renders a dropdown with the current candidate selected when there are two or more", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "AutoFirma",
        verdict: "attention",
        action: {
          kind: "choice",
          target: "rfirma.desktop",
        },
        detail: null,
        candidates: [
          { id: "rfirma.desktop", name: "rFirma", selected: false },
          { id: "autofirma.desktop", name: "AutoFirma", selected: true },
        ],
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    const dropdown = within(row).getByRole("combobox");
    expect(dropdown).toHaveTextContent("AutoFirma");
    expect(within(row).getByRole("button", { name: "Usar rFirma" })).toBeInTheDocument();
  });

  it("chooses rFirma from Usar rFirma and remeasures both signals through Comprobando", async () => {
    const user = userEvent.setup();
    let resolveChoose!: (rows: SignalRow[]) => void;
    const choosePromise = new Promise<SignalRow[]>((resolve) => {
      resolveChoose = resolve;
    });
    const chooseSiteSignatureHandler = vi.fn().mockReturnValue(choosePromise);

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "siteSignature",
          value: "AutoFirma",
          verdict: "attention",
          action: {
            kind: "choice",
            target: "rfirma.desktop",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
        {
          signal: "localCaCertificate",
          value: "0/2",
          verdict: "incorrect",
          action: {
            kind: "repair",
            target: "installLocalCaCertificate",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler,
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const rows = await screen.findAllByRole("status");
    const siteRow = rows[0];
    const certRow = rows[1];
    if (!siteRow || !certRow) throw new Error("deberian existir las dos filas");

    await user.click(within(siteRow).getByRole("button", { name: "Usar rFirma" }));

    expect(chooseSiteSignatureHandler).toHaveBeenCalledWith("rfirma.desktop");
    expect(within(siteRow).getByText("Comprobando")).toBeInTheDocument();
    expect(within(certRow).getByText("Comprobando")).toBeInTheDocument();

    resolveChoose([
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(siteRow).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(certRow).getByText("Correcto")).toBeInTheDocument();
  });

  it("chooses AutoFirma from the dropdown without touching the certificate", async () => {
    const user = userEvent.setup();
    const chooseSiteSignatureHandler = vi.fn().mockResolvedValue([
      {
        signal: "siteSignature",
        value: "AutoFirma",
        verdict: "attention",
        action: {
          kind: "choice",
          target: "rfirma.desktop",
        },
        detail: null,
        candidates: [
          { id: "rfirma.desktop", name: "rFirma", selected: false },
          { id: "autofirma.desktop", name: "AutoFirma", selected: true },
        ],
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "siteSignature",
          value: "rFirma",
          verdict: "correct",
          action: null,
          detail: null,
          candidates: [
            { id: "rfirma.desktop", name: "rFirma", selected: true },
            { id: "autofirma.desktop", name: "AutoFirma", selected: false },
          ],
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler,
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("combobox"));
    await user.click(within(row).getByRole("option", { name: "AutoFirma" }));

    expect(chooseSiteSignatureHandler).toHaveBeenCalledWith("autofirma.desktop");
    await waitFor(() => {
      expect(within(row).getByText("AutoFirma")).toBeInTheDocument();
    });
  });

  it("renders Ninguno and Cómo instalar when no certificate stores are detected", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "0",
        verdict: "attention",
        action: {
          kind: "link",
          target: "certificateIssuance",
        },
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Tus certificados")).toBeInTheDocument();
    expect(within(row).getByText("Ninguno")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Cómo instalar" })).toBeInTheDocument();
  });

  it("renders the store count and Correcto without action when certificates are detected", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "3",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Tus certificados")).toBeInTheDocument();
    expect(within(row).getByText("3 almacenes")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders the local CA certificate signal born in Comprobando with no action", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "",
        verdict: "checking",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificado de rFirma")).toBeInTheDocument();
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("measures the local CA certificate signal on opening, without Volver a comprobar", async () => {
    const born: SignalRow = {
      signal: "localCaCertificate",
      value: "",
      verdict: "checking",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    const measured: SignalRow = {
      signal: "localCaCertificate",
      value: "3/3",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    renderWithCatalog(
      <StatusView
        statusPort={memoryStatus([born], undefined, undefined, undefined, undefined, measured)}
        onClose={() => {}}
      />,
    );

    const row = await screen.findByRole("status");
    await waitFor(() => {
      expect(within(row).getByText("3 de 3 almacenes")).toBeInTheDocument();
    });
    expect(within(row).queryByText("Comprobando")).not.toBeInTheDocument();
  });

  it("renders Incorrecto with Instalar and a store detail list when the certificate is nowhere trusted", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "0/2",
        verdict: "incorrect",
        action: {
          kind: "repair",
          target: "installLocalCaCertificate",
        },
        detail: [
          { brand: "firefox", trusted: false },
          { brand: "chrome", trusted: false },
        ],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificado de rFirma")).toBeInTheDocument();
    expect(within(row).getByText("0 de 2 almacenes")).toBeInTheDocument();
    expect(within(row).getByText("Incorrecto")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: /almacenes/ })).toBeInTheDocument();

    const toggle = within(row).getByRole("button", { name: "Ver almacenes" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(row).queryByText("Firefox")).not.toBeInTheDocument();

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(within(row).getByText("Firefox")).toBeInTheDocument();
    expect(within(row).getByText("Chrome y Chromium")).toBeInTheDocument();
  });

  it("renders Correcto with a trusted detail list when the certificate is in every store", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: [
          { brand: "firefox", trusted: true },
          { brand: "chrome", trusted: true },
        ],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("2 de 2 almacenes")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();

    await user.click(within(row).getByRole("button", { name: "Ver almacenes" }));

    expect(within(row).getAllByText("De confianza")).toHaveLength(2);
  });

  it("remeasures the certificate stores row on Volver a comprobar", async () => {
    const user = userEvent.setup();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "userCertificates",
          value: "0",
          verdict: "attention",
          action: {
            kind: "link",
            target: "certificateIssuance",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Ninguno")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Volver a comprobar" }));

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    resolveRecheck([
      {
        signal: "userCertificates",
        value: "1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("1 almacén")).toBeInTheDocument();
  });

  it("notifies onRowsChange with the rows read at startup", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const onRowsChange = vi.fn();
    renderWithCatalog(
      <StatusView statusPort={memoryStatus(rows)} onClose={() => {}} onRowsChange={onRowsChange} />,
    );

    await screen.findByRole("status");

    await waitFor(() => {
      expect(onRowsChange).toHaveBeenCalledWith(rows);
    });
  });

  it("notifies onRowsChange again once Volver a comprobar remeasures", async () => {
    const user = userEvent.setup();
    const initialRow: SignalRow = {
      signal: "version",
      value: "0.4.1",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    const initialRows: SignalRow[] = [initialRow];
    const recheckedRows: SignalRow[] = [{ ...initialRow, verdict: "attention", value: "0.5.0" }];
    const onRowsChange = vi.fn();
    renderWithCatalog(
      <StatusView
        statusPort={memoryStatus(initialRows, recheckedRows)}
        onClose={() => {}}
        onRowsChange={onRowsChange}
      />,
    );

    await screen.findByRole("status");
    onRowsChange.mockClear();

    await user.click(screen.getByRole("button", { name: "Volver a comprobar" }));

    await waitFor(() => {
      expect(onRowsChange).toHaveBeenCalledWith(recheckedRows);
    });
  });

  it("transitions through Comprobando and remeasures after clicking an action", async () => {
    const user = userEvent.setup();
    const open = vi.fn();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "version",
          value: "0.4.1 → 0.5.0",
          verdict: "attention",
          action: {
            kind: "link",
            target: "releases",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(
      <StatusView statusPort={statusPort} externalDestinations={{ open }} onClose={() => {}} />,
    );

    const row = await screen.findByRole("status");
    const updateButton = within(row).getByRole("button", { name: "Actualizar" });
    await user.click(updateButton);

    expect(open).toHaveBeenCalledWith("releases");
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Actualizar" })).not.toBeInTheDocument();

    resolveRecheck([
      {
        signal: "version",
        value: "0.5.0",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("0.5.0")).toBeInTheDocument();
  });

  it("transitions row through Comprobando and updates on Volver a comprobar", async () => {
    const user = userEvent.setup();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "version",
          value: "0.4.1",
          verdict: "checking",
          action: null,
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    const recheckBtn = screen.getByRole("button", { name: "Volver a comprobar" });
    await user.click(recheckBtn);

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    resolveRecheck([
      {
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
  });

  it("installs the local CA certificate and shows the restart notice when Firefox was alive", async () => {
    const user = userEvent.setup();
    let resolveInstall!: (row: SignalRow) => void;
    const installPromise = new Promise<SignalRow>((resolve) => {
      resolveInstall = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "localCaCertificate",
          value: "0/2",
          verdict: "incorrect",
          action: {
            kind: "repair",
            target: "installLocalCaCertificate",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn().mockReturnValue(installPromise),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("button", { name: "Instalar" }));

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(statusPort.installLocalCaCertificate).toHaveBeenCalledOnce();

    resolveInstall({
      signal: "localCaCertificate",
      value: "2/2",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: true,
    });

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("Reinicia Firefox para que surta efecto.")).toBeInTheDocument();
  });

  it("does not show the restart notice when Firefox was not alive", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(screen.queryByText("Reinicia Firefox para que surta efecto.")).not.toBeInTheDocument();
  });

  // ID-363: el botón lo fija el veredicto, nunca los dos a la vez.
  it("offers Retirar… instead of Instalar when the certificate is trusted everywhere", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: [
          { brand: "firefox", trusted: true },
          { brand: "chrome", trusted: true },
        ],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByRole("button", { name: "Retirar…" })).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();
  });

  it("opens the withdrawal dialog from Retirar…, and remeasures both signals when it closes", async () => {
    const user = userEvent.setup();
    const initialRows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: [
          { brand: "firefox", trusted: true },
          { brand: "chrome", trusted: true },
        ],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const afterWithdrawal: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "attention",
        action: {
          kind: "choice",
          target: "rfirma.desktop",
        },
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "0/2",
        verdict: "incorrect",
        action: {
          kind: "repair",
          target: "installLocalCaCertificate",
        },
        detail: [
          { brand: "firefox", trusted: false },
          { brand: "chrome", trusted: false },
        ],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue(initialRows),
      recheck: vi.fn().mockResolvedValue(afterWithdrawal),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn().mockResolvedValue({
        handler: { kind: "withdrawn" },
        stores: [
          { brand: "firefox", outcome: { kind: "withdrawn" } },
          { brand: "chrome", outcome: { kind: "withdrawn" } },
        ],
      }),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    await screen.findAllByRole("status");
    await user.click(screen.getByRole("button", { name: "Retirar…" }));

    const dialog = await screen.findByRole("alertdialog", {
      name: "Retirar el certificado de rFirma",
    });
    await user.click(within(dialog).getByRole("button", { name: "Retirar" }));
    const result = await screen.findByRole("alertdialog", { name: "Certificado retirado" });
    await user.click(within(result).getByRole("button", { name: "Cerrar" }));

    expect(statusPort.withdrawRfirma).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    await waitFor(() => {
      expect(statusPort.recheck).toHaveBeenCalledOnce();
    });
    expect(await screen.findByText("Sin configurar")).toBeInTheDocument();
    expect(screen.getByText("0 de 2 almacenes")).toBeInTheDocument();
  });

  it("does not close the panel on Escape while the withdrawal dialog is open", async () => {
    const user = userEvent.setup();
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: [{ brand: "firefox", trusted: true }],
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const onClose = vi.fn();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={onClose} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("button", { name: "Retirar…" }));
    await screen.findByRole("alertdialog");

    await user.keyboard("{Escape}");

    expect(onClose).not.toHaveBeenCalled();
  });
});
