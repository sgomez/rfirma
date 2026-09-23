import { useTranslation } from "react-i18next";
import { FileIcon, InfoIcon } from "../design-system/icons";
import type { NamedFailure } from "../errors/classify";
import { Switch } from "../preferences/Switch";
import type { PageChoice, PageSet, PageSets, Placement } from "../viewer/signatureBox";
import { CertificateBlock } from "./CertificateBlock";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";
import type { Destination } from "./destination";
import type { SigningFailure } from "./failure";
import { PanelFooter } from "./PanelFooter";
import { PlacementFieldset } from "./PlacementFieldset";
import { formatSize } from "./panelFormat";
import type { Rubric, RubricFailure } from "./rubric";
import { SignatureFieldsFieldset } from "./SignatureFieldsFieldset";
import "./SigningPanel.css";
import { usePlacementField } from "./usePlacementField";
import type { VisibleSignature } from "./visibleSignature";

export { formatSize } from "./panelFormat";

/** El documento que se va a firmar, con lo que el panel enseña de él. */
interface SigningDocument {
  name: string;
  pages: number;
  /** El tamaño, o `null` mientras nadie lo sepa: no se inventa un cero. */
  sizeBytes: number | null;
  /**
   * Cuántas firmas trae ya. Cualquier número mayor que cero es una cofirma, y
   * `null` es **no se sabe todavía**: la insignia de la bandeja dice si el PDF
   * está firmado, pero no cuántas veces, y el aviso de cofirma necesita el
   * número. Callar es mejor que decir «1 firma» a ojo.
   */
  signatures: number | null;
}

/**
 * En qué punto está la elección del certificado. Son los estados de la ficha;
 * «Listo» es `chosen` con un certificado en vigor.
 *
 * `failed` es el aterrizaje del rechazo (ID-10): sin él la búsqueda que falla
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
   * con varios: el disparador dice «Elegir certificado» y el botón de firmar
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
   * página del conjunto: **colocado es tener páginas** (ID-92).
   */
  placement: Placement | null;
  /**
   * El conjunto que guarda **cada opción**, que es lo que el bloque pinta
   * incluso cuando no manda: el pie de `Solo 1 página` dice su página aunque
   * esté activa `Todas`, y el campo trae el rango que se tecleó allí (#188).
   */
  pageSets: PageSets;
  /**
   * El conjunto de la **opción activa** ha cambiado desde el bloque
   * «Colocación». El panel no compone `Placement`: no sabe dónde cae el
   * recuadro y no tiene por qué saberlo (#185).
   */
  onChoosePages: (pages: PageSet | null) => void;
  /** Cuál de las tres opciones manda sobre el conjunto (ID-97). */
  pageChoice: PageChoice;
  onChangePageChoice: (choice: PageChoice) => void;
  /**
   * La página que se está mirando en el visor. Decide la cara del botón de
   * sellar: si la lleva, ofrece quitarla (#194).
   */
  viewedPage: number;
  /**
   * Sellar la página que se está mirando, o quitarle el sello si ya lo lleva.
   *
   * El botón vive aquí, pero la acción la ejecuta el visor: es quien sabe
   * dónde cae el recuadro cuando no había ninguno todavía —su posición
   * estándar se mide sobre el `viewport` de `pdf.js`, que el panel no tiene
   * (#194)—.
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
  onOpenHelp?: () => void;
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
 *   texto lo compone Rust ya resuelto (ID-19); el propio recuadro, en directo
 *   sobre la hoja, es lo que lo enseña.
 * - **La miniatura de la rúbrica es honesta.** Enseña el fichero ya
 *   normalizado, que es un JPEG y por tanto opaco: un PNG con transparencia se
 *   ve aquí con su fondo blanco, antes de firmar y no dentro del PDF (ID-24).
 */
