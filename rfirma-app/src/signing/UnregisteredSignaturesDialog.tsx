//! La pregunta, justo antes de firmar, por las firmas previas cuyo `/SubFilter` rFirma no sabe leer.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";

interface UnregisteredSignaturesDialogProps {
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * El aviso de las **firmas sin registrar**.
 *
 * Aparece **justo antes de firmar**, cuando el documento trae alguna firma
 * previa cuyo `/SubFilter` no es de los cuatro que el puente sabe leer. Tres
 * cosas que el texto no puede equivocarse:
 *
 * - **No es un rechazo, es una pregunta.** El PDF certificado sí invalida con
 *   certeza y por eso se rechaza sin preguntar; esto es desconocimiento
 *   nuestro, y negarse dejaría a rFirma rechazando documentos que AutoFirma sí
 *   firma.
 * - **No se dice cuántas hay ni de quién son, y no se dice si valen**: rFirma no tiene validador, y
 * enseñar «válida» sin poder
 *   sostenerlo es peor que el silencio. Se avisa de lo que no entendemos y se
 *   calla lo que entendemos.
 * - **No lleva recuento**, así que no hay plural que resolver: un aviso, una
 *   frase.
 *
 * Decir que no **no es un fallo**: devuelve al panel con todo como estaba, y en
 * el recorrido de una sede es lo que sale al cable como `CANCEL`.
 */
export function UnregisteredSignaturesDialog({
  onConfirm,
  onCancel,
}: UnregisteredSignaturesDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog unsealed-pages-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="unsealed-pages-dialog__heading">
          <span className="unsealed-pages-dialog__alert" aria-hidden="true">
            <AlertIcon size={24} />
          </span>
          <p className="rf-title" id={titleId}>
            {t("unregisteredSignatures.title")}
          </p>
        </div>

        <p className="rf-prose">{t("unregisteredSignatures.body")}</p>

        <hr className="rf-divider" />

        <div className="rf-row unsealed-pages-dialog__actions">
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onConfirm}>
            {t("unregisteredSignatures.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
