import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import "./StatusView.css";

interface StatusViewProps {
  onClose: () => void;
}

export function StatusView({ onClose }: StatusViewProps) {
  const { t } = useTranslation();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);

  return (
    <section className="status-view" aria-label={t("status.title")}>
      <div className="status-view__header">
        <h1 className="rf-title status-view__title">{t("status.title")}</h1>
      </div>
      <div className="status-view__body" />
      <div className="status-view__footer">
        <button
          type="button"
          className="rf-btn rf-btn--secondary status-view__close"
          onClick={onClose}
        >
          {t("actions.close")}
        </button>
      </div>
    </section>
  );
}
