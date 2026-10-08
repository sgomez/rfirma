//! El indicador de la cabecera que dice si hay un lector de tarjetas: detectado, no detectado o no soportado; nada de la tarjeta.

import { useTranslation } from "react-i18next";
import { Icon } from "../design-system/icons";
import type { ReaderStatus } from "../signing/certificate";
import "./ReaderIndicator.css";

/** Si hay lector, sin mirar la tarjeta que tenga dentro. */
export function ReaderIndicator({ reader }: { reader: ReaderStatus }) {
  const { t } = useTranslation();

  const { text, hint, className } = (() => {
    switch (reader.kind) {
      case "noReader":
        return {
          text: t("header.reader.missing"),
          hint: t("header.reader.missingHint"),
          className: "rf-caption rf-text-muted reader-indicator",
        };
      case "unavailable":
        return {
          text: t("header.reader.unsupported"),
          hint: t("header.reader.unsupportedHint"),
          className: "rf-caption rf-text-muted reader-indicator reader-indicator--unsupported",
        };
      default:
        return {
          text: t("header.reader.detected"),
          hint: undefined,
          className: "rf-caption reader-indicator",
        };
    }
  })();

  return (
    <span className={className} role="note" aria-label={text} title={hint}>
      <Icon name="smartCard" size={14} />
      {text}
    </span>
  );
}
