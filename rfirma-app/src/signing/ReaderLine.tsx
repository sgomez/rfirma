//! La línea del lector de tarjetas bajo el selector de certificado: qué pasa con el lector, y nada si no hay ninguno.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Icon } from "../design-system/icons";
import { hasAReader, type ReaderStatus } from "./certificate";
import "./ReaderLine.css";

/** Lo que dice el lector, con el indicador que gira mientras se lee la tarjeta (docs/design/panel-de-firma.md § Certificado). */
export function ReaderLine({ reader }: { reader: ReaderStatus }) {
  const { t } = useTranslation();

  if (!hasAReader(reader)) return null;

  const reading = reader.kind === "reading";
  return (
    <span className="rf-caption rf-text-muted reader-line" role="status">
      {reading ? (
        <span className="reader-line__spinner">
          <Icon name="loading" size={14} />
        </span>
      ) : (
        <Icon name="smartCard" size={14} />
      )}
      {readerText(reader, t)}
    </span>
  );
}

function readerText(
  reader: Exclude<ReaderStatus, { kind: "noReader" | "unavailable" }>,
  t: TFunction,
): string {
  switch (reader.kind) {
    case "noCard":
      return t("panel.certificate.reader.noCard");
    case "reading":
      return t("panel.certificate.reader.reading");
    case "dnieReady":
      return t("panel.certificate.reader.dnieReady");
    case "cardReady":
      return t("panel.certificate.reader.cardReady");
    case "unreadable":
      return t("panel.certificate.reader.unreadable");
  }
}
