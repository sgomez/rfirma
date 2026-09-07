import { useTranslation } from "react-i18next";
import { SedeBody } from "./SedeFrame";

interface SedeTransferProps {
  /** Guardar el fichero que propone la sede, o cargar el que elija la persona. */
  transfer: { kind: "saving"; filename: string | null } | { kind: "loading"; multiple: boolean };
}

/**
 * **El fichero que va o que viene**, mientras el diálogo del portal está
 * abierto encima.
 *
 * Del guardado se nombra **el fichero y nunca su carpeta** (ADR-0011): la ruta
 * la elige el diálogo y no vuelve a cruzar. De la carga se dice si es uno o
 * varios, que es lo único que la sede declara.
 *
 * **Cero acciones**, como el tramo de devolver de `SedeSigning`: quien
 * confirma o descarta es la persona dentro del diálogo, y un `Cancelar` aquí
 * sería un segundo mando sobre lo mismo.
 */
export function SedeTransfer({ transfer }: SedeTransferProps) {
  const { t } = useTranslation();

  return (
    <SedeBody steadyFooter footer={null}>
      <div className="rf-stack sede-transfer">
        <p className="rf-title sede-transfer__title">{titleOf(transfer, t)}</p>
        <p className="rf-prose rf-text-muted">{leadOf(transfer, t)}</p>
      </div>
    </SedeBody>
  );
}

type Translate = ReturnType<typeof useTranslation>["t"];

function titleOf(transfer: SedeTransferProps["transfer"], t: Translate) {
  if (transfer.kind === "loading") {
    return transfer.multiple ? t("sede.loading.titleMany") : t("sede.loading.title");
  }
  return transfer.filename === null
    ? t("sede.saving.titleUnnamed")
    : t("sede.saving.title", { filename: transfer.filename });
}

function leadOf(transfer: SedeTransferProps["transfer"], t: Translate) {
  if (transfer.kind === "saving") return t("sede.saving.lead");
  return transfer.multiple ? t("sede.loading.leadMany") : t("sede.loading.lead");
}
