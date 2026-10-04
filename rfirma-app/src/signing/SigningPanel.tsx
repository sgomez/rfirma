//! La columna derecha antes de firmar: la zona que se desliza con todo lo que se decide y el pie fijo con el botón de firmar.

import { useTranslation } from "react-i18next";
import { Switch } from "../design-system/Switch";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { PageMode, PageSet, PageSets, Placement } from "../placement/pageSets";
import { CertificateNotice } from "./CertificateNotice";
import { CertificateSelect } from "./CertificateSelect";
import type { Certificate } from "./certificate";
import type { Destination } from "./destination";
import type { SigningFailure } from "./failure";
import { ModelFieldset } from "./ModelFieldset";
import { PanelFooter } from "./PanelFooter";
import { PlacementFieldset } from "./PlacementFieldset";
import { PreviousSignaturesNotice } from "./PreviousSignaturesNotice";
import type { PreviousSignaturesReport } from "./previousSignatures";
import type { Rubric, RubricFailure } from "./rubric";
import "./SigningPanel.css";
import { usePlacementField } from "./usePlacementField";
import type { VisibleSignature } from "./visibleSignature";

export { formatSize } from "./panelFormat";

/** El documento que se va a firmar, con lo que el panel enseña de él. */
interface SigningDocument {
  /** Identifica el documento entre pestañas, para que un aviso no herede el estado del anterior. */
  id: string;
  name: string;
  pages: number;
  /** El tamaño, o `null` mientras nadie lo sepa: no se inventa un cero. */
  sizeBytes: number | null;
}

/**
 * En qué punto está la elección del certificado. Son los estados de la ficha;
 * «Listo» es `chosen` con un certificado en vigor.
 *
 * `failed` es el aterrizaje del rechazo: sin él la búsqueda que falla
 * no tenía dónde caer y la ficha se quedaba en `loading` para siempre. No es lo
 * mismo que `empty` —«no hay ninguno» y «no he podido buscarlos» son cosas
 * distintas— y por eso son dos estados y no un booleano dentro de uno.
 */
export type CertificateState =
  | { kind: "loading" }
  | { kind: "empty" }
  | { kind: "failed"; failure: NamedFailure }
  /**
   * Hay certificados y **ninguno elegido**, que es lo que pasa la primera vez
   * con varios: el selector dice «Elige un certificado» y el botón de firmar
   * sigue apagado. Elegir con qué identidad se firma un documento con validez
   * jurídica no lo hace la aplicación por su cuenta.
   */
  | { kind: "unchosen"; certificates: readonly Certificate[] }
  /**
   * Uno elegido, **y los demás al lado**: el desplegable los sigue listando,
   * porque cambiar de certificado es abrirlo otra vez y no un botón aparte.
   */
  | { kind: "chosen"; certificate: Certificate; certificates: readonly Certificate[] };

interface SigningPanelProps {
  document: SigningDocument;
  /** El informe de firmas previas del documento, pedido al abrir o cargar. */
  previousSignatures: PreviousSignaturesReport;
  certificate: CertificateState;
  /** Cuál se elige en el desplegable. */
  onChooseCertificate: (certificate: Certificate) => void;
  onRetryCertificates: () => void;
  onChooseModule: () => void;
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  /**
   * Dónde va la firma visible y en qué páginas, o `null` si aún no se ha
   * colocado. `null` es el PDF recién abierto y también haber quitado la última
   * página del conjunto: **colocado es tener páginas**.
   */
  placement: Placement | null;
  /**
   * El conjunto que guarda **cada opción**, que es lo que el bloque pinta
   * incluso cuando no manda: el pie de `Solo 1 página` dice su página aunque
   * esté activa `Todas`, y el campo trae el rango que se tecleó allí.
   */
  pageSets: PageSets;
  /**
   * El conjunto de la **opción activa** ha cambiado desde el bloque
   * «Colocación». El panel no compone `Placement`: no sabe dónde cae el
   * recuadro y no tiene por qué saberlo.
   */
  onChoosePages: (pages: PageSet | null) => void;
  /** Cuál de las tres modos manda sobre el conjunto. */
  pageMode: PageMode;
  onChangePageMode: (mode: PageMode) => void;
  /**
   * La página que se está mirando en el visor. Decide la cara del botón de
   * sellar: si la lleva, ofrece quitarla.
   */
  viewedPage: number;
  /**
   * Sellar la página que se está mirando, o quitarle el sello si ya lo lleva.
   *
   * El botón vive aquí, pero la acción la ejecuta el visor: es quien sabe
   * dónde cae el recuadro cuando no había ninguno todavía —su posición
   * estándar se mide sobre el `viewport` de `pdf.js`, que el panel no tiene—.
   */
  onSeal: () => void;
  onUnseal: () => void;
  rubric: Rubric | null;
  /** El último fallo al elegir la rúbrica, que se cuenta aquí y no al firmar. */
  rubricFailure: RubricFailure | null;
  onChooseRubric: () => void;
  destination: Destination;
  onChangeDestination: () => void;
  onSign: () => void;
  /** Mientras la firma corre, el botón no acepta un segundo empujón. */
  signing: boolean;
  failure: SigningFailure | null;
  /** Cierra el error y vuelve al panel, con el ciclo a medias olvidado en el backend. */
  onBack: () => void;
  onOpenHelp?: () => void;
  /** Vacía el Almacén de rFirma, ofrecido cuando `failure` es `keyringPinMissing` (ADR-0034). */
  onEmptyStore?: () => void;
}

