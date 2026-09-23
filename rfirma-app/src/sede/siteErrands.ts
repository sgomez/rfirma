import type { Certificate } from "../signing/certificate";
import type { StageResult } from "../signing/flow";
import type { StoreSecret } from "../signing/secret";
import {
  documentInPlay,
  documentOf,
  errandOf,
  refusedBy,
  refusedByTheBatch,
} from "./errandConversion";
import type { Errand, ErrandStage, SiteDocument, SiteErrandPort, SiteOutcome } from "./errand";
import type {
  DescribedDocument,
  PortalResult,
  SecretResult,
  SiteErrandView,
  SiteStageView,
} from "./siteErrandView";

export type { DescribedDocument, SiteErrandView } from "./siteErrandView";

/**
 * **El `SiteErrandPort` de verdad**, el que sustituye a `noErrand()` (ID-335,
 * ID-336).
 *
 * No conoce a Tauri, y por eso está aquí y no en `tauri.ts`: recibe las órdenes
 * del backend ya envueltas en [`SiteCommands`] —una función por orden— y lo que
 * pone de su parte es la única cosa que hay que pensar, que es **la conversión
 * de lo que llega a lo que la ventana espera** (TD-78), delegada en
 * `errandConversion.ts`. El fichero que sabe que debajo hay Tauri sigue siendo
 * uno solo, y allí cada método es una línea.
 *
 * # Los momentos no vienen todos del backend
 *
 * El backend empuja sus momentos por el evento (`SiteStageView`) y la ventana
 * conoce alguno más (`ErrandStage`). Los que faltan —los dos tramos de la
 * firma— **son de este adaptador**, porque nacen y mueren dentro de una llamada
 * suya: `site_begin_signing` arranca la firma, `sign_with_pin` ejecuta la
 * firma y `site_finish_signing` entrega. El
 * backend no tiene nada que publicar entremedias, y sondearle por ello sería
 * inventar un ir y venir que no existe.
 */

/**
 * **Las órdenes del trámite, una función por orden.**
 *
 * Es la costura que hace probable a este adaptador sin Tauri (TD-78): las
 * pruebas enchufan dobles y comprueban la conversión y la suscripción, que es
 * lo único que aquí se decide.
 */
export interface SiteCommands {
  /**
   * Se suscribe al evento del trámite y devuelve cómo dejar de escuchar
   * (ID-338). Que no llegue nunca es la respuesta normal.
   */
  watch(onView: (view: SiteErrandView) => void): () => void;
  /**
   * `read_site_errand`: en qué momento está el trámite **ahora**, para la
   * ventana que acaba de montarse (ID-338).
   *
   * El evento sólo lo oye quien ya estaba escuchando, y el primer momento se
   * publica antes de que el frontal exista. Por eso ese se **pide**, igual que
   * la invocación con documento pide la suya, y los siguientes se escuchan.
   * `null` es la respuesta de una ventana que no es de sede.
   */
  readErrand(): Promise<SiteErrandView | null>;
  /** `site_identify`: la persona se identifica ante la sede. */
  identify(certificate: string): Promise<StageResult<void>>;
  /** `site_confirm_signatures`: fija la clave confirmada y vuelve a validar. */
  confirmSignatures(): Promise<StageResult<void>>;
  /** `site_decline`: la sede recibe `CANCEL` en el acto. */
  decline(): Promise<void>;
  /** `site_begin_signing`: prefirma, y dice cómo pedir el secreto. */
  beginSigning(certificate: string): Promise<StageResult<StoreSecret>>;
  /** `sign_with_pin`: la misma orden que el recorrido local (ADR-0001). */
  signWithPin(secret: string): Promise<SecretResult<void>>;
  /** `site_finish_signing`: postfirma, y la sede recibe la firma. */
  finishSigning(): Promise<StageResult<void>>;
  /**
   * `site_save_file`: abre el diálogo del portal y escribe donde la persona eligió. `true` si
   * hay que enseñar el desenlace de guardado; `false` si era el cierre de un `signandsave` que
   * ya se enseñó como firmado.
   */
  saveFile(): Promise<PortalResult<boolean>>;
  /**
   * `site_load_files`: abre el selector del portal y sigue con lo que la persona eligió.
   * Cuántos ficheros se han entregado a la sede, o `null` si el trámite sigue con un paso más
   * (`signandsave` con el documento ya elegido).
   */
  loadFiles(): Promise<PortalResult<number | null>>;
  /** `site_install_certificate`. `false` es que se cerró el diálogo sin elegir. */
  installCertificate(): Promise<boolean>;
  /** `site_look_again`: continúa el trámite, no lo reinicia. */
  lookAgain(): Promise<void>;
  /** `install_local_ca`: sin ella el navegador ni llega a preguntar. */
  installLocalCa(): Promise<void>;
  /** `close_site_window`. */
  closeWindow(): Promise<void>;
  /** `site_dismiss_the_warning`: al descartarlo se abre el canal que retenía. */
  dismissWarning(): Promise<void>;
  /** Lo que el PDF dice de sí mismo, o `null` si no se ha podido leer. */
  describeDocument(id: string): Promise<DescribedDocument | null>;
}

