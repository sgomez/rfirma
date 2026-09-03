import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, SealIcon } from "../design-system/icons";
import "./UnsealedPagesDialog.css";

interface UnsealedPagesDialogProps {
  /** Cuántas páginas del conjunto elegido se quedan sin sello. */
  fallen: number;
  /** Cuántas páginas eligió la persona, **no** las que tiene el documento. */
  chosen: number;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * El diálogo de páginas sin sello (docs/design/dialogo-paginas-sin-sello.md,
 * ID-105, ID-106).
 *
 * Aparece **justo antes de firmar**, y solo cuando `correctPositionSignature`
 * se va a comer alguna página en silencio: es el único aviso que hay, porque
 * no queda marca por página en el visor (#152).
 *
 * Dos cosas que el texto no puede equivocarse:
 *
 * - **«Sin sello», nunca «recortadas»**: la firma criptográfica cubre el
 *   documento entero pase lo que pase; lo que falta en esas páginas es la
 *   marca visible, no un trozo de la firma.
 * - **El denominador es el conjunto elegido, no el documento** (ID-106): con
 *   27 páginas, 13 elegidas y 3 que se caen, dice «3 de las 13», nunca
 *   «3 de las 27».
 *
 * Las páginas que se caen no se nombran una a una (ID-106): con doce, una
 * lista de números es una pared que no ayuda a decidir. Solo el recuento.
 */
export function UnsealedPagesDialog({
  fallen,
  chosen,
  onConfirm,
  onCancel,
}: UnsealedPagesDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const sealed = chosen - fallen;

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

        <p className="rf-prose">{t("sealLoss.body", { fallen, chosen })}</p>

        <div className="unsealed-pages-dialog__remaining">
          <span className="unsealed-pages-dialog__seal" aria-hidden="true">
            <SealIcon size={20} />
          </span>
          <p className="rf-prose">{t("sealLoss.remaining", { sealed, chosen })}</p>
        </div>

        <hr className="rf-divider" />

        <div className="rf-row unsealed-pages-dialog__actions">
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onConfirm}>
            {t("sealLoss.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
