import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FileIcon, InfoIcon } from "../design-system/icons";
import { CertificateSelect } from "../signing/CertificateSelect";
import type { Certificate } from "../signing/certificate";
import { formatSize } from "../signing/SigningPanel";
import type { ErrandStage, SiteDocument, SiteOperation } from "./errand";
import { consentActionKey } from "./errand";
import { SedeBody } from "./SedeFrame";

interface SedeConsentProps {
  origin: string | null;
  operation: SiteOperation;
  stage: Extract<ErrandStage, { kind: "consent" }>;
  onConsent: (certificateId: string) => void;
  onCancel: () => void;
}

/**
 * **2 · Consentimiento.** El corazón del trámite: la pantalla que AutoFirma no
 * tiene.
 *
 * Es una **confirmación escrita**, no el selector de certificados (ID-269). El
 * selector no dice quién pide, ni qué se firma, ni que haya una sede detrás; lo
 * que se reutiliza de él es su **desplegable**, tal cual —mismo componente,
 * misma agrupación, mismo alto de lista—, dentro de la confirmación.
 *
 * Aparece **siempre**, también con un solo certificado: `headless` y
 * `mandatoryCertSelection` se ignoran los dos, porque encadenados —filtro que
 * deja uno, almacén que no pide PIN— la persona no vería absolutamente nada
 * (ID-272).
 */
export function SedeConsent({ origin, operation, stage, onConsent, onCancel }: SedeConsentProps) {
  const { t } = useTranslation();
  const [chosen, setChosen] = useState<Certificate | null>(
    stage.certificates.find((certificate) => certificate.remembered) ??
      stage.certificates[0] ??
      null,
  );
  // `selectcert` es identificarse y todo lo demás es firmar: una sola pregunta,
  // resuelta en el vocabulario del trámite y no repetida aquí.
  const identity = consentActionKey(operation) === "identify";

  return (
    <SedeBody
      footer={
        <>
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button
            type="button"
            className="rf-btn rf-btn--primary"
            disabled={chosen === null}
            onClick={() => chosen !== null && onConsent(chosen.id)}
          >
            {identity ? t("sede.consent.identify") : t("sede.consent.sign")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-consent">
        {origin === null ? (
          /* Sin origen válido queda una **etiqueta serena**, no una advertencia:
             el `Origin` es falsificable desde cualquier programa local y el
             original lo ignora por completo, así que no hay nada que denunciar
             — sólo un silencio que no se rellena con un invento (ID-271). */
          <div className="rf-row rf-gap-xs sede-consent__unknown-origin">
            <span className="sede-consent__icon">
              <InfoIcon size={18} />
            </span>
            <div className="rf-stack sede-consent__unknown-origin-text">
              <p className="rf-title">{t("sede.consent.unknownOriginTitle")}</p>
              <p className="rf-hint">
                {identity
                  ? t("sede.consent.unknownOriginIdentity")
                  : t("sede.consent.unknownOriginSignature")}
              </p>
            </div>
          </div>
        ) : (
          <p className="rf-title sede-consent__asks">
            {identity
              ? t("sede.consent.asksIdentity", { origin })
              : t("sede.consent.asksSignature", { origin })}
          </p>
        )}

        {stage.document !== null && <DocumentCard document={stage.document} />}

        {/* Situación 5 (ID-302, ID-304): información, no alarma — mismo icono
            y mismo borde de 1 px que el origen sin identificar. No hay un
            sexto momento (ID-298): se pregunta aquí, dentro del mismo
            consentimiento. */}
        {stage.document?.hasUnregisteredSignatures && (
          <div className="rf-row rf-gap-xs sede-consent__unrecognized-signatures">
            <span className="sede-consent__icon">
              <InfoIcon size={18} />
            </span>
            <p className="rf-hint">{t("sede.consent.unrecognizedSignatures")}</p>
          </div>
        )}

        <div className="rf-stack rf-gap-xs sede-consent__certificate">
          <p className="rf-label sede-consent__label">
            {identity ? t("sede.consent.identifyWith") : t("sede.consent.signWith")}
          </p>
          <CertificateSelect
            certificates={stage.certificates}
            chosen={chosen}
            onChoose={setChosen}
          />
        </div>

        {/* Debajo del desplegable y no encima: es una nota sobre lo que la lista
            contiene, y se lee después de verla. Dice **que** la sede acotó, y
            nunca qué descartó ni con qué criterio (ID-277). */}
        {stage.narrowed && (
          <p className="rf-prose sede-consent__narrowed">
            {origin === null
              ? t("sede.consent.narrowedUnknownOrigin")
              : t("sede.consent.narrowed", { origin })}
          </p>
        )}

        {identity && (
          <div className="sede-consent__sends">
            <p className="rf-prose">{t("sede.consent.willSend")}</p>
          </div>
        )}
      </div>
    </SedeBody>
  );
}

/**
 * Sólo lo que el PDF dice **de sí mismo** (ID-270): título de sus metadatos si
 * lo trae, páginas, tamaño y si ya viene firmado.
 *
 * No hay nombre de fichero ni ruta porque el protocolo no los trae, y un PDF
 * sin título se nombra como lo que es, en gris, sin inventarle uno.
 */
function DocumentCard({ document }: { document: SiteDocument }) {
  const { t, i18n } = useTranslation();
  const untitled = document.title === null || document.title.trim() === "";

  return (
    <div className="rf-stack sede-consent__document">
      <div className="rf-row rf-gap-xs sede-consent__document-head">
        <span className="sede-consent__icon">
          <FileIcon size={20} />
        </span>
        <div className="rf-stack sede-consent__document-text">
          <p className={`rf-title${untitled ? " sede-consent__untitled" : ""}`}>
            {untitled ? t("sede.consent.untitled") : document.title}
          </p>
          <p className="rf-body rf-text-muted">
            {[
              t("panel.document.pages", { count: document.pages }),
              formatSize(document.sizeBytes, i18n.language),
            ].join(" · ")}
          </p>
        </div>
      </div>
      {document.signatures > 0 && (
        <p className="rf-body sede-consent__cosignature">
          {t("panel.coSignature", { count: document.signatures })}
        </p>
      )}
    </div>
  );
}
