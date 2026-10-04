import { act, screen, waitFor, within } from "@testing-library/react";
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
    await within(panel).findByRole("button", { name: "Firmar" });
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
    expect(within(panel).getByRole("button", { name: "Firmar" })).toBeEnabled();
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
      {},
      { list: async () => [remembered] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByRole("button", { name: "Firmar" });
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

  it("places the box on page 1 of a document opened while another was on page 3", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 5, "segundo.pdf": 5 }),
      {},
      { list: async () => [remembered] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByRole("button", { name: "Firmar" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    await within(panel).findByText("En la página 1");
    await nextPage(user);
    await nextPage(user);
    await within(panel).findByRole("button", { name: "Ponerla aquí" });

    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });
    await screen.findByRole("application", { name: "Recuadro de la firma visible" });

    const current = screen.getByRole("region", { name: "Panel de firma" });
    await waitFor(() => expect(within(current).getByText("En la página 1")).toBeInTheDocument());
    expect(within(current).queryByRole("button", { name: "Ponerla aquí" })).not.toBeInTheDocument();
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
  it("draws no box and keeps the visible-signature switch disabled", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));

    await openPdf(user);
    await screen.findByRole("document", { name: "Hoja del documento" });
    const panel = screen.getByRole("region", { name: "Panel de firma" });

    expect(within(panel).getByRole("switch", { name: "Firma visible" })).toBeDisabled();
    expect(
      screen.queryByRole("application", { name: "Recuadro de la firma visible" }),
    ).not.toBeInTheDocument();
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

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "contrato.pdf", selected: true })).toBeInTheDocument();
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

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "contrato.pdf", selected: true })).toBeInTheDocument();
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
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1", installable: false }));

      const strip = await screen.findByRole("status");
      expect(strip).toHaveTextContent("Hay una versión nueva: 0.4.1");
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
     * «Avisar de versiones nuevas» apagado (ID-180): la
     * comprobación sigue corriendo, pero la franja no se monta.
     */
    it("says nothing when Avisar de versiones nuevas is turned off", async () => {
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
        inMemoryVersionCheck({ version: "0.4.1", installable: false }),
      );

      await screen.findByRole("navigation", { name: "Documentos abiertos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    // Sin red la comprobación ni contesta ni se queja: la ventana se queda
    // como estaba, que es lo que dice el ID-178.
    it("says nothing when the check fails", async () => {
      withVersionCheck({
        latest: async () => Promise.reject(new Error("sin red")),
        install: async () => "networkFailure",
      });

      await screen.findByRole("navigation", { name: "Documentos abiertos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    it("takes the user to About, which is where the upgrade instructions are", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1", installable: false }));

      await screen.findByRole("status");
      await user.click(screen.getByRole("button", { name: "Cómo actualizar" }));

      expect(await screen.findByRole("dialog")).toHaveTextContent("rFirma");
    });

    /**
     * *Acerca de* pregunta por el puerto cada vez que se abre, no solo al
     * arrancar: si la respuesta cambia mientras tanto, la tarjeta la
     * sustituye. La franja del arranque, mientras tanto, sigue mostrando lo
     * conocido entonces.
     */
    it("asks again when About opens, and shows the fresh answer there", async () => {
      const user = userEvent.setup();
      let askedTimes = 0;
      const versions: VersionCheck = {
        latest: async () => {
          askedTimes += 1;
          return askedTimes === 1
            ? { version: "0.4.1", installable: false }
            : { version: "0.5.0", installable: false };
        },
        install: async () => "notAvailable",
      };
      withVersionCheck(versions);

      const strip = await screen.findByRole("status");
      expect(strip).toHaveTextContent("Hay una versión nueva: 0.4.1");

      await user.click(screen.getByRole("button", { name: "Cómo actualizar" }));

      expect(await screen.findByText("Hay una versión nueva: 0.5.0")).toBeInTheDocument();
      // La franja del arranque no se toca: sigue con lo que supo entonces.
      expect(strip).toHaveTextContent("Hay una versión nueva: 0.4.1");
    });

    describe("when the new version is installable", () => {
      const installable = { version: "0.4.1", installable: true };

      it("offers Actualizar ahora instead of Cómo actualizar", async () => {
        withVersionCheck(inMemoryVersionCheck(installable));

        await screen.findByRole("status");
        expect(screen.getByRole("button", { name: "Actualizar ahora" })).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: "Cómo actualizar" })).not.toBeInTheDocument();
      });

      it("asks for confirmation with the version, and installs only once confirmed", async () => {
        const user = userEvent.setup();
        const versions = inMemoryVersionCheck(installable);
        withVersionCheck(versions);

        await screen.findByRole("status");
        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));

        expect(await screen.findByRole("dialog")).toHaveTextContent(
          "¿Actualizar a la versión 0.4.1?",
        );
        expect(versions.installCalls).toBe(0);
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));
        await waitFor(() => expect(versions.installCalls).toBe(1));
      });

      it("can be postponed without installing", async () => {
        const user = userEvent.setup();
        const versions = inMemoryVersionCheck(installable);
        withVersionCheck(versions);

        await screen.findByRole("status");
        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(await screen.findByRole("button", { name: "Ahora no" }));

        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        expect(versions.installCalls).toBe(0);
      });

      it.each([
        ["noUpdate", "Ya no hay versión nueva."],
        ["networkFailure", "Comprueba tu conexión e inténtalo de nuevo."],
        ["invalidSignature", "no tiene una firma válida"],
        ["notAvailable", "no se actualiza desde rFirma"],
      ] as const)("explains the %s failure and stays open", async (installation, message) => {
        const user = userEvent.setup();
        withVersionCheck(inMemoryVersionCheck(installable, installation));

        await screen.findByRole("status");
        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(await screen.findByRole("button", { name: "Instalar y cerrar" }));

        expect(await screen.findByRole("alert")).toHaveTextContent(message);
        await user.click(screen.getByRole("button", { name: "Cerrar" }));
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });

      it("offers nothing in the strip when notify is off, but About still shows the status", async () => {
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
          inMemoryVersionCheck(installable),
        );

        await screen.findByRole("navigation", { name: "Documentos abiertos" });
        expect(screen.queryByRole("button", { name: "Actualizar ahora" })).not.toBeInTheDocument();
      });
    });

    it("is dismissed for good once dismissed", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1", installable: false }));

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

    await user.click(
      await screen.findByRole("button", { name: "Estado de rFirma: requiere atención" }),
    );

    expect(await screen.findByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Estado de rFirma: requiere atención" }),
    ).not.toBeInTheDocument();
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

    expect(
      screen.queryByRole("button", { name: "Estado de rFirma: requiere atención" }),
    ).not.toBeInTheDocument();
  });
});
