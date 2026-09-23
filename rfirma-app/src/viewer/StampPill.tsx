import { useTranslation } from "react-i18next";
import type { StampPreview } from "../signing/stampPreview";

/**
 * La pastilla flotante del estado del sello (ID-107, #202): un texto y, si
 * hace falta, un botón. No hay insignia — se retiró con el rótulo «Vista
 * previa» del panel, junto con sus 16 claves — y no hay nada que decir de la
 * colocación: la etiqueta del botón de sellar, en el panel, ya cuenta eso.
 *
 * Sin certificado, sin colocar y «al día» no montan la pastilla: no hay sello
 * del que hablar, o ya no hace falta decirlo (docs/design/visor-de-documento.md
 * § «La pastilla bajo la hoja»).
 *
 * **Mide lo mismo tenga botón o no.** El hueco del botón queda reservado
 * incluso vacío, para no saltar al pasar de «congelado» a «sin componer»
 * mientras se arrastra el recuadro.
 */
export function StampPill({ state, onCompose }: { state: StampPreview; onCompose: () => void }) {
  const { t } = useTranslation();

  // Las claves se escriben **enteras y a mano**: `i18next-cli` lee el código
  // para cazar la clave que no está en el catálogo y la del catálogo que ya no
  // usa nadie (ID-127), y una clave compuesta con una plantilla es invisible
  // para las dos comprobaciones.
  const said = {
    noCertificate: null,
    unplaced: null,
    frozen: { line: t("viewer.stamp.frozen"), button: null },
    onDemand: { line: t("viewer.stamp.onDemand"), button: t("viewer.stamp.show") },
    composing: { line: t("viewer.stamp.composing"), button: null },
    composed: null,
    failed: { line: t("viewer.stamp.failed"), button: t("viewer.stamp.retry") },
  }[state.kind];

  // noCertificate, unplaced y composed no dicen nada del sello: sin bloque
  // encendido, sin recuadro y «al día» son los tres casos sin pastilla.
  if (!said) return null;

  return (
    <div className="viewer__stamp" role="status">
      <p className="rf-body viewer__stamp-line">{said.line}</p>
      <span className="viewer__stamp-slot">
        {said.button !== null && (
          <button
            type="button"
            className="rf-btn rf-btn--secondary viewer__stamp-button"
            onClick={onCompose}
          >
            {said.button}
          </button>
        )}
      </span>
    </div>
  );
}
