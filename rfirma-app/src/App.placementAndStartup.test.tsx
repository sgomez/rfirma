import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { aCertificate, document, pdfsOf, renderApp, trayDropZone } from "./App.testSupport";
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
 * El bloque «Colocación», con el visor y el panel a la vez (#185, #188).
 *
 * Vive en la grada A y no en el panel porque los tres caminos que colocan
 * —arrastre, pastilla y campo— acaban en el mismo recuadro **solo si los dos
 * componentes están montados**: el panel nombra páginas y no sabe dónde cae el
 * rectángulo, y el visor pone el rectángulo sin saber cuál de las tres opciones
 * manda. Probado por separado, cada uno pasaba en verde con el fallo dentro.
 */
describe("App · Colocación", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  /** Abre el documento y espera al bloque «Colocación» ya pintado. */
  async function openPlacing() {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 8 }),
      {},
      { list: async () => [remembered] },
    );
    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByText("Colocación");
    return { user, panel };
  }

  const box = () => screen.queryByRole("application", { name: "Recuadro de la firma visible" });
  const pill = () => screen.getByRole("button", { name: "Sellar esta página" });

  it("places the box on its standard spot when a range is typed, with nothing placed yet", async () => {
    const { user, panel } = await openPlacing();

    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));
    await user.type(within(panel).getByLabelText("Páginas donde se sella"), "1");

    // El recuadro cae abajo a la derecha sin que nadie lo haya arrastrado: es
    // el ID-102 pedido desde el panel, que es lo que el #185 no hacía.
    expect(box()).not.toBeNull();
  });

  it("places the box when «all the pages» is chosen, with nothing placed yet", async () => {
    const { user, panel } = await openPlacing();

    await user.click(within(panel).getByRole("radio", { name: /Todas las páginas/ }));

    expect(box()).not.toBeNull();
  });

  it("keeps the pill saying the same thing on every option while nothing is placed", async () => {
    const { user, panel } = await openPlacing();

    expect(pill()).toBeInTheDocument();
    // Con el campo vacío «Estas páginas» sigue sin nombrar ninguna página, que
    // es la única opción con la que se puede comparar la pastilla (#188).
    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));

    expect(pill()).toBeInTheDocument();
  });

  it("replaces the sealed page instead of adding to it under «one page only»", async () => {
    const { user, panel } = await openPlacing();

    await user.click(pill());
    await user.click(screen.getByRole("button", { name: "Página siguiente" }));
    await user.click(pill());

    // Ni el aviso del recuadro repetido —que solo aparece con más de una— ni un
    // conjunto de dos: esa opción nombra una página y nada más.
    expect(within(panel).queryByText(/El mismo recuadro/)).toBeNull();
    expect(within(panel).getByText("Página 2")).toBeInTheDocument();
  });

  it("gives each option its own set, so going back brings what was left there", async () => {
    const { user, panel } = await openPlacing();
    const field = () => within(panel).getByLabelText("Páginas donde se sella");

    await user.click(pill());
    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));
    // Se estrena sembrada de la anterior, que es lo que pide la ficha.
    expect(field()).toHaveValue("1");

    await user.clear(field());
    await user.type(field(), "2,5");
    await user.click(within(panel).getByRole("radio", { name: /Solo 1 página/ }));

    // La suya, la 1, y no la más baja del conjunto de al lado (#188).
    expect(within(panel).getByText("Página 1")).toBeInTheDocument();

    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));

    expect(field()).toHaveValue("2,5");
  });
});

/**
 * ID-108: sin certificado utilizable no hay sello que dibujar, y sin sello no
 * hay recuadro. El panel lo cumplía desde siempre —apaga su bloque entero y dice
 * «Elige un certificado para colocar la firma visible»—, pero el visor tenía su
 * propia copia del estado y no lo miraba: ofrecía sellar y dejaba trazar (#190).
 */
describe("App, sin un certificado elegido todavía", () => {
  /** Pulsar, mover y soltar sobre la hoja: el gesto que coloca el recuadro. */
  function traceOverSheet() {
    const sheet = screen.getByRole("document", { name: "Hoja del documento" });
    fireEvent.pointerDown(sheet, { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
    fireEvent.pointerUp(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
  }

  it("neither offers to seal the page nor lets the sheet be traced", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));

    await user.click(trayDropZone());
    await screen.findByRole("document", { name: "Hoja del documento" });

    expect(screen.getByText("Elige un certificado para colocar la firma visible")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Sellar esta página" })).not.toBeInTheDocument();

    traceOverSheet();

    expect(screen.queryByRole("application")).not.toBeInTheDocument();
  });

  it("lets the sheet be traced as soon as a certificate is chosen", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 3 }),
      {},
      { list: async () => [aCertificate] },
    );

    await user.click(trayDropZone());
    await screen.findByRole("document", { name: "Hoja del documento" });
    const panel = screen.getByRole("region", { name: "Panel de firma" });
    await user.click(await within(panel).findByRole("combobox", { name: "Certificado" }));
    // La lista vive en un portal, fuera de `panel` (ID-308).
    await user.click(screen.getAllByRole("option")[0] as HTMLElement);

    traceOverSheet();

    expect(
      screen.getByRole("application", { name: "Recuadro de la firma visible" }),
    ).toBeInTheDocument();
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
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
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
      expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    });

    it("says nothing at all when there is no new version", async () => {
      withVersionCheck(inMemoryVersionCheck());

      await screen.findByRole("region", { name: "Bandeja de documentos" });
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

      await screen.findByRole("region", { name: "Bandeja de documentos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    // Sin red la comprobación ni contesta ni se queja: la ventana se queda
    // como estaba, que es lo que dice el ID-178.
    it("says nothing when the check fails", async () => {
      withVersionCheck({ latest: async () => Promise.reject(new Error("sin red")) });

      await screen.findByRole("region", { name: "Bandeja de documentos" });
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
      expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
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
