import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import { unavailableExternalDestinationOpener } from "./desktop/externalDestination";
import type { Drop, FakeDocumentDrops } from "./documents/drops";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryRecents } from "./documents/recents";
import type { Certificate } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { memoryStatus, type SignalRow } from "./status/status";
import { inMemoryVersionCheck, type VersionCheck } from "./updates/newVersion";
import { unavailablePdfSource } from "./viewer/source";

/**
 * La firma visible, con el visor y el panel a la vez: el panel nombra páginas
 * y el visor pone el rectángulo, y solo montados juntos acaban en el mismo
 * recuadro.
 */
describe("App · Firma visible, en qué páginas", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  async function openVisible() {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 8 }),
      {},
      { list: async () => [remembered] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByRole("button", { name: "Firmar como Ada Lovelace" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    await within(panel).findByText("En la página 1");
    return { user, panel };
  }

  const box = () => screen.queryByRole("application", { name: "Recuadro de la firma visible" });
  const field = (panel: HTMLElement) =>
    within(panel).getByRole("textbox", { name: "Páginas de la firma visible" });
  const nextPage = (user: ReturnType<typeof userEvent.setup>) =>
    user.click(screen.getByRole("button", { name: "Página siguiente" }));

  it("turns on already placed, on the page in view, and signing stays on", async () => {
    const { panel } = await openVisible();

    expect(box()).not.toBeNull();
    expect(within(panel).getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeEnabled();
    expect(within(panel).queryByText(/Coloca la firma/)).not.toBeInTheDocument();
  });

  it("moves the box under «one page» instead of adding a page to it", async () => {
    const { user, panel } = await openVisible();

    await nextPage(user);
    await user.click(within(panel).getByRole("button", { name: "Ponerla aquí" }));

    expect(await within(panel).findByText("En la página 2")).toBeInTheDocument();
    expect(within(panel).queryByRole("button", { name: "Ponerla aquí" })).not.toBeInTheDocument();
  });

  it("gives each option its own set, so going back brings what was left there", async () => {
    const { user, panel } = await openVisible();

    await user.click(within(panel).getByRole("radio", { name: "Varias" }));
    expect(field(panel)).toHaveValue("1");

    await user.clear(field(panel));
    await user.type(field(panel), "2,5");
    await user.click(within(panel).getByRole("radio", { name: "Una página" }));

    expect(within(panel).getByText("En la página 1")).toBeInTheDocument();

    await user.click(within(panel).getByRole("radio", { name: "Varias" }));

    expect(field(panel)).toHaveValue("2,5");
  });

  it("puts the page in view in the set under «several», and takes it off again", async () => {
    const { user, panel } = await openVisible();
    await user.click(within(panel).getByRole("radio", { name: "Varias" }));

    await nextPage(user);
    await user.click(within(panel).getByRole("button", { name: "Ponerla aquí" }));

    await waitFor(() => expect(field(panel)).toHaveValue("1-2"));

    await user.click(within(panel).getByRole("button", { name: "Quitarla de aquí" }));

    await waitFor(() => expect(field(panel)).toHaveValue("1"));
  });

  it("places the box on a document opened with the switch already on", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    await within(panel).findByText("En la página 1");

    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    expect(
      await within(screen.getByRole("region", { name: "Panel de firma" })).findByText(
        "En la página 1",
      ),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("application", { name: "Recuadro de la firma visible" }),
    ).toBeInTheDocument();
  });

  it("keeps the box and shows nothing below under «all»", async () => {
    const { user, panel } = await openVisible();

    await user.click(within(panel).getByRole("radio", { name: "Todas" }));

    expect(box()).not.toBeNull();
    expect(within(panel).queryByText(/En la página/)).not.toBeInTheDocument();
    expect(within(panel).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
  });
});

describe("App, sin un certificado elegido todavía", () => {
  function traceOverSheet() {
    const sheet = screen.getByRole("document", { name: "Hoja del documento" });
    fireEvent.pointerDown(sheet, { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
    fireEvent.pointerUp(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
  }

  it("turns the visible signature on and draws its box, empty, before any certificate", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));

    await openPdf(user);
    await screen.findByRole("document", { name: "Hoja del documento" });
    const panel = screen.getByRole("region", { name: "Panel de firma" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));

    expect(
      await screen.findByRole("application", { name: "Recuadro de la firma visible" }),
    ).toBeInTheDocument();
    expect(within(panel).getByText("En la página 1")).toBeInTheDocument();
    expect(screen.queryByText(/Elige un certificado para colocar/)).not.toBeInTheDocument();
  });

  it("lets the sheet be traced without a certificate", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));

    await openPdf(user);
    await screen.findByRole("document", { name: "Hoja del documento" });
    const panel = screen.getByRole("region", { name: "Panel de firma" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    const placed = await screen.findByRole("application", { name: "Recuadro de la firma visible" });
    const before = placed.getAttribute("style");

    traceOverSheet();

    expect(
      screen
        .getByRole("application", { name: "Recuadro de la firma visible" })
        .getAttribute("style"),
    ).not.toBe(before);
  });
});

