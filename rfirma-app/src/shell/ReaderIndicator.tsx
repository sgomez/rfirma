//! El indicador de la cabecera que dice si hay un lector de tarjetas.

import { useTranslation } from "react-i18next";
import { Icon } from "../design-system/icons";
import type { ReaderStatus } from "../signing/certificate";
import "./ReaderIndicator.css";

type Presence = "detected" | "missing" | "unsupported";

const PRESENCE: Record<ReaderStatus["kind"], Presence> = {
  unavailable: "unsupported",
  noReader: "missing",
  noCard: "detected",
  reading: "detected",
  dnieReady: "detected",
  cardReady: "detected",
  unreadable: "detected",
};

/** Si hay lector, sin mirar la tarjeta que tenga dentro. */
export function ReaderIndicator({ reader }: { reader: ReaderStatus }) {
  const { t } = useTranslation();

  const looks = {
    detected: {
      text: t("header.reader.detected"),
      hint: undefined,
      className: "rf-caption reader-indicator",
    },
    missing: {
      text: t("header.reader.missing"),
      hint: t("header.reader.missingHint"),
      className: "rf-caption rf-text-muted reader-indicator",
    },
    unsupported: {
      text: t("header.reader.unsupported"),
      hint: t("header.reader.unsupportedHint"),
      className: "rf-caption rf-text-muted reader-indicator reader-indicator--unsupported",
    },
  }[PRESENCE[reader.kind]];

  return (
    <span className={looks.className} role="note" aria-label={looks.text} title={looks.hint}>
      <Icon name="smartCard" size={14} />
      {looks.text}
    </span>
  );
}
