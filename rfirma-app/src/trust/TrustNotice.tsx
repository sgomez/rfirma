import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./TrustNotice.css";

interface TrustNoticeProps {
  /**
   * Si ya se descartó en un arranque anterior (`Preferences.trustNoticeSeen`).
   * `true` significa que no se monta nada: el aviso es del **primer**
   * arranque, no de todos (#365).
   */
  seen: boolean;
  /**
   * Persiste el descarte para que no vuelva a aparecer. Se llama una vez, al
   * pulsar «Entendido», y quien nos monta es responsable de escribirlo en
   * `PreferencesStore` (`main.tsx`).
   */
  onAcknowledge: () => void;
}

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
 * Se descarta con `Entendido` y **no vuelve a aparecer en ningún arranque
 * posterior**: `seen` decide si se monta, y `onAcknowledge` persiste el
 * descarte fuera de este componente.
 */
export function TrustNotice({ seen, onAcknowledge }: TrustNoticeProps) {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(false);
  const titleId = useId();
  const descriptionId = useId();
  const button = useRef<HTMLButtonElement>(null);

  // El foco entra en el botón al montarse, igual que `PinDialog`: es el único
  // control del diálogo, y quien navegue por teclado no debería tener que
  // buscarlo.
  useEffect(() => {
    if (!seen) button.current?.focus();
  }, [seen]);

  if (seen || dismissed) return null;

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog trust-notice"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
      >
        <p className="rf-title" id={titleId}>
          {t("trust.notice.title")}
        </p>
        <div id={descriptionId}>
          <p className="rf-prose">{t("trust.notice.localCa")}</p>
          <p className="rf-prose">{t("trust.notice.localNetwork")}</p>
          <p className="rf-prose rf-text-muted">{t("trust.notice.cannotTellIfDenied")}</p>
        </div>
        <hr className="rf-divider" />
        <div className="rf-row trust-notice__actions">
          <button
            ref={button}
            type="button"
            className="rf-btn rf-btn--primary"
            onClick={() => {
              setDismissed(true);
              onAcknowledge();
            }}
          >
            {t("trust.notice.acknowledge")}
          </button>
        </div>
      </div>
    </div>
  );
}
