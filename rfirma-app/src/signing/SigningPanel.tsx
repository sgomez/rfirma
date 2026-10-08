//! La columna derecha antes de firmar: la zona que se desliza con todo lo que se decide y el pie fijo con el botón de firmar.

import { ErrorNotice } from "../errors/ErrorNotice";
import type { PlacementBlockState } from "../placement/PlacementBlock";
import { CertificateExits } from "./CertificateExits";
import { CertificateSelect } from "./CertificateSelect";
import type { CertificateSection } from "./certificate";
import type { DestinationSection } from "./destination";
import type { SigningFailure } from "./failure";
import type { SigningSection } from "./flow";
import { ModelFieldset } from "./ModelFieldset";
import { PanelFooter } from "./PanelFooter";
import { PreviousSignaturesNotice } from "./PreviousSignaturesNotice";
import type { PreviousSignaturesReport } from "./previousSignatures";
import type { RubricSection } from "./rubric";
import "./SigningPanel.css";
import { VisibleSignatureFieldset } from "./VisibleSignatureFieldset";
import type { VisibleSignatureSection } from "./visibleSignature";

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
  signature: VisibleSignatureSection;
  /** La colocación de la firma visible, tal y como la entrega su estado. */
  placementState: PlacementBlockState;
  rubric: RubricSection;
  destination: DestinationSection;
  signing: SigningSection;
  failure: SigningFailure | null;
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
  signature: signatureSection,
  placementState,
  rubric,
  destination,
  signing: signingSection,
  failure,
  onOpenHelp,
  onEmptyStore,
}: SigningPanelProps) {
  const certificate = certificateSection.state;
  const { value: signature, change: onChangeSignature } = signatureSection;
  const signing = signingSection.running;
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
            <div className={signing ? "panel__controls--dim" : undefined}>
              <CertificateSelect
                certificates={
                  certificate.kind === "unchosen" || certificate.kind === "chosen"
                    ? certificate.certificates
                    : []
                }
                chosen={chosen}
                onChoose={certificateSection.choose}
                searching={certificate.kind === "loading"}
                absent={
                  certificate.kind === "empty" || certificate.kind === "failed"
                    ? certificate.kind
                    : undefined
                }
                disabled={signing}
              />
            </div>
            <CertificateExits
              state={certificate}
              installFailure={certificateSection.installFailure}
              onInstall={() => void certificateSection.install()}
              onLookAgain={() => void certificateSection.lookAgain()}
              onOpenHelp={onOpenHelp}
            />

            {previousSignatures.signatures.length > 0 && (
              <PreviousSignaturesNotice
                key={document.id}
                report={previousSignatures}
                certificate={chosen}
              />
            )}

            <VisibleSignatureFieldset
              signature={signatureSection}
              certificate={certificate}
              placementState={placementState}
              signing={signing}
            />

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
        destination={destination.value}
        documentName={document.name}
        onChangeDestination={() => void destination.chooseSingle()}
        signing={signing}
        blocked={blocked}
        closed={previousSignatures.closed === true}
        certificate={certificate}
        onSign={() => void signingSection.sign()}
        onBack={signingSection.back}
      />
    </div>
  );
}