export function SigningPanel({
  document,
  certificate,
  onChooseCertificate,
  onRetryCertificates,
  onChooseModule,
  signature,
  onChangeSignature,
  placement,
  pageSets,
  onChoosePages,
  pageChoice,
  onChangePageChoice,
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
  onOpenHelp,
}: SigningPanelProps) {
  const { t, i18n } = useTranslation();
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
  const usable = chosen !== null && isUsable(chosen.status);

  const { pagesText, rangeError, echo, sealedCount, sealButton, typePages } = usePlacementField({
    documentPages: document.pages,
    pageSets,
    pageChoice,
    placement,
    viewedPage,
    onChoosePages,
    onSeal,
    onUnseal,
  });

  // Con el interruptor encendido y sin colocar **no se firma**, y el pie manda
  // hacer la acción en vez de describir el estado (ID-93). Con el interruptor
  // apagado se firma, invisible, como siempre.
  const unplaced = signature.enabled && placement === null;
  const blocked = signature.enabled && (placement === null || rangeError !== null);

  return (
    <div className="panel">
      <div className="panel__scroll">
        <div className="panel__header">
          <span className="panel__header-icon">
            <FileIcon />
          </span>
          <div className="panel__header-text">
            <p className="rf-title panel__document">{document.name}</p>
            <p className="rf-body rf-text-muted">
              {[
                t("panel.document.pages", { count: document.pages }),
                document.sizeBytes === null ? null : formatSize(document.sizeBytes, i18n.language),
              ]
                .filter((piece) => piece !== null)
                .join(" · ")}
            </p>
          </div>
        </div>

        {document.signatures !== null && document.signatures > 0 && (
          <div className="panel__co-signature">
            <span className="panel__notice-icon">
              <InfoIcon />
            </span>
            <p className="rf-prose">{t("panel.coSignature", { count: document.signatures })}</p>
          </div>
        )}

        <hr className="rf-divider" />

        <section className="panel__section" aria-label={t("panel.certificate.title")}>
          <p className="rf-label panel__heading">{t("panel.certificate.title")}</p>
          <CertificateBlock
            state={certificate}
            onChoose={onChooseCertificate}
            onRetry={onRetryCertificates}
            onChooseModule={onChooseModule}
            onOpenHelp={onOpenHelp}
          />
        </section>

        <hr className="rf-divider" />

        <section
          className={usable ? "panel__section" : "panel__section panel__section--inert"}
          aria-label={t("panel.visibleSignature.title")}
          inert={!usable}
        >
          <p className="rf-label panel__heading">{t("panel.visibleSignature.title")}</p>
          {/* ID-108: sin certificado no hay sello que dibujar, y sin sello no
              hay recuadro. El aviso va encima del interruptor porque es lo que
              explica por qué el bloque entero está en gris. */}
          {!usable && <p className="rf-hint">{t("panel.visibleSignature.noCertificate")}</p>}
          <Switch
            // El interruptor se pinta **en «no»** dentro de un bloque apagado.
            // Encendido prometía un recuadro que no hay, y la preferencia que
            // guarda `signature.enabled` no se pierde: vuelve al reaparecer el
            // certificado, igual que la colocación.
            checked={usable && signature.enabled}
            label={t("panel.visibleSignature.toggle")}
            onChange={(enabled) => onChangeSignature({ ...signature, enabled })}
          />

          {usable && signature.enabled && (
            <>
              <PlacementFieldset
                documentPages={document.pages}
                pageSets={pageSets}
                pageChoice={pageChoice}
                onChangePageChoice={onChangePageChoice}
                pagesText={pagesText}
                onTypePages={typePages}
                rangeError={rangeError}
                echo={echo}
                sealedCount={sealedCount}
                sealButton={sealButton}
              />

              <SignatureFieldsFieldset
                signature={signature}
                onChangeSignature={onChangeSignature}
                rubric={rubric}
                rubricFailure={rubricFailure}
                onChooseRubric={onChooseRubric}
                onOpenHelp={onOpenHelp}
              />
            </>
          )}
        </section>
      </div>

      <PanelFooter
        failure={failure}
        destination={destination}
        documentName={document.name}
        onChangeDestination={onChangeDestination}
        unplaced={unplaced}
        usable={usable}
        signing={signing}
        blocked={blocked}
        onSign={onSign}
        onOpenHelp={onOpenHelp}
      />
    </div>
  );
}
