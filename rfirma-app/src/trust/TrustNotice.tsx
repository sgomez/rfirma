import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./TrustNotice.css";

/**
 * El aviso del primer arranque: la CA local y el permiso de red local,
 * explicados **juntos** y **antes** de que el navegador pregunte (ID-230,
 * ID-231).
 *
 * Se monta sin condición ninguna, en `main.tsx`, junto a `App`: no reacciona a
 * ningún fallo, porque para entonces ya sería tarde —el permiso lo pide el
 * navegador en nombre de la sede, no de rFirma, y ni siquiera nombra a rFirma
 * en su aviso—.
 *
 * **No promete diagnosticar una denegación** (ID-230): el fallo, si lo hay,
 * ocurre entero dentro del navegador, y eso se dice tal cual, no se calla.
 *
 * Se descarta con `Entendido` y no vuelve a aparecer en la sesión. Persistir
 * el descarte entre arranques exigiría una orden de Tauri que hoy no existe
 * —`app::trust::refresh_local_ca_trust` no está enganchado a ningún comando
 * (#344)—, y añadirla es trabajo de otra ficha: este aviso es sobre el texto,
 * no sobre esa tubería.
 */
export function TrustNotice() {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(false);
  const titleId = useId();

  if (dismissed) return null;

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog trust-notice"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <p className="rf-title" id={titleId}>
          {t("trust.notice.title")}
        </p>
        <p className="rf-prose">{t("trust.notice.localCa")}</p>
        <p className="rf-prose">{t("trust.notice.localNetwork")}</p>
        <p className="rf-prose rf-text-muted">{t("trust.notice.cannotTellIfDenied")}</p>
        <hr className="rf-divider" />
        <div className="rf-row trust-notice__actions">
          <button
            type="button"
            className="rf-btn rf-btn--primary"
            onClick={() => setDismissed(true)}
          >
            {t("trust.notice.acknowledge")}
          </button>
        </div>
      </div>
    </div>
  );
}