/**
 * La columna derecha: **todo lo que hay que decidir antes de firmar**, y el
 * botón que firma (docs/design/panel-de-firma.md).
 *
 * Es la **única región de la aplicación con un botón primario**, y el botón va
 * al final: el panel entero se lee como una decisión que termina en una acción.
 *
 * Dos cosas que parecen detalles y son la ficha entera:
 *
 * - **No hay comodines.** El contenido del recuadro se marca con casillas y el
 *   texto lo compone Rust ya resuelto; el propio recuadro, en directo
 *   sobre la hoja, es lo que lo enseña.
 * - **La miniatura de la rúbrica es honesta.** Enseña el fichero ya
 *   normalizado, que es un JPEG y por tanto opaco: un PNG con transparencia se
 *   ve aquí con su fondo blanco, antes de firmar y no dentro del PDF.
 */
export function SigningPanel({
  document,
  previousSignatures,
  certificate,
  onChooseCertificate,
  onRetryCertificates,
  onChooseModule,
  signature,
  onChangeSignature,
  placement,
  pageSets,
  onChoosePages,
  pageMode,
  onChangePageMode,
  viewedPage,
  onSeal,
  onUnseal,
  rubric,
  rubricFailure,
  onChooseRubric,
  destination,
  onChangeDestination,
  onSign,
  signing,
  failure,
  onBack,
  onOpenHelp,
  onEmptyStore,
}: SigningPanelProps) {
  const { t } = useTranslation();
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;

  const { pagesText, rangeError, pageButton, typePages } = usePlacementField({
    documentPages: document.pages,
    pageSets,
    pageMode,
    placement,
    viewedPage,
    onChoosePages,
    onSeal,
    onUnseal,
  });

  const visible = signature.enabled && chosen !== null;
  const blocked = visible && rangeError !== null;

  return (
    <div className="panel">
      <div className="panel__scroll">
        {failure ? (
          // Error al firmar: la zona que se desliza se sustituye por la tarjeta
          // del fallo, como el resto del panel (docs/design/panel-de-firma.md §
          // Error al firmar). La cofirma, el certificado y la firma visible no
          // aportan nada mientras el documento sigue exactamente como estaba.
          <ErrorNotice
            situation={failure.situation}
            technicalDetail={failure.detail}
            onOpenHelp={onOpenHelp}
            onEmptyStore={onEmptyStore}
            documentUnchanged
          />
        ) : (
          <>
            {(certificate.kind === "loading" ||
              certificate.kind === "unchosen" ||
              certificate.kind === "chosen") && (
              <div className={signing ? "panel__controls--dim" : undefined}>
                <CertificateSelect
                  certificates={certificate.kind === "loading" ? [] : certificate.certificates}
                  chosen={chosen}
                  onChoose={onChooseCertificate}
                  searching={certificate.kind === "loading"}
                  disabled={signing}
                />
              </div>
            )}

            {previousSignatures.signatures.length > 0 && (
              <PreviousSignaturesNotice
                key={document.id}
                report={previousSignatures}
                certificate={chosen}
              />
            )}

            {(certificate.kind === "empty" || certificate.kind === "failed") && (
              <CertificateNotice state={certificate} onOpenHelp={onOpenHelp} />
            )}

            <section className="panel__visible" aria-label={t("panel.visibleSignature.title")}>
              <div className={signing ? "panel__toggle panel__toggle--dim" : "panel__toggle"}>
                <Switch
                  trailing
                  checked={visible}
                  disabled={chosen === null}
                  label={t("panel.visibleSignature.title")}
                  title={
                    visible
                      ? t("panel.visibleSignature.turnOff")
                      : t("panel.visibleSignature.turnOn")
                  }
                  onChange={(enabled) => onChangeSignature({ ...signature, enabled })}
                />
              </div>
              {(certificate.kind === "loading" || certificate.kind === "unchosen") && (
                <p className="rf-hint panel__visible-hint">
                  {t("panel.visibleSignature.needsCertificate")}
                </p>
              )}

              {visible && (
                <div
                  className={signing ? "panel__placement panel__controls--dim" : "panel__placement"}
                >
                  <PlacementFieldset
                    pageSets={pageSets}
                    pageMode={pageMode}
                    onChangePageMode={onChangePageMode}
                    pagesText={pagesText}
                    onTypePages={typePages}
                    rangeError={rangeError}
                    pageButton={pageButton}
                  />
                </div>
              )}
            </section>

            {visible && (
              <div
                className={
                  signing ? "panel__model-controls panel__controls--dim" : "panel__model-controls"
                }
              >
                <ModelFieldset
                  signature={signature}
                  onChangeSignature={onChangeSignature}
                  certificate={chosen}
                  rubric={rubric}
                  rubricFailure={rubricFailure}
                  onChooseRubric={onChooseRubric}
                  onOpenHelp={onOpenHelp}
                />
              </div>
            )}
          </>
        )}
      </div>

      <PanelFooter
        failure={failure}
        destination={destination}
        documentName={document.name}
        onChangeDestination={onChangeDestination}
        signing={signing}
        blocked={blocked}
        closed={previousSignatures.closed === true}
        certificate={certificate}
        onRetryCertificates={onRetryCertificates}
        onChooseModule={onChooseModule}
        onSign={onSign}
        onBack={onBack}
      />
    </div>
  );
}
