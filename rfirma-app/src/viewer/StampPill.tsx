//! La pastilla flotante bajo la hoja con el estado del sello y, si hace falta, el botón que lo compone.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import type { StampPreview } from "../signing/stampPreview";

/**
 * La pastilla flotante del estado del sello: un texto y, si
 * hace falta, un botón. No hay insignia — se retiró con el rótulo «Vista
 * previa» del panel, junto con sus 16 claves — y no hay nada que decir de la
 * colocación: la etiqueta del botón de sellar, en el panel, ya cuenta eso.
 *
 * Sin certificado, sin colocar, mientras se arrastra y «al día» no montan la
 * pastilla: no hay sello del que hablar, o ya no hace falta decirlo
 * (docs/design/visor-de-documento.md § «La pastilla bajo la hoja»).
 *
 * **Mide lo mismo tenga botón o no.** El hueco del botón queda reservado
 * incluso vacío, para no saltar al pasar de «sin componer» a «componiendo» o a
 * «fallido».
 */
export function StampPill({ state, onCompose }: { state: StampPreview; onCompose: () => void }) {
  const { t } = useTranslation();
  const lineId = useId();

  // Las claves se escriben **enteras y a mano**: `i18next-cli` lee el código
  // para cazar la clave que no está en el catálogo y la del catálogo que ya no
  // usa nadie, y una clave compuesta con una plantilla es invisible
  // para las dos comprobaciones.
  const said = {
    noCertificate: null,
    unplaced: null,
    frozen: null,
    onDemand: { line: t("viewer.stamp.onDemand"), button: t("viewer.stamp.show") },
    composing: { line: t("viewer.stamp.composing"), button: null },
    composed: null,
    failed: { line: t("viewer.stamp.failed"), button: t("actions.retry") },
  }[state.kind];

  // noCertificate, unplaced, frozen y composed no dicen nada del sello: sin
  // bloque encendido, sin recuadro, en pleno arrastre y «al día» son los
  // cuatro casos sin pastilla.
  if (!said) return null;

  return (
    <div className="viewer__stamp" role="status">
      <p className="rf-body viewer__stamp-line" id={lineId}>
        {said.line}
      </p>
      <span className="viewer__stamp-slot">
        {said.button !== null && (
          <Button
            variant="secondary"
            className="viewer__stamp-button"
            aria-describedby={lineId}
            onClick={onCompose}
          >
            {said.button}
          </Button>
        )}
      </span>
    </div>
  );
}
