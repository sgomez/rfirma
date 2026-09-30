import { useTranslation } from "react-i18next";
import { NotificationStrip } from "../shell/NotificationStrip";
import type { NewVersion } from "./newVersion";

interface NewVersionStripProps {
  /** La versión que anunciar, o nada si no hay ninguna o no toca enseñarla. */
  newVersion: NewVersion | null;
  /** Abre el diálogo al que lleva la acción de la franja. */
  onOpen: (dialog: "installUpdate" | "about") => void;
  onDismiss: () => void;
}

/**
 * El único inquilino de la franja (ID-354). Si la versión se puede instalar
 * desde aquí, la acción es «Actualizar ahora»; si no, no descarga nada: lleva
 * a *Acerca de*, que es donde están las órdenes de alta del repositorio
 * (ID-181), y así el `opener:deny-open-url` del ID-85 sigue sin hacer falta.
 */
export function NewVersionStrip({ newVersion, onOpen, onDismiss }: NewVersionStripProps) {
  const { t } = useTranslation();
  if (newVersion === null) return null;
  return (
    <NotificationStrip
      message={t("notifications.newVersion.message", { version: newVersion.version })}
      action={
        newVersion.installable
          ? { label: t("updates.install.action"), onSelect: () => onOpen("installUpdate") }
          : { label: t("notifications.newVersion.action"), onSelect: () => onOpen("about") }
      }
      dismissLabel={t("actions.dismiss")}
      onDismiss={onDismiss}
    />
  );
}
