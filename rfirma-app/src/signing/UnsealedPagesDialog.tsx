//! El diálogo de páginas sin sello, justo antes de firmar: cuántas del conjunto elegido se quedan sin firma visible.

import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { Icon } from "../design-system/icons";
import { Row } from "../design-system/Row";
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
  const signAnyway = useRef<HTMLButtonElement>(null);

  return (
    <Dialog
      label={t("sealLoss.title", { count: fallen })}
      onClose={onCancel}
      primary={signAnyway}
      className="unsealed-pages-dialog"
    >
      <div className="unsealed-pages-dialog__heading">
        <span className="unsealed-pages-dialog__alert" aria-hidden="true">
          <Icon name="warning" size={24} />
        </span>
        <p className="rf-title">{t("sealLoss.title", { count: fallen })}</p>
      </div>

      <p className="rf-prose">{t("sealLoss.body")}</p>

      <hr className="rf-divider" />

      <Row className="unsealed-pages-dialog__actions">
        <Button variant="ghost" onClick={onCancel}>
          {t("actions.cancel")}
        </Button>
        <Button variant="primary" ref={signAnyway} onClick={onConfirm}>
          {t("actions.signAnyway")}
        </Button>
      </Row>
    </Dialog>
  );
}
