//! La línea del lector de tarjetas bajo el selector de certificado: qué pasa con el lector, y nada si no hay ninguno.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Icon } from "../design-system/icons";
import { hasAReader, type ReaderStatus } from "./certificate";
import "./ReaderLine.css";

/** La plataforma de la que depende la ayuda para una tarjeta ilegible. */
export type CardHelpPlatform = "windows" | "linux" | "other";

/** La plataforma que corresponde al `userAgent` del WebView. */
function cardHelpPlatformFor(userAgent: string): CardHelpPlatform {
  if (/windows/i.test(userAgent)) return "windows";
  if (/linux/i.test(userAgent) && !/android/i.test(userAgent)) return "linux";
  return "other";
}

/** Lo que dice el lector, con el indicador que gira mientras se lee la tarjeta (docs/design/panel-de-firma.md § Certificado). */
export function ReaderLine({
  reader,
  platform = cardHelpPlatformFor(navigator.userAgent),
}: {
  reader: ReaderStatus;
  platform?: CardHelpPlatform;
}) {
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
      <span>
        {readerText(reader, t)}
        {reader.kind === "unreadable" && <CardHelp platform={platform} />}
      </span>
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

function CardHelp({ platform }: { platform: CardHelpPlatform }) {
  const { t } = useTranslation();
  if (platform === "other") return null;
  return (
    <span className="reader-line__help">
      {platform === "windows"
        ? t("panel.certificate.reader.unreadableHelp.windows")
        : t("panel.certificate.reader.unreadableHelp.linux")}
    </span>
  );
}