/**
 * El puerto de verdad, contra las órdenes del backend.
 *
 * Se construye **una sola vez**, fuera del árbol de React: `SedeWindow` se
 * resuscribe cuando el puerto cambia de identidad, y uno nuevo en cada pintada
 * lo haría en bucle.
 */
export function siteErrands(commands: SiteCommands): SiteErrandPort {
  let listener: ((errand: Errand | null) => void) | null = null;
  let errand: Errand | null = null;
  /** Con qué certificado y sobre qué se está firmando: un documento o un lote. */
  let signing: {
    certificate: Certificate;
    document: SiteDocument | null;
    signs: number | null;
  } | null = null;
  /**
   * Cuántos momentos han llegado. Leer el documento es asíncrono, así que uno
   * que llegue mientras se lee tiene que ganar: sin este contador, una lectura
   * lenta repintaría un consentimiento ya caducado.
   */
  let arrivals = 0;

  const publish = (next: Errand | null) => {
    errand = next;
    listener?.(next);
  };

  /** Cambia de momento sin tocar el origen ni la operación del trámite vivo. */
  const move = (stage: ErrandStage) => {
    if (errand !== null) publish({ ...errand, stage });
  };

  const finish = (outcome: SiteOutcome) => {
    signing = null;
    move({ kind: "outcome", outcome });
  };

  /**
   * El diálogo del portal, que es quien pregunta en los momentos de guardar y
   * de cargar.
   *
   * Sale **solo**, en cuanto el momento llega: la ventana no tiene ahí ningún
   * botón porque quien confirma es la persona dentro del diálogo, y la ruta
   * que elija no vuelve nunca hasta aquí (ADR-0011). Lo que sigue lo publica el
   * backend, salvo que la orden falle: eso es un desenlace.
   */
  const openPortal = async (stage: SiteStageView, arrival: number) => {
    if (stage.kind !== "saving" && stage.kind !== "loading") return;
    if (stage.kind === "saving") {
      const done = await commands.saveFile();
      if (arrival !== arrivals) return;
      if (!done.ok) {
        finish(refusedBy(done.failure));
        return;
      }
      // `false` es el cierre de un `signandsave`, ya enseñado como firmado:
      // aquí no hay nada más que enseñar.
      if (done.value) finish({ kind: "saved" });
      return;
    }
    const done = await commands.loadFiles();
    if (arrival !== arrivals) return;
    if (!done.ok) {
      finish(refusedBy(done.failure));
      return;
    }
    // `null` es que el trámite sigue: `signandsave` continúa con el
    // documento ya elegido, y el momento que sigue lo publica el backend.
    if (done.value !== null) finish({ kind: "loaded", fileCount: done.value });
  };

  const receive = async (view: SiteErrandView) => {
    const arrival = ++arrivals;
    // Un momento del backend manda sobre cualquier momento local: la sede ya
    // ha contestado, o el trámite ha cambiado de sitio.
    signing = null;
    if (view.stage.kind !== "askingToSign") {
      publish(errandOf(view));
      void openPortal(view.stage, arrival);
      return;
    }
    const described = await commands.describeDocument(view.stage.document);
    if (arrival !== arrivals) return;
    publish(
      errandOf(view, documentOf(described, view.stage.round, view.stage.unregisteredSignatures)),
    );
  };

  /** El tramo de la firma a la sede: firmar y entregar. */
  const sign = async (secret = "") => {
    const held = signing;
    if (held === null) return;

    const signed = await commands.signWithPin(secret);
    if (!signed.ok) {
      const batch = held.signs !== null;
      finish(batch ? refusedByTheBatch(signed.failure) : refusedBy(signed.failure));
      return;
    }

    // El lote no tiene postfirma que pedir desde aquí: la orden de firma lo
    // hace entero —prefirma, `PK1` y postfirma— y vuelve con la sede ya servida.
    if (held.signs !== null) {
      finish({ kind: "batchSigned", signs: held.signs });
      return;
    }

    move({ kind: "signing", certificate: held.certificate, phase: "returning" });
    const handed = await commands.finishSigning();
    finish(handed.ok ? { kind: "signed", document: held.document } : refusedBy(handed.failure));
  };

  return {
    watch(onChange) {
      listener = onChange;
      const stop = commands.watch((view) => void receive(view));

      // La escucha se pone primero y el momento guardado se pide después: así
      // ningún momento se cuela entre las dos cosas. Y si mientras se leía ha
      // llegado uno por el evento, ese manda —`arrivals` lo delata—, porque el
      // guardado es por definición el mismo o más viejo.
      void commands.readErrand().then((view) => {
        if (view !== null && arrivals === 0) void receive(view);
      });

      return () => {
        listener = null;
        stop();
      };
    },

    async consent(certificateId) {
      const stage = errand?.stage;
      if (errand === null || stage?.kind !== "consent") return;
      const certificate = stage.certificates.find((one) => one.id === certificateId);
      if (certificate === undefined) return;

      // El mismo contador que protege la lectura del documento, y por lo mismo:
      // consentir espera al backend, y un momento suyo que llegue mientras
      // tanto manda. Sin esto, el momento local que se publica al volver de la
      // orden pisaría el que el backend acaba de publicar.
      const arrival = arrivals;

      // `selectcert` no firma nada: la sede recibe la identidad y el trámite
      // termina ahí (ID-275). El tramo que se enseña es el de entregar, que es
      // el único que hay.
      if (errand.operation === "selectcert") {
        move({ kind: "signing", certificate, phase: "returning" });
        const identified = await commands.identify(certificateId);
        if (arrival !== arrivals) return;
        finish(identified.ok ? { kind: "signed", document: null } : refusedBy(identified.failure));
        return;
      }

      signing = { certificate, document: stage.document, signs: stage.signs };
      move({ kind: "signing", certificate, phase: "signing" });
      const begun = await commands.beginSigning(certificateId);
      if (arrival !== arrivals) return;
      if (!begun.ok) {
        finish(stage.signs !== null ? refusedByTheBatch(begun.failure) : refusedBy(begun.failure));
        return;
      }
      await sign("");
    },

    async confirmSignatures() {
      if (errand?.stage.kind !== "confirming") return;
      // El momento que sigue lo publica el backend, que vuelve a validar con la
      // clave ya fijada: aquí no se adelanta ninguno. Y como la pantalla no
      // cambia mientras tanto, el contador es lo único que separa una segunda
      // pulsación del rechazo local que mataría el trámite vivo.
      const arrival = arrivals;
      const confirmed = await commands.confirmSignatures();
      if (arrival !== arrivals) return;
      if (!confirmed.ok) finish(refusedBy(confirmed.failure));
    },

    async cancel() {
      const abandoned = documentInPlay(errand);
      const wasAnswering =
        errand !== null &&
        (errand.stage.kind === "consent" ||
          errand.stage.kind === "confirming" ||
          errand.stage.kind === "signing");
      signing = null;
      await commands.decline();
      // Decir que no a lo que se tenía delante **es un desenlace**, y se queda
      // en pantalla los quince segundos como los otros dos (ID-274). Irse desde
      // cualquier otro momento —la espera, el callejón, «sin certificado»— no
      // es un desenlace sino marcharse, y entonces la ventana se cierra.
      if (wasAnswering) {
        finish({ kind: "cancelled", document: abandoned });
        return;
      }
      await commands.closeWindow();
    },

    close: () => commands.closeWindow(),

    lookAgain: () => commands.lookAgain(),

    async installCertificate() {
      // Instalar y volver a mirar son el mismo gesto desde aquí: quien acaba de
      // meter un `.p12` quiere seguir el trámite, no pulsar un segundo botón.
      // Cerrar el diálogo sin elegir nada deja la pantalla como estaba.
      if (await commands.installCertificate()) await commands.lookAgain();
    },

    installLocalCa: () => commands.installLocalCa(),

    dismissWarning: () => commands.dismissWarning(),
  };
}
