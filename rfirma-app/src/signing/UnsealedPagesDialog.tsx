//! El diálogo de páginas sin sello, justo antes de firmar: cuántas del conjunto elegido se quedan sin firma visible.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import "./UnsealedPagesDialog.css";

interface UnsealedPagesDialogProps {
  /** Cuántas páginas del conjunto elegido se quedan sin sello. */
  fallen: number;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * El diálogo de páginas sin sello (docs/design/dialogo-paginas-sin-firma-visible.md).
 *
 * Aparece **justo antes de firmar**, y solo cuando `correctPositionSignature`
 * se va a comer alguna página en silencio: es el único aviso que hay, porque
 * no queda marca por página en el visor.
 *
 * Dos cosas que el texto no puede equivocarse:
 *
 * - **«Sin firma visible», nunca «recortadas»**: la firma criptográfica cubre
 *   el documento entero pase lo que pase; lo que falta en esas páginas es la
 *   marca visible, no un trozo de la firma.
 *
 * Las páginas que se caen no se nombran una a una: con doce, una
 * lista de números es una pared que no ayuda a decidir. Solo el recuento.
 */
export function UnsealedPagesDialog({ fallen, onConfirm, onCancel }: UnsealedPagesDialogProps) {
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
            {t("sealLoss.title", { count: fallen })}
          </p>
        </div>

        <p className="rf-prose">{t("sealLoss.body")}</p>

        <hr className="rf-divider" />

        <div className="rf-row unsealed-pages-dialog__actions">
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onConfirm}>
            {t("actions.signAnyway")}
          </button>
        </div>
      </div>
    </div>
  );
}
