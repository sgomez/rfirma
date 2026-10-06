//! La columna derecha antes de firmar: la zona que se desliza con todo lo que se decide y el pie fijo con el botón de firmar.

import { useTranslation } from "react-i18next";
import { Switch } from "../design-system/Switch";
import { ErrorNotice } from "../errors/ErrorNotice";
import { PlacementBlock, type PlacementBlockState } from "../placement/PlacementBlock";
import { CertificateNotice } from "./CertificateNotice";
import { CertificateSelect } from "./CertificateSelect";
import type { CertificateSection } from "./certificate";
import type { Destination } from "./destination";
import type { SigningFailure } from "./failure";
import { ModelFieldset } from "./ModelFieldset";
import { PanelFooter } from "./PanelFooter";
import { PreviousSignaturesNotice } from "./PreviousSignaturesNotice";
import type { PreviousSignaturesReport } from "./previousSignatures";
import type { RubricSection } from "./rubric";
import "./SigningPanel.css";
import type { VisibleSignature } from "./visibleSignature";

/** El documento que se va a firmar, con lo que el panel enseña de él. */
interface SigningDocument {
  /** Identifica el documento entre pestañas, para que un aviso no herede el estado del anterior. */
  id: string;
  name: string;
  /** El tamaño, o `null` mientras nadie lo sepa: no se inventa un cero. */
  sizeBytes: number | null;
}

interface SigningPanelProps {
  document: SigningDocument;
  /** El informe de firmas previas del documento, pedido al abrir o cargar. */
  previousSignatures: PreviousSignaturesReport;
  certificate: CertificateSection;
  signature: VisibleSignature;
  onChangeSignature: (signature: VisibleSignature) => void;
  /** La colocación de la firma visible, tal y como la entrega su estado. */
  placementState: PlacementBlockState;
  rubric: RubricSection;
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
  certificate: certificateSection,
  signature,
  onChangeSignature,
  placementState,
  rubric,
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
  const certificate = certificateSection.state;
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;

  const visible = signature.enabled && chosen !== null;
  const blocked = visible && placementState.rangeError !== null;

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
                  onChoose={certificateSection.choose}
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
                  <PlacementBlock state={placementState} />
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
                  rubric={rubric.value}
                  rubricFailure={rubric.failure}
                  onChooseRubric={rubric.choose}
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
        onRetryCertificates={() => void certificateSection.lookAgain()}
        onChooseModule={() => void certificateSection.lookAgain()}
        onSign={onSign}
        onBack={onBack}
      />
    </div>
  );
}
