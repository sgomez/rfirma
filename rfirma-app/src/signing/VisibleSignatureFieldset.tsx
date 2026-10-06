//! La sección de la firma visible: el interruptor, la ayuda si falta certificado y la colocación en el documento.

import { useTranslation } from "react-i18next";
import { Switch } from "../design-system/Switch";
import { PlacementBlock, type PlacementBlockState } from "../placement/PlacementBlock";
import type { CertificateState } from "./certificate";
import type { VisibleSignatureSection } from "./visibleSignature";

interface VisibleSignatureFieldsetProps {
  signature: VisibleSignatureSection;
  certificate: CertificateState;
  placementState: PlacementBlockState;
  signing: boolean;
}

/** Si se estampa un recuadro y dónde (docs/design/panel-de-firma.md § Firma visible). */
export function VisibleSignatureFieldset({
  signature: { value: signature, change: onChangeSignature },
  certificate,
  placementState,
  signing,
}: VisibleSignatureFieldsetProps) {
  const { t } = useTranslation();
  const chosen = certificate.kind === "chosen";
  const visible = signature.enabled && chosen;

  return (
    <section className="panel__visible" aria-label={t("panel.visibleSignature.title")}>
      <div className={signing ? "panel__toggle panel__toggle--dim" : "panel__toggle"}>
        <Switch
          trailing
          checked={visible}
          disabled={!chosen}
          label={t("panel.visibleSignature.title")}
          title={visible ? t("panel.visibleSignature.turnOff") : t("panel.visibleSignature.turnOn")}
          onChange={(enabled) => onChangeSignature({ ...signature, enabled })}
        />
      </div>
      {(certificate.kind === "loading" || certificate.kind === "unchosen") && (
        <p className="rf-hint panel__visible-hint">
          {t("panel.visibleSignature.needsCertificate")}
        </p>
      )}

      {visible && (
        <div className={signing ? "panel__placement panel__controls--dim" : "panel__placement"}>
          <PlacementBlock state={placementState} />
        </div>
      )}
    </section>
  );
}