/**
 * **La invocación desde fuera** (ID-157…ID-159): `rfirma documento.pdf`. Lo
 * que el doble entrega por `pending` es lo mismo que devuelve `read_invocation`
 * en Rust, y desemboca en la misma ventana que el arrastre — que es justo lo
 * que estas dos pruebas comprueban: **no hay una segunda interfaz**.
 */
describe("App, invocada con un documento", () => {
  it("opens the invoked PDF in the full window, just like a dropped one", async () => {
    renderApp(
      inMemoryRecents(),
      [],
      pdfsOf({ "contrato.pdf": 3 }),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      { document: document("contrato.pdf"), alsoEntering: [], failure: null, discarded: 0 },
    );

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("contrato.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^3 páginas/)).toBeInTheDocument();
  });

  /**
   * `pending()` es una lectura que **consume**, y el efecto que la pide se
   * rehace mientras la llamada está en vuelo: `<StrictMode>` lo hace en
   * desarrollo, y en producción lo hace la lectura asíncrona de los ajustes
   * cuando «Recordar mi actividad» viene apagado —cambia la identidad de
   * `accept`—. Si la entrega dependiera del ciclo de vida del efecto, la
   * respuesta llegaría a un efecto ya limpiado y el documento invocado
   * desaparecería sin ningún aviso.
   *
   * Por eso el doble no contesta solo: la prueba deja que el efecto se rehaga
   * con la lectura en vuelo y la contesta después.
   */
  it("delivers the invoked PDF when the effect remounts while the read is in flight", async () => {
    let answer: (invoked: Drop | null) => void = () => {};
    const base = inMemoryDocumentDrops();
    let asked = 0;
    const drops: FakeDocumentDrops = {
      ...base,
      pending: () => {
        asked += 1;
        return new Promise<Drop | null>((resolve) => {
          answer = resolve;
        });
      },
    };

    renderApp(
      inMemoryRecents(),
      [],
      pdfsOf({ "contrato.pdf": 3 }),
      { rememberActivity: false },
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      null,
      drops,
    );

    // Los ajustes ya han llegado, así que el efecto se ha rehecho: la lectura
    // de la invocación sigue viva y no se ha vuelto a pedir.
    await act(async () => {
      await Promise.resolve();
    });
    expect(asked).toBe(1);

    await act(async () => {
      answer({ document: document("contrato.pdf"), alsoEntering: [], failure: null, discarded: 0 });
    });

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("contrato.pdf")).toBeInTheDocument();
  });

  /** ID-158: no arranca ningún modo especial, abre la ventana y lo dice. */
  it("opens the normal window and says so when the argument is not a PDF", async () => {
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      {
        document: null,
        alsoEntering: [],
        failure: { situation: "notAPdf", detail: "el fichero no es un PDF" },
        discarded: 0,
      },
    );

    expect(await screen.findByRole("alert")).toHaveTextContent("Ese fichero no es un PDF");
    expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
  });

  /**
   * El aviso de versión nueva, que es el primer inquilino de la franja
   * (ID-181, ID-207). Se comprueba desde la aplicación entera porque lo que
   * decide el ticket es **dónde** notifica rFirma: bajo la cabecera y sin
   * modal.
   */
  describe("the new-version notice", () => {
    const withVersionCheck = (versions: VersionCheck) =>
      renderApp(
        inMemoryRecents(),
        [],
        unavailablePdfSource(),
        {},
        emptyCertificateStore(),
        emptyRubricPicker(),
        unavailableSigningBackend(),
        null,
        inMemoryDocumentDrops(),
        versions,
      );

    it("shows a strip under the header, and nothing modal, when there is a new version", async () => {
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      const strip = await screen.findByRole("status");
      expect(strip).toHaveTextContent("Hay una versión nueva de rFirma: 0.4.1");
      // Nada modal: ni diálogo encima ni ventana atenuada.
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
    });

    it("says nothing at all when there is no new version", async () => {
      withVersionCheck(inMemoryVersionCheck());

      await screen.findByRole("navigation", { name: "Documentos abiertos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    /**
     * «Avisarme cuando haya una versión nueva» apagado (ID-180): la
     * comprobación sigue corriendo, pero la franja no se monta.
     */
    it("says nothing when Avisarme cuando haya una versión nueva is turned off", async () => {
      renderApp(
        inMemoryRecents(),
        [],
        unavailablePdfSource(),
        { notifyNewVersion: false },
        emptyCertificateStore(),
        emptyRubricPicker(),
        unavailableSigningBackend(),
        null,
        inMemoryDocumentDrops(),
        inMemoryVersionCheck({ version: "0.4.1" }),
      );

      await screen.findByRole("navigation", { name: "Documentos abiertos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    // Sin red la comprobación ni contesta ni se queja: la ventana se queda
    // como estaba, que es lo que dice el ID-178.
    it("says nothing when the check fails", async () => {
      withVersionCheck({ latest: async () => Promise.reject(new Error("sin red")) });

      await screen.findByRole("navigation", { name: "Documentos abiertos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    it("takes the user to About, which is where the upgrade instructions are", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      await screen.findByRole("status");
      await user.click(screen.getByRole("button", { name: "Cómo actualizar" }));

      expect(await screen.findByRole("dialog")).toHaveTextContent("rFirma");
    });

    it("is dismissed for good once dismissed", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      await screen.findByRole("status");
      await user.click(screen.getByRole("button", { name: "Descartar" }));

      await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
      // La ventana sigue entera debajo: descartar no navega a ninguna parte.
      expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
    });
  });
});

// ID-353/ID-347: el triángulo del menú se mide al arrancar y con cada
// remedición del panel, con su propia regla de disparo.
describe("App, el triángulo de aviso del menú", () => {
  function rowsWithSitesUnconfigured(): SignalRow[] {
    return [
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
  }

  function rowsWithSitesOpeningAutoFirma(): SignalRow[] {
    return [
      {
        signal: "siteSignature",
        value: "AutoFirma",
        verdict: "attention",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
  }

  it("lights up at startup when Firma en sedes is Sin configurar", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      null,
      inMemoryDocumentDrops(null),
      inMemoryVersionCheck(),
      unavailableExternalDestinationOpener(),
      memoryStatus(rowsWithSitesUnconfigured()),
    );

    await user.click(await screen.findByRole("button", { name: "Menú" }));

    expect(screen.getByRole("img", { name: "Requiere atención" })).toBeInTheDocument();
  });

  it("stays off when the sites open AutoFirma, the trap a naive implementation breaks", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      null,
      inMemoryDocumentDrops(null),
      inMemoryVersionCheck(),
      unavailableExternalDestinationOpener(),
      memoryStatus(rowsWithSitesOpeningAutoFirma()),
    );

    await user.click(await screen.findByRole("button", { name: "Menú" }));

    expect(screen.queryByRole("img", { name: "Requiere atención" })).not.toBeInTheDocument();
  });
});
