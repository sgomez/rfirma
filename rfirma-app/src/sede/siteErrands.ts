import type { Catalog } from "../i18n/catalog";
import type { Certificate } from "../signing/certificate";
import type { StageResult } from "../signing/flow";
import type { StoreSecret } from "../signing/secret";
import { belongsToPinDialog, type TokenFailure } from "../signing/token";
import type {
  Errand,
  ErrandStage,
  RefusalSituation,
  SiteDocument,
  SiteErrandPort,
  SiteOutcome,
} from "./errand";

/**
 * **El `SiteErrandPort` de verdad**, el que sustituye a `noErrand()` (ID-335,
 * ID-336).
 *
 * No conoce a Tauri, y por eso está aquí y no en `tauri.ts`: recibe las órdenes
 * del backend ya envueltas en [`SiteCommands`] —una función por orden— y lo que
 * pone de su parte es la única cosa que hay que pensar, que es **la conversión
 * de lo que llega a lo que la ventana espera** (TD-78). El fichero que sabe que
 * debajo hay Tauri sigue siendo uno solo, y allí cada método es una línea.
 *
 * # Los momentos no vienen todos del backend
 *
 * El backend empuja seis momentos por el evento (`SiteStageView`) y la ventana
 * conoce siete (`ErrandStage`). Los dos que faltan —el secreto del almacén y
 * los dos tramos de la firma— **son de este adaptador**, porque nacen y mueren
 * dentro de una llamada suya: `site_begin_signing` contesta cómo hay que pedir
 * el secreto, `sign_with_pin` lo consume y `site_finish_signing` entrega. El
 * backend no tiene nada que publicar entremedias, y sondearle por ello sería
 * inventar un ir y venir que no existe.
 */

/**
 * **El trámite tal como lo emite el backend**: `commands::SiteErrandView`,
 * campo a campo.
 *
 * Detrás no hay ninguna ruta (ADR-0011): el documento que manda la sede viaja
 * por su **asa opaca** y el origen viaja **a secas**, sólo para atribuir
 * (ID-271, ID-339).
 */
export interface SiteErrandView {
  origin: string | null;
  stage: SiteStageView;
}

/** El momento de la secuencia, tal como lo emite el backend. */
export type SiteStageView =
  | { kind: "waiting" }
  | { kind: "askingForConsent"; certificates: readonly Certificate[] }
  | {
      kind: "askingToSign";
      /** El asa opaca con la que se lee el documento, nunca su ruta (ID-286). */
      document: string;
      round: "sign" | "cosign";
      certificates: readonly Certificate[];
      unregisteredSignatures: boolean;
    }
  | { kind: "noChannel"; reason: "channelNotOpened" | "localCaMissing" }
  | { kind: "outcome"; outcome: { kind: "refused"; situation: string; detail: string } }
  | { kind: "noCertificate"; reason: "none" | "excluded"; owned: number };

/**
 * Lo que el PDF de la sede dice de sí mismo, leído por su asa.
 *
 * Es lo único que se puede enseñar del documento: la petición **no trae
 * nombre** (ID-270) y de la ruta del fichero de paso no llega nada. Sale de
 * abrir los bytes que devuelve `read_document`, así que `null` es que no se han
 * podido leer, y entonces no hay tarjeta que pintar.
 */
export interface DescribedDocument {
  title: string | null;
  pages: number;
  sizeBytes: number;
}

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
  /** `site_identify`: la persona se identifica ante la sede. */
  identify(certificate: string): Promise<StageResult<void>>;
  /** `site_decline`: la sede recibe `CANCEL` en el acto. */
  decline(): Promise<void>;
  /** `site_begin_signing`: prefirma, y dice cómo pedir el secreto. */
  beginSigning(certificate: string): Promise<StageResult<StoreSecret>>;
  /** `sign_with_pin`: la misma orden que el recorrido local (ADR-0001). */
  signWithPin(secret: string): Promise<StageResult<void>>;
  /** `site_finish_signing`: postfirma, y la sede recibe la firma. */
  finishSigning(): Promise<StageResult<void>>;
  /** `site_install_certificate`. `false` es que se cerró el diálogo sin elegir. */
  installCertificate(): Promise<boolean>;
  /** `site_look_again`: continúa el trámite, no lo reinicia. */
  lookAgain(): Promise<void>;
  /** `install_local_ca`: sin ella el navegador ni llega a preguntar. */
  installLocalCa(): Promise<void>;
  /** `close_site_window`. */
  closeWindow(): Promise<void>;
  /** Lo que el PDF dice de sí mismo, o `null` si no se ha podido leer. */
  describeDocument(id: string): Promise<DescribedDocument | null>;
}

/**
 * Las situaciones de rechazo que el catálogo sabe redactar.
 *
 * Un `Record` y no una lista: si `sede.refusals` gana una clave, `tsc` exige
 * que entre también aquí, y ninguna situación nueva acaba cayendo en `unknown`
 * sin que nadie se entere.
 */
const REFUSALS: Record<keyof Catalog["sede"]["refusals"], true> = {
  appendedSignaturePage: true,
  unsupportedFilter: true,
  unsupportedProtocolVersion: true,
  missingFormat: true,
  errandInFlight: true,
  unknown: true,
};

/** La situación tal como la sabe nombrar el catálogo, o `unknown`. */
function refusalOf(situation: string): RefusalSituation {
  return situation in REFUSALS ? (situation as RefusalSituation) : "unknown";
}

/** Un fallo de una etapa, contado como el desenlace que la ventana enseña. */
function refusedBy(failure: TokenFailure): SiteOutcome {
  return { kind: "refused", situation: refusalOf(failure.situation), detail: failure.detail };
}

/**
 * **El momento del backend, en el vocabulario de la ventana** (TD-78).
 *
 * La operación no viaja en el evento porque está en el momento: la sede que
 * sólo pide identidad manda `askingForConsent`, y la que manda un documento
 * manda `askingToSign`. En la espera todavía no se sabe cuál de las dos es, y
 * ahí `operation` no la mira nadie —`consentActionKey` sólo se consulta al
 * consentir—.
 *
 * `document` llega aparte porque leerlo por su asa es una ida y vuelta al
 * backend, y esta función es pura.
 */
export function errandOf(view: SiteErrandView, document: SiteDocument | null = null): Errand {
  const stage = view.stage;
  const operation = stage.kind === "askingForConsent" ? "selectcert" : "sign";
  return { origin: view.origin, operation, stage: stageOf(stage, document) };
}

function stageOf(stage: SiteStageView, document: SiteDocument | null): ErrandStage {
  switch (stage.kind) {
    case "waiting":
      return { kind: "waiting" };
    case "noChannel":
      return { kind: "noChannel", reason: stage.reason };
    case "noCertificate":
      return { kind: "noCertificate", reason: stage.reason, owned: stage.owned };
    case "outcome":
      return {
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: refusalOf(stage.outcome.situation),
          detail: stage.outcome.detail,
        },
      };
    case "askingForConsent":
      // Sin documento porque no lo hay: `selectcert` no manda ninguno. Y
      // `narrowed` es `false` porque el backend no dice si la sede acotó la
      // lista: lo que cruza son las filas ya cribadas y nunca el criterio
      // (ID-277).
      return { kind: "consent", document: null, certificates: stage.certificates, narrowed: false };
    case "askingToSign":
      return { kind: "consent", document, certificates: stage.certificates, narrowed: false };
  }
}

/**
 * El documento del consentimiento, con lo que dice de sí mismo y lo que dijo el
 * backend de sus firmas.
 *
 * `signatures` sale de `round` y no de un recuento: la sede pide `cosign`
 * cuando el PDF ya viene firmado, y cuántas firmas trae exactamente no lo
 * cuenta nadie —tampoco el recorrido local, que pasa `signatures: null`—. Lo
 * que la ficha pide enseñar es **el aviso de cofirma**, y eso es lo que hay.
 */
function documentOf(
  described: DescribedDocument | null,
  round: "sign" | "cosign",
  unregisteredSignatures: boolean,
): SiteDocument | null {
  if (described === null) return null;
  return {
    ...described,
    signatures: round === "cosign" ? 1 : 0,
    hasUnregisteredSignatures: unregisteredSignatures,
  };
}

/** Qué documento se estaba consintiendo, para poder nombrarlo en el desenlace. */
function documentInPlay(errand: Errand | null): SiteDocument | null {
  const stage = errand?.stage;
  return stage?.kind === "consent" ? stage.document : null;
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
  /** Con qué certificado y sobre qué documento se está firmando. */
  let signing: { certificate: Certificate; document: SiteDocument | null } | null = null;
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

  const receive = async (view: SiteErrandView) => {
    const arrival = ++arrivals;
    // Un momento del backend manda sobre cualquier momento local: la sede ya
    // ha contestado, o el trámite ha cambiado de sitio.
    signing = null;
    if (view.stage.kind !== "askingToSign") {
      publish(errandOf(view));
      return;
    }
    const described = await commands.describeDocument(view.stage.document);
    if (arrival !== arrivals) return;
    publish(
      errandOf(view, documentOf(described, view.stage.round, view.stage.unregisteredSignatures)),
    );
  };

  /** El tramo que va del secreto a la sede: firmar y entregar. */
  const sign = async (secret: string) => {
    const held = signing;
    if (held === null) return;

    move({ kind: "signing", certificate: held.certificate, phase: "signing" });
    const signed = await commands.signWithPin(secret);
    if (!signed.ok) {
      // Un PIN incorrecto se reintenta dentro del diálogo, sin reiniciar nada;
      // lo demás sale del diálogo, y aquí salir es el desenlace.
      if (belongsToPinDialog(signed.failure)) {
        move({ kind: "secret", certificate: held.certificate, failure: signed.failure });
        return;
      }
      finish(refusedBy(signed.failure));
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

      signing = { certificate, document: stage.document };
      move({ kind: "signing", certificate, phase: "signing" });
      const begun = await commands.beginSigning(certificateId);
      if (arrival !== arrivals) return;
      if (!begun.ok) {
        finish(refusedBy(begun.failure));
        return;
      }
      // Sin sesión no hay diálogo y no se inventa ningún PIN: se manda la
      // cadena vacía, igual que en el recorrido local.
      if (begun.value.kind === "notNeeded") {
        await sign("");
        return;
      }
      move({ kind: "secret", certificate, failure: null });
    },

    submitSecret: (secret) => sign(secret),

    async cancel() {
      const abandoned = documentInPlay(errand);
      const wasAnswering =
        errand !== null &&
        (errand.stage.kind === "consent" ||
          errand.stage.kind === "secret" ||
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
  };
}
